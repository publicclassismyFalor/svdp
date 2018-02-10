use ::serde_json;
use serde_json::Value;

use std::process::Command;
use std::collections::HashMap;

use std::thread;
use std::sync::{mpsc, Arc, Mutex};
//use std::time::Duration;

use std::io::Error;

struct Disk {
    device: String,  /* device name: /dev/vda */

    total: u64,  /* M */
    spent: u64,
    rd: u32,  /* kbytes */
    wr: u32,
    rdio: u32,  /* tps */
    wrio: u32,
}

//struct NetIf {
//    device: String,  /* device name: eth0 */
//
//    rd: u32,  /* kbytes */
//    wr: u32,
//    rdio: u32,  /* tps */
//    wrio: u32,
//}

struct Data {
    cpu_rate: u16,
    mem_rate: u16,
    load: [u16;2],  /* load_5m/load_15m */
    tcp_conn: u32,

    disk: HashMap<String, Disk>,  /* K: device */
    //netif: HashMap<String, NetIf>,
}

/* key: instance_id */
struct Ecs {
    data: HashMap<i32, Data>,  /* K: time_stamp, V: Data */

    disk: HashMap<String, String>,  /* K: device, V: device_id */
    netif: HashMap<String, String>,
}

impl Ecs {
    fn new() -> Ecs {
        Ecs {
            data: HashMap::new(),
            disk: HashMap::new(),
            netif: HashMap::new(),
        }
    }
}

enum DT {
    Base,
    Disk,
    //NetIf,
}

trait Sv {
    fn argv_new(&self, region: String) -> Vec<String>;
    fn meta_insert(&self, holder: &mut HashMap<String, Ecs>, data: Vec<u8>);
    //fn data_insert(&self, holder: Arc<Mutex<HashMap<String, Ecs>>>, data: Vec<u8>);
    fn reflect(&self) -> DT;
}

struct SvBase();
struct SvDisk();
//struct SvNetIf();

impl Sv for SvBase {
    fn argv_new(&self, region: String) -> Vec<String> {
        vec![
            "-region".to_owned(),
            region,
            "-domain".to_owned(),
            "ecs.aliyuncs.com".to_owned(),
            "-apiName".to_owned(),
            "DescribeInstances".to_owned(),
            "-apiVersion".to_owned(),
            "2014-05-26".to_owned(),
            "Action".to_owned(),
            "DescribeInstances".to_owned(),
            "PageSize".to_owned(),
            "100".to_owned(),
        ]
    }

    fn meta_insert(&self, holder: &mut HashMap<String, Ecs>, data: Vec<u8>) {
        let v: Value = ::serde_json::from_slice(&data).unwrap_or(Value::Null);
        if Value::Null == v {
            return;
        }

        let body = &v["Instances"]["Instance"];
        for i in 0.. {
            if Value::Null == body[i] {
                break;
            } else {
                if let Value::String(ref id) = body[i]["InstanceId"] {
                    holder.insert((*id).clone(), Ecs::new());
                }
            }
        }
    }

    fn reflect(&self) -> DT {
        DT::Base
    }
}

impl Sv for SvDisk {
    fn argv_new(&self, region: String) -> Vec<String> {
        vec![
            "-region".to_owned(),
            region,
            "-domain".to_owned(),
            "ecs.aliyuncs.com".to_owned(),
            "-apiName".to_owned(),
            "DescribeDisks".to_owned(),
            "-apiVersion".to_owned(),
            "2014-05-26".to_owned(),
            "Action".to_owned(),
            "DescribeDisks".to_owned(),
            "PageSize".to_owned(),
            "100".to_owned(),
        ]
    }

    fn meta_insert(&self, holder: &mut HashMap<String, Ecs>, data: Vec<u8>) {
        let v: Value = serde_json::from_slice(&data).unwrap_or(Value::Null);
        if Value::Null == v {
            return;
        }

        let body = &v["Disks"]["Disk"];
        let mut diskid;
        let mut device;
        for i in 0.. {
            if Value::Null == body[i] {
                break;
            } else {
                if let Value::String(ref ecsid) = body[i]["InstanceId"] {
                    if let Some(ecs) = holder.get_mut(ecsid) {
                        if let Value::String(ref id) = body[i]["DiskId"] {
                            diskid= id;
                        } else {
                            continue;
                        }

                        if let Value::String(ref dev) = body[i]["Device"] {
                            device = dev;
                        } else {
                            continue;
                        }

                        ecs.disk.insert((*device).clone(), (*diskid).clone());
                    }
                }
            }
        }
    }

