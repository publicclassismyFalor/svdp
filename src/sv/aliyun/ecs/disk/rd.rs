use std::sync::{Arc, Mutex};
use std::collections::HashMap;

use super::Disk;
use super::super::base;
use super::super::Ecs;
use super::super::super::{DATA, BASESTAMP, INTERVAL};

pub struct Data;

impl DATA for Data {
    type Holder = Arc<Mutex<HashMap<u64, Ecs>>>;

    fn argv_new(&self, region: String) -> Vec<String> {
        let mut argv = base::argv_new(region);
        argv.push("disk_readbytes".to_owned());

        argv
    }

    fn insert(&self, holder: &Arc<Mutex<HashMap<u64, Ecs>>>, data: Vec<u8>) {
        let setter = |disk: &mut Disk, v: i32| disk.rd = v / 1024;

        super::insert(holder, data, setter);
    }
}
