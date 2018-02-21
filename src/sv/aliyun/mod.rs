mod ecs;
mod slb;
mod rds;
mod redis;
mod memcache;
mod mongodb;

use ::std;
use std::thread;
use std::time::Duration;

use std::io::Error;
use std::process::Command;

use std::sync::mpsc;

use ::serde_json;
use serde_json::Value;
use postgres::{Connection, TlsMode};

pub const PGINFO: &str = "postgres://fh@%2Fhome%2Ffh/svdp";

pub const CMD: &str = "/tmp/aliyun_cmdb";
pub const ARGV: &[&str] = &["-userId", "LTAIHYRtkSXC1uTl", "-userKey", "l1eLkvNkVRoPZwV9jwRpmq1xPOefGV"];

pub static mut BASESTAMP: u64 = 0;
pub const INTERVAL: u64 = 5 * 60 * 1000;

pub fn argv_new_base(region: String) -> Vec<String> {
    let mut argv = vec![
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
        "Length".to_owned(),
        "1000".to_owned(),
    ];

    argv.push("StartTime".to_owned());
    unsafe {
        argv.push(BASESTAMP.to_string());
    }

    argv.push("EndTime".to_owned());
    unsafe {
        argv.push((BASESTAMP + INTERVAL).to_string());
    }

    argv
}

pub trait DATA {
    type Holder;

    fn argv_new(&self, region: String) -> Vec<String>;

    fn get(&self, holder: Self::Holder, region: String) {
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

    fn insert(&self, holder: &Self::Holder, data: Vec<u8>);
}

pub fn go() {
    let ts_now = || 1000 * std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).unwrap().as_secs();
    unsafe { BASESTAMP = ts_now() / INTERVAL * INTERVAL - INTERVAL; }

    let pgconn = Connection::connect(PGINFO, TlsMode::None).unwrap();
    let tbsuffix = &["ecs", "slb", "rds", "redis", "memcache", "mongodb"];

    for tbsuf in tbsuffix {
        pgconn.execute(&format!("CREATE TABLE IF NOT EXISTS sv_{} (ts int PRIMARY KEY, sv jsonb) PARTITION BY RANGE (ts)", tbsuf), &[]).unwrap();
    }

    loop {
        let regions;
        match get_region() {
            Some(r) => {
                regions = r;
            },
            None => {
                eprintln!("[file: {}, line: {}] ==> !!! regions sync failed !!!", file!(), line!());
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
                    eprintln!("[file: {}, line: {}] ==> {}", file!(), line!(), e);
                }
            }

            tbmark += 1;
        }

        while ts_now() >= (basestamp + INTERVAL) {
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
        }

        thread::sleep(Duration::from_secs(INTERVAL));
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

fn cmd_exec(mut extra: Vec<String>) -> Result<Vec<u8>, Error> {
    let mut argv: Vec<String> = Vec::new();

    for x in ARGV.iter() {
        argv.push((**x).to_string());
    }

    argv.append(&mut extra);

    let output = Command::new(CMD).args(argv).output() ?;

    if output.status.success() {
        return Ok(output.stdout);
    } else {
        return Err(Error::from_raw_os_error(output.status.code().unwrap_or(1)));
    }
}
