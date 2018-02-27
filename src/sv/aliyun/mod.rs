mod ecs;
mod slb;
mod rds;
mod redis;
mod memcache;
mod mongodb;

use ::std;
use std::thread;
use std::time::Duration;
use std::sync::{Arc, RwLock};
use std::collections::{HashMap, VecDeque};

use std::io::Error;
use std::process::Command;

use std::sync::mpsc;

use ::serde_json;
use serde_json::Value;
use postgres::{Connection, TlsMode};

pub const CMD: &str = "/tmp/aliyun_cmdb";
pub const ARGV: &[&str] = &["-userId", "LTAIHYRtkSXC1uTl", "-userKey", "l1eLkvNkVRoPZwV9jwRpmq1xPOefGV"];

pub static mut BASESTAMP: u64 = 0;
pub const INTERVAL: u64 = 5 * 60 * 1000;
pub const CACHEINTERVAL: u64 = 5 * 60;

lazy_static! {
    pub static ref CACHE_ECS: Arc<RwLock<VecDeque<(i32, HashMap<String, ecs::Inner>)>>> = Arc::new(RwLock::new(VecDeque::new()));
    pub static ref CACHE_SLB: Arc<RwLock<VecDeque<(i32, HashMap<String, slb::Inner>)>>> = Arc::new(RwLock::new(VecDeque::new()));
    pub static ref CACHE_RDS: Arc<RwLock<VecDeque<(i32, HashMap<String, rds::Inner>)>>> = Arc::new(RwLock::new(VecDeque::new()));
    pub static ref CACHE_MONGODB: Arc<RwLock<VecDeque<(i32, HashMap<String, mongodb::Inner>)>>> = Arc::new(RwLock::new(VecDeque::new()));
    pub static ref CACHE_REDIS: Arc<RwLock<VecDeque<(i32, HashMap<String, redis::Inner>)>>> = Arc::new(RwLock::new(VecDeque::new()));
    pub static ref CACHE_MEMCACHE: Arc<RwLock<VecDeque<(i32, HashMap<String, memcache::Inner>)>>> = Arc::new(RwLock::new(VecDeque::new()));
}

pub fn go() {
    let ts_now = || 1000 * std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).unwrap().as_secs();

    let pgconn = Connection::connect(::CONF.pg_login_url.as_str(), TlsMode::None).unwrap();
    pgconn.execute("CREATE TABLE IF NOT EXISTS sv_meta (last_basestamp int)", &[]).unwrap();

    let qrow = pgconn.query("SELECT last_basestamp FROM sv_meta", &[]).unwrap();
    if qrow.is_empty() {
        unsafe { BASESTAMP = ts_now() / INTERVAL * INTERVAL - 2 * INTERVAL; }
    } else {
        if let Some(qres) = qrow.get(0).get(0) {
            let ts: i32 = qres;
            unsafe { BASESTAMP = 1000 * ts as u64; }
        } else {
            errexit!("db err");
        }
    }

    let tbsuffix = &["ecs", "slb", "rds", "redis", "memcache", "mongodb"];

    for tbsuf in tbsuffix {
        pgconn.execute(&format!("CREATE TABLE IF NOT EXISTS sv_{} (ts int, sv jsonb) PARTITION BY RANGE (ts)", tbsuf), &[]).unwrap();
    }

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

        let mut basestamp;
        unsafe { basestamp = BASESTAMP; }

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

                /* delete tables created before 10 days ago */
                if let Err(e) = pgconn.execute(
                    &format!("DROP TABLE IF EXISTS sv_{}_{}",
                    tbsuf,
                    tbmark - 1 - 240), &[]) {
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
