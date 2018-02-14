mod ecs;
mod slb;
mod rds;
mod redis;
mod memcache;
mod mongodb;

use ::std;
use std::thread;
use std::time::Duration;

use std::process::Command;
use std::io::Error;

use ::serde_json;
use serde_json::Value;

pub const PGINFO: &str = "postgres://fh@%2Fhome%2Ffh";

pub const CMD: &str = "/tmp/aliyun_cmdb";
pub const ARGV: &[&str] = &["-userId", "LTAIHYRtkSXC1uTl", "-userKey", "l1eLkvNkVRoPZwV9jwRpmq1xPOefGV"];

pub static mut BASESTAMP: u64 = 0;
pub const INTERVAL: u64 = 1 * 60 * 1000;

pub trait DATA {
    type Holder;

    fn argv_new(&self, region: String) -> Vec<String>;
    fn get(&self, holder: Self::Holder, region: String);
    fn insert(&self, holder: &Self::Holder, data: Vec<u8>);
}

pub fn go() {
    let ts_now = || 1000 * std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).unwrap().as_secs();

    unsafe { BASESTAMP = ts_now() / 15000 * 15000 - INTERVAL; }

    loop {
        let regions;
        match get_region() {
            Some(r) => {
                regions = r;
            },
            None => {
                eprintln!("!!! regions sync failed !!!");
                thread::sleep(Duration::from_secs(10));
                continue;
            }
        }

        let mut basestamp;
        unsafe { basestamp = BASESTAMP; }
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
