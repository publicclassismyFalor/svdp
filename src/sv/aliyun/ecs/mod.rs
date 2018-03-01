mod cpu;
mod mem;
mod load5m;
mod load15m;
mod tcp;

pub mod disk;
pub mod netif;

mod base;

use ::serde_json;
use serde_json::Value;
use postgres::{Connection, TlsMode};

use std::collections::HashMap;

use std::thread;
use std::sync::{mpsc, Arc, Mutex};

use super::{DATA, BASESTAMP, INTERVAL, cmd_exec};

pub const ACSITEM: &str = "acs_ecs_dashboard";
//pub const MSPERIOD: u64 = 15000;  // ms period
pub const MSPERIOD: u64 = super::CACHEINTERVAL;

//enum DT {
//    Ecs,
//    Disk,
//}

/* key: time_stamp */
pub struct Ecs {
    data: HashMap<String, Inner>,  /* K: instance_id, V: Supervisor Data */
    //disk: HashMap<String, String>,  /* K: Device, V: DiskId */
}

impl Ecs {
    fn new() -> Ecs {
        Ecs {
            data: HashMap::new(),
            //disk: HashMap::new(),
        }
    }
}

#[derive(Serialize, Deserialize, Clone)]
pub struct Inner {
    cpu_ratio: i16,
    mem_ratio: i16,
    load5m: i32,
    load15m: i32,
    tcp: i32,  /* tcp conn cnt */

    disk: HashMap<String, disk::Disk>,  /* K: device */
    netif: HashMap<String, netif::NetIf>,  /* K: IP */
}

impl Inner {
    fn new() -> Inner {
        Inner {
            cpu_ratio: 0,
            mem_ratio: 0,
            load5m: 0,
            load15m: 0,
            tcp: 0,

            disk: HashMap::new(),
            netif: HashMap::new(),
        }
    }

    fn cpu_ratio(me: &Self, _: &str, _: &str) -> i32 { me.cpu_ratio as i32 }
    fn mem_ratio(me: &Self, _: &str, _: &str) -> i32 { me.mem_ratio as i32 }
    fn load5m(me: &Self, _: &str, _: &str) -> i32 { me.load5m }
    fn load15m(me: &Self, _: &str, _: &str) -> i32 { me.load15m }
    fn tcp(me: &Self, _: &str, _: &str) -> i32 { me.tcp }

    fn disk(me: &Self, dev: &str, item: &str) -> i32 {
        if let Some(v) = me.disk.get(dev) {
            match item {
                "ratio" => v.ratio,
                "rd" => v.rd,
                "wr" => v.wr,
                "rdtps" => v.rdtps,
                "wrtps" => v.wrtps,
                _ => -1
            }
        } else {
            -1
        }
    }

    fn netif(me: &Self, dev: &str, item: &str) -> i32 {
        if let Some(v) = me.netif.get(dev) {
            match item {
                "rd" => v.rd,
                "wr" => v.wr,
                "rdtps" => v.rdtps,
                "wrtps" => v.wrtps,
                _ => -1
            }
        } else {
            -1
        }
    }

    pub fn get_cb(me: &str) -> Option<fn(&Inner, &str, &str) -> i32> {
        match me {
            "cpu_ratio" => Some(Inner::cpu_ratio),
            "mem_ratio" => Some(Inner::mem_ratio),
            "load5m" => Some(Inner::load5m),
            "load15m" => Some(Inner::load15m),
            "tcp" => Some(Inner::tcp),
            "disk" => Some(Inner::disk),
            "netif" => Some(Inner::netif),
            _ => None
        }
    }
}

struct Meta;

trait META {
    fn argv_new(&self, region: String) -> Vec<String>;
    fn insert(&self, holder: &Arc<Mutex<HashMap<u64, Ecs>>>, data: Vec<u8>);
    //fn reflect(&self) -> DT;
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

    fn insert(&self, holder: &Arc<Mutex<HashMap<u64, Ecs>>>, data: Vec<u8>) {
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
                    let mut h = holder.lock().unwrap();
                    for (_, ecs) in h.iter_mut() {
                        ecs.data.insert((*id).clone(), Inner::new());
                    }
                }
            }
        }
    }

    //fn reflect(&self) -> DT {
    //    DT::Ecs
    //}
}

/*
 * return HashMap(contains meta info of all ecs+disk+netif)
 * @param start_time: unix time_stamp
 */
