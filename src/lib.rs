extern crate serde_json;

mod ecs;
mod slb;
mod rds;
mod redis;
mod memcache;
mod mongodb;

use std::thread;
use std::time::Duration;

pub const CMD: &str = "/tmp/aliyun_cmdb";
pub const ARGV: &[&str] = &["-userId", "LTAIHYRtkSXC1uTl", "-userKey", "l1eLkvNkVRoPZwV9jwRpmq1xPOefGV"];

pub static mut BASESTAMP: u64 = 0;
pub const INTERVAL: u64 = 5 * 60 * 1000;

pub fn run() {
    let ts_now = || -> u64 {1000 * std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).unwrap().as_secs()};

    unsafe { BASESTAMP = ts_now() / 15000 * 15000 - INTERVAL; }

    let mut basestamp;
    loop {
        unsafe { basestamp = BASESTAMP; }

        while ts_now() >= (basestamp + INTERVAL) {
            let mut tids = vec![];

            tids.push(thread::spawn(|| ecs::sv()));
            tids.push(thread::spawn(|| slb::sv()));
            tids.push(thread::spawn(|| rds::sv()));
            tids.push(thread::spawn(|| redis::sv()));
            tids.push(thread::spawn(|| memcache::sv()));
            tids.push(thread::spawn(|| mongodb::sv()));

            for tid in tids.into_iter() {
                tid.join().unwrap();
            }

            basestamp += INTERVAL;
        }

        unsafe { BASESTAMP = basestamp; }

        thread::sleep(Duration::from_secs(INTERVAL));
    }
}
