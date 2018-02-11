extern crate serde_json;

mod ecs;
mod slb;
mod rds;
mod redis;
mod memcache;
mod mongodb;

use std::thread;

pub const CMD: &str = "/tmp/aliyun_cmdb";
pub const ARGV: &[&str] = &["-userId", "LTAIHYRtkSXC1uTl", "-userKey", "l1eLkvNkVRoPZwV9jwRpmq1xPOefGV"];

pub static mut BASESTAMP: u64 = 0;
pub const INTERVAL: u64 = 5 * 60 * 1000;

pub fn run() {
    unsafe {
        BASESTAMP = 1000 * (std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).unwrap().as_secs() - 5 * 60);
    }

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
}
