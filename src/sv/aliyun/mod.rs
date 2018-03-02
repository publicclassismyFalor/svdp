pub mod serv;

pub mod ecs;
pub mod slb;
pub mod rds;
pub mod redis;
pub mod memcache;
pub mod mongodb;

use ::std;
use std::thread;
use std::time::Duration;
use std::sync::{Arc, RwLock};
use std::collections::{HashMap, VecDeque};

use std::fs::File;
use std::io::Read;

use std::sync::mpsc;

use regex::Regex;

use serde_json;
use serde_json::Value;

use rand::Rng;

use reqwest;

use data_encoding::BASE64;
use url::percent_encoding::{utf8_percent_encode, DEFAULT_ENCODE_SET};
use ring::{digest, hmac};

use postgres::{Connection, TlsMode};

use ::time::{strftime, now_utc};

pub const ACCESSID: &str = "LTAIHYRtkSXC1uTl";
//pub const ACCESSKEY: &str = "l1eLkvNkVRoPZwV9jwRpmq1xPOefGV";
pub const SIGKEY: &str = "l1eLkvNkVRoPZwV9jwRpmq1xPOefGV&";  // 即 ACCESSKEY 之后追加一个 '&'，用于 URL 签名 

pub static mut BASESTAMP: u64 = 0;
pub const INTERVAL: u64 = 5 * 60 * 1000;
pub const CACHEINTERVAL: i32 = (INTERVAL / 1000) as i32;  // 与 INTERVAL 同步，确保每次只取一条数据，免去排序的麻烦

type Ecs = Arc<RwLock<VecDeque<(i32, HashMap<String, ecs::Inner>)>>>;
type Slb = Arc<RwLock<VecDeque<(i32, HashMap<String, slb::Inner>)>>>;
type Rds = Arc<RwLock<VecDeque<(i32, HashMap<String, rds::Inner>)>>>;
type MongoDB = Arc<RwLock<VecDeque<(i32, HashMap<String, mongodb::Inner>)>>>;
type Redis = Arc<RwLock<VecDeque<(i32, HashMap<String, redis::Inner>)>>>;
type Memcache = Arc<RwLock<VecDeque<(i32, HashMap<String, memcache::Inner>)>>>;

lazy_static! {
    static ref SV_CLIENT: reqwest::Client = reqwest::Client::new();
}

lazy_static! {
    pub static ref CACHE_ECS: Ecs = Arc::new(RwLock::new(VecDeque::new()));
    pub static ref CACHE_SLB: Slb = Arc::new(RwLock::new(VecDeque::new()));
    pub static ref CACHE_RDS: Rds = Arc::new(RwLock::new(VecDeque::new()));
    pub static ref CACHE_MONGODB: MongoDB = Arc::new(RwLock::new(VecDeque::new()));
    pub static ref CACHE_REDIS: Redis = Arc::new(RwLock::new(VecDeque::new()));
    pub static ref CACHE_MEMCACHE: Memcache = Arc::new(RwLock::new(VecDeque::new()));
}

macro_rules! cacheload {
    ($rows: expr, $mytype: ty, $mycache: path) => {
        for row in $rows {
            let ts: i32 = row.get(0);
            let sv: String = row.get(1);

            if let Ok(svb) = serde_json::from_str::<HashMap<String, $mytype>>(&sv) {
                $mycache.write().unwrap().push_front((ts, svb));
            } else {
                err!(sv);
            }
        }
    }
}

