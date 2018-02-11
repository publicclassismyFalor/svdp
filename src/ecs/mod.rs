mod cpu;
mod mem;
mod load5m;
mod load15m;
mod tcp;

mod disk;
mod netif;

use ::std;
use ::serde_json;
use serde_json::Value;

use std::process::Command;
use std::collections::HashMap;

use std::thread;
use std::sync::{mpsc, Arc, RwLock};

use std::io::Error;

enum DT {
    Ecs,
    Disk,
    NetIf,
}

/* key: instance_id */
struct Ecs {
    data: HashMap<i32, Inner>,  /* K: time_stamp, V: Supervisor Data */

    disk: HashMap<String, String>,  /* K: Device, V: DiskId */
}

struct Inner {
    cpu_rate: i16,
    mem_rate: i16,
    load5m: u16,
    load15m: u16,
    tcp: u32,  /* tcp conn cnt */

    disk: HashMap<String, disk::Disk>,  /* K: device */
    netif: HashMap<String, netif::NetIf>,  /* K: IP */
}

struct Meta();

trait META {
    fn argv_new(&self, region: String) -> Vec<String>;
    fn insert(&self, holder: &mut HashMap<String, Ecs>, data: Vec<u8>);
    fn reflect(&self) -> DT;
}

trait DATA {
    fn get(&self, holder: Arc<RwLock<HashMap<String, Ecs>>>, region: String) {
        let mut extra = self.argv_new(region);

        let (tx, rx) = mpsc::channel();

        thread::spawn(move || {
            let mut v: Value = Value::Null;;
            if let Ok(ret) = cmd_exec(extra.clone()) {
                v = serde_json::from_slice(&ret).unwrap_or(Value::Null);
                if Value::Null == v {
                    return;
                }

                tx.send(ret).unwrap();
            }

            extra.push("Cursor".to_owned());

            let mut v1: Value = Value::Null;;
            loop {
                if let Value::String(ref cursor) = v["Cursor"] {
                    extra.push((*cursor).clone());

                    if let Ok(ret) = cmd_exec(extra.clone()) {
                        v1 = serde_json::from_slice(&ret).unwrap_or(Value::Null);
                        if Value::Null == v {
                            return;
                        }

                        tx.send(ret).unwrap();
                    }

                    extra.pop();
                } else {
                    break;
                }

                v = v1.clone();
            }
        });

        for r in rx {
            self.insert(&holder, r);
        }
    }

    fn insert(&self, holder: &Arc<RwLock<HashMap<String, Ecs>>>, data: Vec<u8>);

    fn argv_new(&self, region: String) -> Vec<String>;
    fn argv_new_base(&self, region: String) -> Vec<String> {
        vec![
            "-region".to_owned(),
            region,
            "-domain".to_owned(),
            "metrics.aliyuncs.com".to_owned(),
            "-apiName".to_owned(),
            "QueryMetricList".to_owned(),
            "-apiVersion".to_owned(),
            "2017-03-01".to_owned(),
            "Action".to_owned(),
            "QueryMetricList".to_owned(),
            "Project".to_owned(),
            "acs_ecs_dashboard".to_owned(),
            "Length".to_owned(),
            "1000".to_owned(),
            "Metric".to_owned(),
        ]
    }
}

impl Ecs {
    fn new() -> Ecs {
        Ecs {
            data: HashMap::new(),
            disk: HashMap::new(),
        }
    }
}

impl META for Meta {
    fn argv_new(&self, region: String) -> Vec<String> {
        vec![
            "-region".to_owned(),
            region.to_owned(),
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

    fn insert(&self, holder: &mut HashMap<String, Ecs>, data: Vec<u8>) {
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
        DT::Ecs
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
fn get_meta <T: META> (mut holder: HashMap<String, Ecs>, region: String, t: T) -> HashMap<String, Ecs> {
    let mut extra = t.argv_new(region.clone());

    if let Ok(ret) = cmd_exec(extra.clone()) {
        let v: Value = serde_json::from_slice(&ret).unwrap_or(Value::Null);
        if Value::Null == v {
            return holder;
        }

        let mut pages;
        if let Value::Number(ref total) = v["TotalCount"] {
            pages = total.as_u64().unwrap_or(0);
            if 0 == pages {
                return holder;
            } else if 0 == pages % 100 {
                pages = pages / 100;
            } else {
                pages = 1 + pages / 100;
            }
        } else {
            return holder;
        }

        t.insert(&mut holder, ret);

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

            for hunk in rx {
                t.insert(&mut holder, hunk);
            }
        }

        match t.reflect() {
            DT::Ecs=> {
                holder = get_meta(holder, region, disk::Meta());
            },
            _ => {}
        }
    }

    holder
}

fn get_data(holder: HashMap<String, Ecs>, region: String) {
    let mut tids = vec![];
    let holder = Arc::new(RwLock::new(holder));

    let h = Arc::clone(&holder);
    let r = region.clone();
    tids.push(thread::spawn(|| {
            cpu::Data().get(h, r);
        }));

    let h = Arc::clone(&holder);
    let r = region.clone();
    tids.push(thread::spawn(|| {
            mem::Data().get(h, r);
        }));

    let h = Arc::clone(&holder);
    let r = region.clone();
    tids.push(thread::spawn(move || {
            disk::Data().get(h, r);
        }));

    let h = Arc::clone(&holder);
    let r = region.clone();
    tids.push(thread::spawn(move || {
            load5m::Data().get(h, r);
        }));

    let h = Arc::clone(&holder);
    let r = region.clone();
    tids.push(thread::spawn(move || {
            load15m::Data().get(h, r);
        }));

    let h = Arc::clone(&holder);
    let r = region.clone();
    tids.push(thread::spawn(move || {
            tcp::Data().get(h, r);
        }));

    let h = Arc::clone(&holder);
    let r = region.clone();
    tids.push(thread::spawn(move || {
            disk::rd::Data().get(h, r);
        }));

    let h = Arc::clone(&holder);
    let r = region.clone();
    tids.push(thread::spawn(move || {
            disk::wr::Data().get(h, r);
        }));

    let h = Arc::clone(&holder);
    let r = region.clone();
    tids.push(thread::spawn(move || {
            disk::rd_tps::Data().get(h, r);
        }));

    let h = Arc::clone(&holder);
    let r = region.clone();
    tids.push(thread::spawn(move || {
            disk::wr_tps::Data().get(h, r);
        }));

    let h = Arc::clone(&holder);
    let r = region.clone();
    tids.push(thread::spawn(move || {
            netif::rd::Data().get(h, r);
        }));

    let h = Arc::clone(&holder);
    let r = region.clone();
    tids.push(thread::spawn(move || {
            netif::wr::Data().get(h, r);
        }));

    let h = Arc::clone(&holder);
    let r = region.clone();
    tids.push(thread::spawn(move || {
            netif::rd_tps::Data().get(h, r);
        }));

    tids.push(thread::spawn(move || {
            netif::wr_tps::Data().get(holder, region);
        }));

    for tid in tids {
        tid.join().unwrap();
    }

    // TODO 发送本次的结果至前端
}

/********************
 * Public InterFace *
 ********************/
pub fn sv() {
    if let Some(regions) = get_region() {
        let mut tids = vec![];
        for region in regions.into_iter() {
            tids.push(thread::spawn(move || {
                    get_data(get_meta(HashMap::new(), region.clone(), Meta()), region);
                }));
        }

        for tid in tids {
            tid.join().unwrap();
        }
    }
}
