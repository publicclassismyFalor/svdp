use ::serde_json;
use serde_json::Value;

use std::process::Command;
use std::collections::HashMap;

use std::thread;
use std::sync::{mpsc, Arc, RwLock};
//use std::time::Duration;

use std::io::Error;

struct Disk {
    device: String,  /* device name: /dev/vda */

    disk_rate: i32,

    rd: u32,  /* kbytes */
    wr: u32,
    rdio: u32,  /* tps */
    wrio: u32,
}

struct NetIf {
    device: String,  /* device name: eth0 */

    rd: u32,  /* kbytes */
    wr: u32,
    rdio: u32,  /* tps */
    wrio: u32,
}

struct Data {
    cpu_rate: i16,
    mem_rate: i16,
    load5m: u16,
    load15m: u16,
    tcp_conn: u32,

    disk: HashMap<String, Disk>,  /* K: device */
    netif: HashMap<String, NetIf>,
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
    NetIf,
}

trait SvMeta {
    fn argv_new(&self, region: &str) -> Vec<String>;
    fn insert(&self, holder: &mut HashMap<String, Ecs>, data: Vec<u8>);
    fn reflect(&self) -> DT;
}

struct SvMetaBase();
struct SvMetaDisk();
struct SvMetaNetIf();

impl SvMeta for SvMetaBase {
    fn argv_new(&self, region: &str) -> Vec<String> {
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
        DT::Base
    }
}