fn get_meta <T: META> (holder: Arc<Mutex<HashMap<u64, Ecs>>>, region: String, t: T) {
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
                        tx.send(ret).unwrap_or_else(|e|{ err!(e); });
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

        //match t.reflect() {
        //    DT::Ecs=> {
        //        let h = Arc::clone(&holder);
        //        get_meta(h, region, disk::Meta());
        //    },
        //    _ => {}
        //}
    }
}

fn get_data(holder: Arc<Mutex<HashMap<u64, Ecs>>>, region: String) {
    let mut tids = vec![];

    let h = Arc::clone(&holder);
    let r = region.clone();
    tids.push(thread::spawn(|| {
            cpu::Data.get(h, r);
        }));

    let h = Arc::clone(&holder);
    let r = region.clone();
    tids.push(thread::spawn(|| {
            mem::Data.get(h, r);
        }));

    let h = Arc::clone(&holder);
    let r = region.clone();
    tids.push(thread::spawn(move || {
            disk::Data.get(h, r);
        }));

    let h = Arc::clone(&holder);
    let r = region.clone();
    tids.push(thread::spawn(move || {
            load5m::Data.get(h, r);
        }));

    let h = Arc::clone(&holder);
    let r = region.clone();
    tids.push(thread::spawn(move || {
            load15m::Data.get(h, r);
        }));

    let h = Arc::clone(&holder);
    let r = region.clone();
    tids.push(thread::spawn(move || {
            tcp::Data.get(h, r);
        }));

    let h = Arc::clone(&holder);
    let r = region.clone();
    tids.push(thread::spawn(move || {
            disk::rd::Data.get(h, r);
        }));

    let h = Arc::clone(&holder);
    let r = region.clone();
    tids.push(thread::spawn(move || {
            disk::wr::Data.get(h, r);
        }));

    let h = Arc::clone(&holder);
    let r = region.clone();
    tids.push(thread::spawn(move || {
            disk::rd_tps::Data.get(h, r);
        }));

    let h = Arc::clone(&holder);
    let r = region.clone();
    tids.push(thread::spawn(move || {
            disk::wr_tps::Data.get(h, r);
        }));

    let h = Arc::clone(&holder);
    let r = region.clone();
    tids.push(thread::spawn(move || {
            netif::rd::Data.get(h, r);
        }));

    let h = Arc::clone(&holder);
    let r = region.clone();
    tids.push(thread::spawn(move || {
            netif::wr::Data.get(h, r);
        }));

    let h = Arc::clone(&holder);
    let r = region.clone();
    tids.push(thread::spawn(move || {
            netif::rd_tps::Data.get(h, r);
        }));

    let h = Arc::clone(&holder);
    tids.push(thread::spawn(move || {
            netif::wr_tps::Data.get(h, region);
        }));

    for tid in tids {
        tid.join().unwrap();
    }

    /* write final result to DB */
    if let Ok(pgconn) = Connection::connect(::CONF.pg_login_url.as_str(), TlsMode::None) {
        for (ts, v) in holder.lock().unwrap().iter() {
            if let Err(e) = pgconn.execute(
                "INSERT INTO sv_ecs VALUES ($1, $2)",
                &[
                    &((ts / 1000) as i32),
                    &serde_json::to_value(&v.data).unwrap()
                ]) {
                err!(e);
            }

            if 0 == *ts % super::CACHEINTERVAL {
                let mut cache_deque = super::CACHE_ECS.write().unwrap();

                /* 若系统内存占用已超过阀值，则销毁最旧的数据条目 */
                if super::mem_insufficient() {
                    cache_deque.pop_front();
                }

                cache_deque.push_back((*ts as i32, v.data.clone()));
            }
        }
    } else {
        err!("DB connect failed");
    }
}

/********************
 * Public InterFace *
 ********************/
pub fn sv(regions: Vec<String>) {
    let mut holder= HashMap::new();

    let ts;
    unsafe { ts = BASESTAMP; }

    /* Aliyun TimeStamp: (StartTime, EndTime] */
    for i in 1..(INTERVAL / MSPERIOD + 1) {
        holder.insert(ts + i * MSPERIOD, Ecs::new());
    }

    let holder = Arc::new(Mutex::new(holder));

    let mut tids = vec![];
    for region in regions.into_iter() {
        let h = Arc::clone(&holder);
        tids.push(thread::spawn(move || {
            get_meta(h, region, Meta);
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
