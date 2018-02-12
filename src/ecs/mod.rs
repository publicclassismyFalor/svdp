mod cpu;
mod mem;
mod load5m;
mod load15m;
mod tcp;

mod disk;
mod netif;

use ::serde_json;
use serde_json::Value;

use std::process::Command;
use std::collections::HashMap;

use std::thread;
use std::sync::{mpsc, Arc, Mutex};

use std::io::Error;

enum DT {
    Ecs,
    Disk,
}

/* key: instance_id */
struct Ecs {
    data: HashMap<u64, Inner>,  /* K: time_stamp, V: Supervisor Data */

    disk: HashMap<String, String>,  /* K: Device, V: DiskId */
}

struct Inner {
    cpu_rate: i16,
    mem_rate: i16,
    load5m: i32,
    load15m: i32,
    tcp: i32,  /* tcp conn cnt */

    disk: HashMap<String, disk::Disk>,  /* K: device */
    netif: HashMap<String, netif::NetIf>,  /* K: IP */
}

struct Meta();

trait META {
    fn argv_new(&self, region: String) -> Vec<String>;
    fn insert(&self, holder: &Arc<Mutex<HashMap<String, Ecs>>>, data: Vec<u8>);
    fn reflect(&self) -> DT;
}

trait DATA {
    fn get(&self, holder: Arc<Mutex<HashMap<String, Ecs>>>, region: String) {
        let mut extra = self.argv_new(region);

        let (tx, rx) = mpsc::channel();

        thread::spawn(move || {
            if let Ok(ret) = cmd_exec(extra.clone()) {
                let v = serde_json::from_slice(&ret).unwrap_or(Value::Null);
                if Value::Null == v {
                    return;
                }

                tx.send(ret).unwrap();

                if let Value::String(ref cursor) = v["Cursor"] {
                    extra.push("Cursor".to_owned());
                    extra.push((*cursor).clone());

                    while let Ok(ret) = cmd_exec(extra.clone()) {
                        let v = serde_json::from_slice(&ret).unwrap_or(Value::Null);
                        if Value::Null == v {
                            return;
                        }

                        tx.send(ret).unwrap();

                        if let Value::String(ref cursor) = v["Cursor"] {
                            extra.pop();
                            extra.push((*cursor).clone());
                        } else {
                            break;
                        }
                    }
                }
            }
        });

        for r in rx {
            self.insert(&holder, r);
        }
    }

    fn insert(&self, holder: &Arc<Mutex<HashMap<String, Ecs>>>, data: Vec<u8>);

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
            "Period".to_owned(),
            "15".to_owned(),
            "Length".to_owned(),
            "1000".to_owned(),
            "Metric".to_owned(),
        ]
    }
}

impl Ecs {
    fn new() -> Ecs {
        let mut res = Ecs {
            data: HashMap::new(),
            disk: HashMap::new(),
        };

        let ts;
        unsafe { ts = ::BASESTAMP; }

        /* Aliyun TimeStamp: (StartTime, EndTime] */
        for i in 1..(::INTERVAL / 15000 + 1) {
            res.data.insert(ts + i * 15000, Inner::new());
        }

        res
    }
}

