mod base;

mod cpu;
mod mem;
mod disk;
mod disk_tps;
mod conn;
mod delay;

use ::serde_json;
use postgres::{Connection, TlsMode};

use std::collections::HashMap;

use std::thread;
use std::sync::{Arc, Mutex};

use super::{DATA, BASESTAMP, INTERVAL};

pub const ACSITEM: &str = "acs_rds_dashboard";
//pub const MSPERIOD: u64 = 300000;  // ms period
pub const MSPERIOD: u64 = super::CACHEINTERVAL;

/* key: time_stamp */
pub struct Rds {
    data: HashMap<String, Inner>,  /* K: instance_id, V: Supervisor Data */
}

#[derive(Serialize, Deserialize, Clone)]
pub struct Inner {
    cpu_ratio: i16,
    mem_ratio: i16,
    disk_ratio: i16,

    disktps_ratio: i16,  // disk io usage percent: in + out
    conn_ratio: i16,

    delay: i16,  // unit: second
}

impl Rds {
    fn new() -> Rds {
        Rds {
            data: HashMap::new(),
        }
    }
}

impl Inner {
    fn new() -> Inner {
        Inner {
            cpu_ratio: 0,
            mem_ratio: 0,
            disk_ratio: 0,
            disktps_ratio: 0,
            conn_ratio: 0,
            delay: 0,
        }
    }

    fn cpu_ratio(me: &Self, _: &str, _: &str) -> i32 { me.cpu_ratio as i32 }
    fn mem_ratio(me: &Self, _: &str, _: &str) -> i32 { me.mem_ratio as i32 }
    fn disk_ratio(me: &Self, _: &str, _: &str) -> i32 { me.disk_ratio as i32 }
    fn disktps_ratio(me: &Self, _: &str, _: &str) -> i32 { me.disktps_ratio as i32 }
    fn conn_ratio(me: &Self, _: &str, _: &str) -> i32 { me.conn_ratio as i32 }
    fn delay(me: &Self, _: &str, _: &str) -> i32 { me.delay as i32 }

    pub fn get_cb(me: &str) -> Option<fn(&Inner, &str, &str) -> i32> {
        match me {
            "cpu_ratio" => Some(Inner::cpu_ratio),
            "mem_ratio" => Some(Inner::mem_ratio),
            "disk_ratio" => Some(Inner::disk_ratio),
            "disktps_ratio" => Some(Inner::disktps_ratio),
            "conn_ratio" => Some(Inner::conn_ratio),
            "delay" => Some(Inner::delay),
            _ => None
        }
    }
}

fn get_data(holder: Arc<Mutex<HashMap<u64, Rds>>>, region: String) {
    let mut tids = vec![];

    let h = Arc::clone(&holder);
    let r = region.clone();
    tids.push(thread::spawn(move || {
            cpu::Data.get(h, r);
        }));

    let h = Arc::clone(&holder);
    let r = region.clone();
    tids.push(thread::spawn(move || {
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
            disk_tps::Data.get(h, r);
        }));

    let h = Arc::clone(&holder);
    let r = region.clone();
    tids.push(thread::spawn(move || {
            conn::Data.get(h, r);
        }));

    let h = Arc::clone(&holder);
    tids.push(thread::spawn(move || {
            delay::Data.get(h, region);
        }));

    for tid in tids {
        tid.join().unwrap();
    }

    /* write final result to DB */
    if let Ok(pgconn) = Connection::connect(::CONF.pg_login_url.as_str(), TlsMode::None) {
        for (ts, v) in holder.lock().unwrap().iter() {
            if let Err(e) = pgconn.execute(
                "INSERT INTO sv_rds VALUES ($1, $2)",
                &[
                    &((ts / 1000) as i32),
                    &serde_json::to_value(&v.data).unwrap()
                ]) {
                err!(e);
            }

            if 0 == *ts % super::CACHEINTERVAL {
                let mut cache_deque = super::CACHE_RDS.write().unwrap();

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
pub fn sv(_regions: Vec<String>) {
    let mut holder= HashMap::new();

    let ts;
    unsafe { ts = BASESTAMP; }

    /* Aliyun TimeStamp: (StartTime, EndTime] */
    for i in 1..(INTERVAL / MSPERIOD + 1) {
        holder.insert(ts + i * MSPERIOD, Rds::new());
    }

    let holder = Arc::new(Mutex::new(holder));

    /*
     * Aliyun BUG ?
     * 不传 Dimensions，则 region 字段不起过滤作用，
     * 任一有效值皆会返回所有区域的数据
     */
    get_data(holder, "cn-beijing".to_owned());
}