pub fn go() {
    let ts_now = || 1000 * std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).unwrap().as_secs();

    let pgconn = Connection::connect(::CONF.pg_login_url.as_str(), TlsMode::None).unwrap();
    pgconn.execute("CREATE TABLE IF NOT EXISTS sv_meta (last_basestamp int)", &[]).unwrap();

    let rows = pgconn.query("SELECT last_basestamp FROM sv_meta", &[]).unwrap();
    if rows.is_empty() {
        unsafe { BASESTAMP = ts_now() / INTERVAL * INTERVAL - 2 * INTERVAL; }
    } else {
        if let Some(row) = rows.get(0).get(0) {
            let ts: i32 = row;
            unsafe { BASESTAMP = 1000 * ts as u64; }
        } else {
            errexit!("db err");
        }
    }

    let mut basestamp;
    unsafe { basestamp = BASESTAMP; }

    let tbsuffix = &["ecs", "slb", "rds", "mongodb", "redis", "memcache"];

    for tbsuf in tbsuffix {
        pgconn.execute(&format!("CREATE TABLE IF NOT EXISTS sv_{} (ts int, sv jsonb) PARTITION BY RANGE (ts)", tbsuf), &[]).unwrap();
    }

    /* 从 DB 中缓存最多 30 天的数据 */
    for i in 0..30 {
        if mem_insufficient() {
            break;
        } else {
            for j in 0..6 {
                let rows = pgconn.query(
                    &format!("SELECT ts, sv::text FROM sv_{} WHERE ts > {} AND ts <= {} AND ts % {} = 0 ORDER BY ts DESC",
                             tbsuffix[j],
                             basestamp / 1000 - (i + 1) * 24 * 3600,
                             basestamp / 1000 - i * 24 * 3600,
                             CACHEINTERVAL),
                    &[]).unwrap();
                if rows.is_empty() {
                    break;
                } else {
                    match j {
                        0 => {
                            cacheload!(&rows, ecs::Inner, CACHE_ECS);
                        },
                        1 => {
                            cacheload!(&rows, slb::Inner, CACHE_SLB);
                        },
                        2 => {
                            cacheload!(&rows, rds::Inner, CACHE_RDS);
                        },
                        3 => {
                            cacheload!(&rows, mongodb::Inner, CACHE_MONGODB);
                        },
                        4 => {
                            cacheload!(&rows, redis::Inner, CACHE_REDIS);
                        },
                        5 => {
                            cacheload!(&rows, memcache::Inner, CACHE_MEMCACHE);
                        },
                        _ => unreachable!()
                    }
                }
            }
        }
    }

    /* 启动网络服务 */
    start_serv();

    loop {
        let regions;
        match get_region() {
            Some(r) => {
                regions = r;
            },
            None => {
                err!("get region list failed");
                thread::sleep(Duration::from_secs(10));
                continue;
            }
        }

        let mut tbmark = basestamp / 1000 / 3600;
        while (5 + ts_now() / 1000 / 3600) > tbmark {
            for tbsuf in tbsuffix {
                if let Err(e) = pgconn.execute(
                    &format!("CREATE TABLE IF NOT EXISTS sv_{}_{} PARTITION OF sv_{} FOR VALUES FROM ({}) TO ({})",
                    tbsuf,
                    tbmark - 1,
                    tbsuf,
                    (3600 * (tbmark - 1)) as i32,
                    (3600 * tbmark) as i32), &[]) {
                    err!(e);
                }

                /* delete tables created before 90 days ago */
                if let Err(e) = pgconn.execute(
                    &format!("DROP TABLE IF EXISTS sv_{}_{}",
                    tbsuf,
                    tbmark - 1 - 24 * 90), &[]) {
                    err!(e);
                }
            }

            tbmark += 1;
        }

        /*
         * The monitoring data of the Aliyun is not written in real time,
         * need a double delay interval
         */
        while ts_now() >= (basestamp + 2 * INTERVAL) {
            let mut tids = vec![];

            let r = regions.clone();
            tids.push(thread::spawn(move|| ecs::sv(r)));

            let r = regions.clone();
            tids.push(thread::spawn(move|| slb::sv(r)));

            let r = regions.clone();
            tids.push(thread::spawn(move|| rds::sv(r)));

            let r = regions.clone();
            tids.push(thread::spawn(move|| redis::sv(r)));

            let r = regions.clone();
            tids.push(thread::spawn(move|| memcache::sv(r)));

            let r = regions.clone();
            tids.push(thread::spawn(move|| mongodb::sv(r)));

            for tid in tids.into_iter() {
                tid.join().unwrap();
            }

            basestamp += INTERVAL;
            unsafe { BASESTAMP = basestamp; }

            pgconn.execute("DELETE FROM sv_meta", &[]).unwrap();
            pgconn.execute(&format!("INSERT INTO sv_meta VALUES ({})", basestamp / 1000), &[]).unwrap();
        }

        thread::sleep(Duration::from_millis(INTERVAL));
    }
}

include!("mod_include.rs");