impl Inner {
    fn new() -> Inner {
        Inner {
            cpu_rate: 0,
            mem_rate: 0,
            load5m: 0,
            load15m: 0,
            tcp: 0,

            disk: HashMap::new(),
            netif: HashMap::new(),
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

    fn insert(&self, holder: &Arc<Mutex<HashMap<String, Ecs>>>, data: Vec<u8>) {
        let v: Value = serde_json::from_slice(&data).unwrap_or(Value::Null);
        if Value::Null == v {
            return;
        }

        let body = &v["Instances"]["Instance"];
        for i in 0.. {
            if Value::Null == body[i] {
                break;
            } else {
                if let Value::String(ref id) = body[i]["InstanceId"] {
                    holder.lock().unwrap().insert((*id).clone(), Ecs::new());
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
fn get_meta <T: META> (holder: Arc<Mutex<HashMap<String, Ecs>>>, region: String, t: T) {
    let mut extra = t.argv_new(region.clone());

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

        t.insert(&holder, ret);

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
                t.insert(&holder, hunk);
            }
        }

        match t.reflect() {
            DT::Ecs=> {
                let h = Arc::clone(&holder);
                get_meta(h, region, disk::Meta());
            },
            _ => {}
        }
    }
}

fn get_data(holder: Arc<Mutex<HashMap<String, Ecs>>>, region: String) {
    let mut tids = vec![];

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

    let h = Arc::clone(&holder);
    tids.push(thread::spawn(move || {
            netif::wr_tps::Data().get(h, region);
        }));

    for tid in tids {
        tid.join().unwrap();
    }

    /* Final Result */
    let mut ts;
    let mut cpu_rate;
    let mut mem_rate;
    let mut load5m;
    let mut load15m;
    let mut tcp;
    let mut disk: Vec<ResDisk>;
    let mut netif: Vec<ResNetIf>;

    let mut resfinal = ResFinal::new();
    for (ecsid, v) in holder.lock().unwrap().iter() {
        for (k1, v1) in v.data.iter() {
            ts = (k1 / 1000) as i32;
            cpu_rate = v1.cpu_rate;
            mem_rate = v1.mem_rate;
            load5m = v1.load5m;
            load15m = v1.load15m;
            tcp = v1.tcp;

            disk = Vec::new();
            for (k2, v2) in v1.disk.iter() {
                disk.push(ResDisk {
                    id: k2.to_owned(),
                    //id: v.disk.get(k2).unwrap_or(&String::from("_")).to_owned(),
                    rate: v2.rate,
                    rd: v2.rd,
                    wr: v2.wr,
                    rdtps: v2.rdtps,
                    wrtps: v2.wrtps,
                });
            }

            netif = Vec::new();
            for (k3, v3) in v1.netif.iter() {
                netif.push(ResNetIf {
                    ip: k3.to_owned(),
                    rd: v3.rd,
                    wr: v3.wr,
                    rdtps: v3.rdtps,
                    wrtps: v3.wrtps,
                });
            }

            resfinal.res.push(Res::new(ecsid.to_owned(), ts, cpu_rate, mem_rate, load5m, load15m, tcp, disk, netif));
        }
    }

    println!("{}", serde_json::to_string(&resfinal).unwrap());
    // TODO 发送本次的结果至前端
}

#[derive(Serialize, Deserialize)]
struct ResDisk {
    id: String,

    rate: i32,
    rd: i32,
    wr: i32,
    rdtps: i32,
    wrtps: i32,
}

#[derive(Serialize, Deserialize)]
struct ResNetIf {
    ip: String,

    rd: i32,
    wr: i32,
    rdtps: i32,
    wrtps: i32,
}

#[derive(Serialize, Deserialize)]
struct Res {
    id: String,
    ts: i32,

    cpu_rate: i16,
    mem_rate: i16,
    load5m: i32,
    load15m: i32,
    tcp: i32,

    disk: Vec<ResDisk>,
    netif: Vec<ResNetIf>,
}

impl Res {
    fn new(id: String, ts: i32,
           cpu_rate: i16, mem_rate: i16,
           load5m: i32, load15m: i32, tcp: i32,
           disk: Vec<ResDisk>, netif: Vec<ResNetIf>) -> Res {
        Res {
            id,
            ts,
            cpu_rate,
            mem_rate,
            load5m,
            load15m,
            tcp,
            disk,
            netif,
        }
    }
}

#[derive(Serialize, Deserialize)]
struct ResFinal {
    class: String,
    res: Vec<Res>,
}

impl ResFinal {
    fn new() -> ResFinal {
        ResFinal {
            class: "ecs".to_owned(),
            res: Vec::new(),
        }
    }
}

fn insert<F: Fn(&mut Inner, f64)>(holder: &Arc<Mutex<HashMap<String, Ecs>>>, data: Vec<u8>, set: F) {
    let v: Value = serde_json::from_slice(&data).unwrap_or(Value::Null);
    if Value::Null == v {
        return;
    }

    let body = &v["Datapoints"];
    for i in 0.. {
        if Value::Null == body[i] {
            break;
        } else {
            let mut ecsid;
            let mut ts;

            if let Value::String(ref id) = body[i]["instanceId"] {
                ecsid = id;
            } else { continue; }

            if let Value::Number(ref t) = body[i]["timestamp"] {
                if let Some(t) = t.as_u64() {
                    ts = t;
                } else { continue; }
            } else { continue; }

            if let Some(ecs) = holder.lock().unwrap().get_mut(ecsid) {
                /* align with 15s */
                if let Some(mut inner) = ecs.data.get_mut(&(ts / 15000 * 15000)) {
                    if let Value::Number(ref v) = body[i]["Average"] {
                        if let Some(v) = v.as_f64() {
                            set(&mut inner, v);
                        } else { continue; }
                    } else { continue; }
                } else { continue; }
            }
        }
    }
}

/********************
 * Public InterFace *
 ********************/
pub fn sv() {
    if let Some(regions) = get_region() {
        let mut tids = vec![];
        let holder = Arc::new(Mutex::new(HashMap::new()));

        for region in regions.into_iter() {
            let h = Arc::clone(&holder);
            tids.push(thread::spawn(move || {
                get_meta(h, region, Meta());
            }));
        }

        for tid in tids {
            tid.join().unwrap();
        }

        /*
         * Aliyun BUG ?
         * 不传 Dimensions，则 region 字段不起过滤作用，
         * 任一有效值皆会返回所有区域的数据
         */
        get_data(holder, "cn-beijing".to_owned());
    }
}
