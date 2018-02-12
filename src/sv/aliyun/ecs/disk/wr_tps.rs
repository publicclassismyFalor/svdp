use std::sync::{Arc, Mutex};
use std::collections::HashMap;

use super::Disk;
use super::super::base;
use super::super::Ecs;
use super::super::super::{DATA, BASESTAMP, INTERVAL};

pub struct Data();

impl DATA for Data {
    type Holder = Arc<Mutex<HashMap<String, Ecs>>>;

    fn argv_new(&self, region: String) -> Vec<String> {
        let mut argv = base::argv_new(region);
        argv.push("disk_writeiops".to_owned());

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

    fn get(&self, holder: Self::Holder, region: String) {
        base::get(holder, region, Data());
    }

    fn insert(&self, holder: &Arc<Mutex<HashMap<String, Ecs>>>, data: Vec<u8>) {
        let setter = |disk: &mut Disk, v: i32| disk.wrtps = v;

        super::insert(holder, data, setter);
    }
}