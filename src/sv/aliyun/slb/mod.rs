mod base;

mod rd;
mod wr;
mod rd_tps;
mod wr_tps;
mod conn;  /* tcp conn cnt */

use ::serde_json;
use postgres::{Connection, TlsMode};

use std::collections::HashMap;

use std::thread;
use std::sync::{Arc, Mutex};

use super::{DATA, BASESTAMP, INTERVAL};

pub const ACSITEM: &str = "acs_slb_dashboard";
//pub const MSPERIOD: u64 = 60000;  // ms period
pub const MSPERIOD: u64 = (super::CACHEINTERVAL as u64) * 1000;

/* key: time_stamp */
pub struct Slb {
    data: HashMap<String, Inner>,  /* K: instance_id, V: Supervisor Data */
}

#[derive(Serialize, Deserialize, Clone)]
pub struct Inner {
    rd: i32,  /* kbytes */
    wr: i32,
    rdtps: i32,
    wrtps: i32,
    conn: i32,
}

impl Slb {
    fn new() -> Slb {
        Slb {
            data: HashMap::new(),
        }
    }
}

impl Inner {
    fn new() -> Inner {
        Inner {
            rd: 0,
            wr: 0,
            rdtps: 0,
            wrtps: 0,
            conn: 0,
        }
    }

    fn rd(me: &Self, _: &str, _: &str) -> i32 { me.rd }
    fn wr(me: &Self, _: &str, _: &str) -> i32 { me.wr }
    fn rdtps(me: &Self, _: &str, _: &str) -> i32 { me.rdtps }
    fn wrtps(me: &Self, _: &str, _: &str) -> i32 { me.wrtps }
    fn conn(me: &Self, _: &str, _: &str) -> i32 { me.conn }

    pub fn get_cb(me: &str) -> Option<fn(&Inner, &str, &str) -> i32> {
        match me {
            "rd" => Some(Inner::rd),
            "wr" => Some(Inner::wr),
            "rdtps" => Some(Inner::rdtps),
            "wrtps" => Some(Inner::wrtps),
            "conn" => Some(Inner::conn),
            _ => None
        }
    }
}

fn get_data(holder: Arc<Mutex<HashMap<u64, Slb>>>, region: String) {
    let mut tids = vec![];

    let h = Arc::clone(&holder);
    let r = region.clone();
    tids.push(thread::spawn(move || {
            rd::Data.get(h, r);
        }));

    let h = Arc::clone(&holder);
    let r = region.clone();
    tids.push(thread::spawn(move || {
            wr::Data.get(h, r);
        }));

    let h = Arc::clone(&holder);
    let r = region.clone();
    tids.push(thread::spawn(move || {
            rd_tps::Data.get(h, r);
        }));

    let h = Arc::clone(&holder);
    let r = region.clone();
    tids.push(thread::spawn(move || {
            wr_tps::Data.get(h, r);
        }));

    let h = Arc::clone(&holder);
    tids.push(thread::spawn(move || {
            conn::Data.get(h, region);
        }));

    for tid in tids {
        tid.join().unwrap();
    }

    /* write final result to DB */
    if let Ok(pgconn) = Connection::connect(::CONF.pg_login_url.as_str(), TlsMode::None) {
        for (ts, v) in holder.lock().unwrap().iter() {
            let ts = (ts / 1000) as i32;
            if let Err(e) = pgconn.execute(
                "INSERT INTO sv_slb VALUES ($1, $2)",
                &[
                    &ts,
                    &serde_json::to_value(&v.data).unwrap()
                ]) {
                err!(e);
            }

            if 0 == ts % super::CACHEINTERVAL {
                let mut cache_deque = super::CACHE_SLB.write().unwrap();

                /* 若系统内存占用已超过阀值，则销毁最旧的数据条目 */
                if super::mem_insufficient() {
                    cache_deque.pop_front();
                }

                cache_deque.push_back((ts, v.data.clone()));
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
        holder.insert(ts + i * MSPERIOD, Slb::new());
    }

    let holder = Arc::new(Mutex::new(holder));

    /*
     * Aliyun BUG ?
     * 不传 Dimensions，则 region 字段不起过滤作用，
     * 任一有效值皆会返回所有区域的数据
     */
    get_data(holder, "cn-beijing".to_owned());
}