impl SvMeta for SvMetaDisk {
    fn argv_new(&self, region: &str) -> Vec<String> {
        vec![
            "-region".to_owned(),
            region.to_owned(),
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

    fn insert(&self, holder: &mut HashMap<String, Ecs>, data: Vec<u8>) {
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

// TODO
impl SvMeta for SvMetaNetIf {
    fn argv_new(&self, region: &str) -> Vec<String> {
        vec![]
    }

    fn insert(&self, holder: &mut HashMap<String, Ecs>, data: Vec<u8>) {
    }

    fn reflect(&self) -> DT {
        DT::NetIf
    }
}


trait SvData {
    fn argv_new(&self, region: &str, dimensions: String) -> Vec<String>;
    fn insert(&self, holder: &Arc<RwLock<HashMap<String, Ecs>>>, data: Vec<u8>);

    fn argv_new_base(&self, region: &str) -> Vec<String> {
        vec![
            "-region".to_owned(),
            region.to_owned(),
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
        ]
    }
}

struct SvDataCpu();
impl SvData for SvDataCpu {
    fn argv_new(&self, region: &str, dimensions: String) -> Vec<String> {
        let mut argv = self.argv_new_base(region);

        argv.push("Metric".to_owned());
        argv.push("cpu_total".to_owned());
        argv.push("Dimensions".to_owned());
        argv.push(dimensions);

        argv
    }

    fn insert(&self, holder: &Arc<RwLock<HashMap<String, Ecs>>>, data: Vec<u8>) {
    }
}

struct SvDataMem();
impl SvData for SvDataMem {
    fn argv_new(&self, region: &str, dimensions: String) -> Vec<String> {
        let mut argv = self.argv_new_base(region);

        argv.push("Metric".to_owned());
        argv.push("memory_usedutilization".to_owned());
        argv.push("Dimensions".to_owned());
        argv.push(dimensions);

        argv
    }

    fn insert(&self, holder: &Arc<RwLock<HashMap<String, Ecs>>>, data: Vec<u8>) {
    }
}

struct SvDataDisk();
impl SvData for SvDataDisk {
    fn argv_new(&self, region: &str, dimensions: String) -> Vec<String> {
        let mut argv = self.argv_new_base(region);

        argv.push("Metric".to_owned());
        argv.push("diskusage_utilization".to_owned());
        argv.push("Dimensions".to_owned());
        argv.push(dimensions);

        argv
    }

    fn insert(&self, holder: &Arc<RwLock<HashMap<String, Ecs>>>, data: Vec<u8>) {
    }
}

struct SvDataLoad5m();
impl SvData for SvDataLoad5m {
    fn argv_new(&self, region: &str, dimensions: String) -> Vec<String> {
        let mut argv = self.argv_new_base(region);

        argv.push("Metric".to_owned());
        argv.push("load_5m".to_owned());
        argv.push("Dimensions".to_owned());
        argv.push(dimensions);

        argv
    }

    fn insert(&self, holder: &Arc<RwLock<HashMap<String, Ecs>>>, data: Vec<u8>) {
    }
}

struct SvDataLoad15m();
impl SvData for SvDataLoad15m {
    fn argv_new(&self, region: &str, dimensions: String) -> Vec<String> {
        let mut argv = self.argv_new_base(region);

        argv.push("Metric".to_owned());
        argv.push("load_15m".to_owned());
        argv.push("Dimensions".to_owned());
        argv.push(dimensions);

        argv
    }

    fn insert(&self, holder: &Arc<RwLock<HashMap<String, Ecs>>>, data: Vec<u8>) {
    }
}

struct SvDataTcp();
impl SvData for SvDataTcp {
    fn argv_new(&self, region: &str, dimensions: String) -> Vec<String> {
        let mut argv = self.argv_new_base(region);

        argv.push("Metric".to_owned());
        argv.push("net_tcpconnection".to_owned());
        argv.push("Dimensions".to_owned());
        argv.push(dimensions);

        argv
    }

    fn insert(&self, holder: &Arc<RwLock<HashMap<String, Ecs>>>, data: Vec<u8>) {
    }
}

struct SvDataDiskRd();
impl SvData for SvDataDiskRd {
    fn argv_new(&self, region: &str, dimensions: String) -> Vec<String> {
        let mut argv = self.argv_new_base(region);

        argv.push("Metric".to_owned());
        argv.push("disk_readbytes".to_owned());
        argv.push("Dimensions".to_owned());
        argv.push(dimensions);

        argv
    }

    fn insert(&self, holder: &Arc<RwLock<HashMap<String, Ecs>>>, data: Vec<u8>) {
    }
}

struct SvDataDiskWr();
impl SvData for SvDataDiskWr {
    fn argv_new(&self, region: &str, dimensions: String) -> Vec<String> {
        let mut argv = self.argv_new_base(region);

        argv.push("Metric".to_owned());
        argv.push("disk_writebytes".to_owned());
        argv.push("Dimensions".to_owned());
        argv.push(dimensions);

        argv
    }

    fn insert(&self, holder: &Arc<RwLock<HashMap<String, Ecs>>>, data: Vec<u8>) {
    }
}

struct SvDataDiskRdIo();
impl SvData for SvDataDiskRdIo {
    fn argv_new(&self, region: &str, dimensions: String) -> Vec<String> {
        let mut argv = self.argv_new_base(region);

        argv.push("Metric".to_owned());
        argv.push("disk_readiops".to_owned());
        argv.push("Dimensions".to_owned());
        argv.push(dimensions);

        argv
    }

    fn insert(&self, holder: &Arc<RwLock<HashMap<String, Ecs>>>, data: Vec<u8>) {
    }
}

struct SvDataDiskWrIo();
impl SvData for SvDataDiskWrIo {
    fn argv_new(&self, region: &str, dimensions: String) -> Vec<String> {
        let mut argv = self.argv_new_base(region);

        argv.push("Metric".to_owned());
        argv.push("disk_writeiops".to_owned());
        argv.push("Dimensions".to_owned());
        argv.push(dimensions);

        argv
    }

    fn insert(&self, holder: &Arc<RwLock<HashMap<String, Ecs>>>, data: Vec<u8>) {
    }
}

struct SvDataNetIfRd();
impl SvData for SvDataNetIfRd {
    fn argv_new(&self, region: &str, dimensions: String) -> Vec<String> {
        let mut argv = self.argv_new_base(region);

        argv.push("Metric".to_owned());
        argv.push("networkin_rate".to_owned());
        argv.push("Dimensions".to_owned());
        argv.push(dimensions);

        argv
    }

    fn insert(&self, holder: &Arc<RwLock<HashMap<String, Ecs>>>, data: Vec<u8>) {
    }
}

struct SvDataNetIfWr();
impl SvData for SvDataNetIfWr {
    fn argv_new(&self, region: &str, dimensions: String) -> Vec<String> {
        let mut argv = self.argv_new_base(region);

        argv.push("Metric".to_owned());
        argv.push("networkout_rate".to_owned());
        argv.push("Dimensions".to_owned());
        argv.push(dimensions);

        argv
    }

    fn insert(&self, holder: &Arc<RwLock<HashMap<String, Ecs>>>, data: Vec<u8>) {
    }
}

struct SvDataNetIfRdIo();
impl SvData for SvDataNetIfRdIo {
    fn argv_new(&self, region: &str, dimensions: String) -> Vec<String> {
        let mut argv = self.argv_new_base(region);

        argv.push("Metric".to_owned());
        argv.push("networkin_packages".to_owned());
        argv.push("Dimensions".to_owned());
        argv.push(dimensions);

        argv
    }

    fn insert(&self, holder: &Arc<RwLock<HashMap<String, Ecs>>>, data: Vec<u8>) {
    }
}

struct SvDataNetIfWrIo();
impl SvData for SvDataNetIfWrIo {
    fn argv_new(&self, region: &str, dimensions: String) -> Vec<String> {
        let mut argv = self.argv_new_base(region);

        argv.push("Metric".to_owned());
        argv.push("networkout_packages".to_owned());
        argv.push("Dimensions".to_owned());
        argv.push(dimensions);

        argv
    }

    fn insert(&self, holder: &Arc<RwLock<HashMap<String, Ecs>>>, data: Vec<u8>) {
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
fn get_meta <T: SvMeta> (mut holder: HashMap<String, Ecs>, region: String, t: T) -> HashMap<String, Ecs> {
    let mut extra = t.argv_new(&region);

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
            DT::Base => {
                holder = get_meta(holder, region.clone(), SvMetaDisk());
                holder = get_meta(holder, region, SvMetaNetIf());
            },
            _ => {}
        }
    }

    holder
}

fn get_data_worker <T: SvData> (holder: Arc<RwLock<HashMap<String, Ecs>>>, region: String, t: T) {

}

fn get_data(holder: HashMap<String, Ecs>, region: String) {
    let mut tids = vec![];
    let holder = Arc::new(RwLock::new(holder));

    let h = Arc::clone(&holder);
    let r = region.clone();
    tids.push(thread::spawn(move || {
            get_data_worker(h, r, SvDataCpu());
        }));

    let h = Arc::clone(&holder);
    let r = region.clone();
    tids.push(thread::spawn(move || {
            get_data_worker(h, r, SvDataMem());
        }));

    let h = Arc::clone(&holder);
    let r = region.clone();
    tids.push(thread::spawn(move || {
            get_data_worker(h, r, SvDataDisk());
        }));

    let h = Arc::clone(&holder);
    let r = region.clone();
    tids.push(thread::spawn(move || {
            get_data_worker(h, r, SvDataLoad5m());
        }));

    let h = Arc::clone(&holder);
    let r = region.clone();
    tids.push(thread::spawn(move || {
            get_data_worker(h, r, SvDataLoad15m());
        }));

    let h = Arc::clone(&holder);
    let r = region.clone();
    tids.push(thread::spawn(move || {
            get_data_worker(h, r, SvDataTcp());
        }));

    let h = Arc::clone(&holder);
    let r = region.clone();
    tids.push(thread::spawn(move || {
            get_data_worker(h, r, SvDataDiskRd());
        }));

    let h = Arc::clone(&holder);
    let r = region.clone();
    tids.push(thread::spawn(move || {
            get_data_worker(h, r, SvDataDiskWr());
        }));

    let h = Arc::clone(&holder);
    let r = region.clone();
    tids.push(thread::spawn(move || {
            get_data_worker(h, r, SvDataDiskRdIo());
        }));

    let h = Arc::clone(&holder);
    let r = region.clone();
    tids.push(thread::spawn(move || {
            get_data_worker(h, r, SvDataDiskWrIo());
        }));

    let h = Arc::clone(&holder);
    let r = region.clone();
    tids.push(thread::spawn(move || {
            get_data_worker(h, r, SvDataNetIfRd());
        }));

    let h = Arc::clone(&holder);
    let r = region.clone();
    tids.push(thread::spawn(move || {
            get_data_worker(h, r, SvDataNetIfWr());
        }));

    let h = Arc::clone(&holder);
    let r = region.clone();
    tids.push(thread::spawn(move || {
            get_data_worker(h, r, SvDataNetIfRdIo());
        }));

    let r = region.clone();
    tids.push(thread::spawn(move || {
            get_data_worker(holder, r, SvDataNetIfWrIo());
        }));

    for tid in tids {
        tid.join().unwrap();
    }
}

/********************
 * Public InterFace *
 ********************/
pub fn sv() {
    if let Some(regions) = get_region() {
        let mut tids = vec![];
        for region in regions.into_iter() {
            tids.push(thread::spawn(move || {
                    get_data(get_meta(HashMap::new(), region.clone(), SvMetaBase()), region);
                }));
        }

        for tid in tids {
            tid.join().unwrap();
        }
    }
}
