mod base;

mod cpu;
mod mem;
mod disk;
mod disk_tps;
mod conn;
mod rd;
mod wr;

use ::serde_json;
use postgres::{Connection, TlsMode};

use std::collections::HashMap;

use std::thread;
use std::sync::{Arc, Mutex};

use super::{DATA, BASESTAMP, INTERVAL};

pub const ACSITEM: &str = "acs_mongodb";
pub const MSPERIOD: u64 = 300000;

/* key: time_stamp */
pub struct MongoDB {
    data: HashMap<String, Inner>,  /* K: instance_id, V: Supervisor Data */
}

#[derive(Serialize, Deserialize)]
pub struct Inner {
    cpu_ratio: i16,
    mem_ratio: i16,
    conn_ratio: i16,

    disk_ratio: i16,
    disktps_ratio: i16,

    rd: i32,
    wr: i32,
}

impl MongoDB {
    fn new() -> MongoDB {
        MongoDB {
            data: HashMap::new(),
        }
    }
}

impl Inner {
    fn new() -> Inner {
        Inner {
            cpu_ratio: 0,
            mem_ratio: 0,
            conn_ratio: 0,
            disk_ratio: 0,
            disktps_ratio: 0,
            rd: 0,
            wr: 0,
        }
    }
}

fn get_data(holder: Arc<Mutex<HashMap<u64, MongoDB>>>, region: String) {
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
    let r = region.clone();
    tids.push(thread::spawn(move || {
            rd::Data.get(h, r);
        }));

    let h = Arc::clone(&holder);
    tids.push(thread::spawn(move || {
            wr::Data.get(h, region);
        }));

    for tid in tids {
        tid.join().unwrap();
    }

    /* write final result to DB */
    if let Ok(pgconn) = Connection::connect(::CONF.pg_login_url.as_str(), TlsMode::None) {
        for (ts, v) in holder.lock().unwrap().iter() {
            if let Err(e) = pgconn.execute(
                "INSERT INTO sv_mongodb VALUES ($1, $2)",
                &[
                    &((ts / 1000) as i32),
                    &serde_json::to_value(&v.data).unwrap()
                ]) {
                eprintln!("[file: {}, line: {}] ==> {}", file!(), line!(), e);
            }
        }
    } else {
        eprintln!("[file: {}, line: {}] ==> DB connect failed.", file!(), line!());
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
        holder.insert(ts + i * MSPERIOD, MongoDB::new());
    }

    let holder = Arc::new(Mutex::new(holder));

    /*
     * Aliyun BUG ?
     * 不传 Dimensions，则 region 字段不起过滤作用，
     * 任一有效值皆会返回所有区域的数据
     */
    get_data(holder, "cn-beijing".to_owned());
}