    fn reflect(&self) -> DT {
        DT::Disk
    }
}

fn cmd_exec(mut extra: Vec<String>) -> Result<Vec<u8>, Error> {
    let mut argv: Vec<String> = Vec::new();

    for x in ::ARGV.iter() {
        argv.push((**x).to_string());
    }

    argv.append(&mut extra);

    let output = Command::new(::CMD).args(argv).output() ?;

    if output.status.success() {
        return Ok(output.stdout);
    } else {
        return Err(Error::from_raw_os_error(output.status.code().unwrap_or(1)));
    }
}

fn get_region() -> Option<Vec<String>> {
    let mut res: Vec<String> = Vec::new();
    let extra = vec![
        "-domain".to_owned(),
        "ecs.aliyuncs.com".to_owned(),
        "-apiName".to_owned(),
        "DescribeRegions".to_owned(),
        "-apiVersion".to_owned(),
        "2014-05-26".to_owned(),
        "Action".to_owned(),
        "DescribeRegions".to_owned(),
    ];

    if let Ok(stdout) = cmd_exec(extra) {
        let v: Value = serde_json::from_slice(&stdout).unwrap_or(Value::Null);
        if Value::Null == v {
            return None;
        }

        for x in 0.. {
            if Value::Null == v["Regions"]["Region"][x] {
                break;
            } else {
                /* map 方式解析出来的 json string 是带引号的，需要处理掉 */
                if let Value::String(ref s) = v["Regions"]["Region"][x]["RegionId"] {
                    res.push(s.to_string());
                } else {
                    return None;
                }
            }
        }
    } else {
        return None;
    }

    Some(res)
}

/*
 * return HashMap(contains meta info of all ecs+disk+netif)
 * @param start_time: unix time_stamp
 */
fn get_meta <T: Sv> (mut holder: &mut HashMap<String, Ecs>, region: String, dt/*data type*/: T) {
    let mut extra = dt.argv_new(region.clone());

    if let Ok(ret) = cmd_exec(extra.clone()) {
        let v: Value = serde_json::from_slice(&ret).unwrap_or(Value::Null);
        if Value::Null == v {
            return;
        }

        let mut pages;
        if let Value::Number(ref total) = v["TotalCount"] {
            pages = total.as_u64().unwrap_or(0);
            if 0 == pages {
                return;
            } else if 0 == pages % 100 {
                pages = pages / 100;
            } else {
                pages = 1 + pages / 100;
            }
        } else {
            return;
        }

        dt.meta_insert(&mut holder, ret);

        if 1 < pages {
            extra.push("PageNumber".to_owned());

            let worker = |tx: mpsc::Sender<Vec<u8>>, page: u64, mut extra_: Vec<String>| {
                thread::spawn(move || {
                    extra_.push(page.to_string());
                    if let Ok(ret) = cmd_exec(extra_) {
                        tx.send(ret).unwrap_or_else(|e| eprintln!("{}", e));
                    }
                });
            };

            let (tx, rx) = mpsc::channel();

            for i in 3..(pages + 1) {
                worker(mpsc::Sender::clone(&tx), i, extra.clone());
            }

            /* consume the origin tx and extra */
            worker(tx, 2, extra);

            for i in rx {
                dt.meta_insert(&mut holder, i);
            }
        }

        match dt.reflect() {
            DT::Base => {
                get_meta(&mut holder, region.clone(), SvDisk());
                //get_meta(&mut holder, region, SvNetIf());
            },
            _ => {}
        }
    }
}

/********************
 * Public InterFace *
 ********************/
pub fn sv() {
    if let Some(regions) = get_region() {
        let mut tids = vec![];
        for region in regions.into_iter() {
            tids.push(
                thread::spawn(move || get_meta(&mut HashMap::new(), region, SvBase()))
                );
        }

        for tid in tids {
            tid.join().unwrap();
        }
    }
}
