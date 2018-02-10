extern crate serde_json;

mod ecs;
mod slb;
mod rds;
mod redis;
mod memcache;
mod mongodb;

use std::thread;

const CMD: &str = "/tmp/aliyun_cmdb";
const ARGV: &[&str] = &["-userId", "LTAIHYRtkSXC1uTl", "-userKey", "l1eLkvNkVRoPZwV9jwRpmq1xPOefGV"];

static mut TIMESTAMP: u64 = 0;

pub fn run() {
    unsafe {
        TIMESTAMP = 1000 * (std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).unwrap().as_secs() - 15 * 60);
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
