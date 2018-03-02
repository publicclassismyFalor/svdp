use std::sync::{Arc, Mutex};
use std::collections::HashMap;

use super::Disk;
use super::super::base;
use super::super::Ecs;
use super::super::super::DATA;

pub struct Data;  /* IOps */

impl DATA for Data {
    type Holder = Arc<Mutex<HashMap<u64, Ecs>>>;

    fn argv_new(&self) -> Vec<[String; 2]> {
        let mut argv = base::argv_new();
        argv.push(["Metric".to_owned(), ME.to_owned()]);

        argv
    }

    fn insert(&self, holder: &Arc<Mutex<HashMap<u64, Ecs>>>, data: Vec<u8>) {
        super::insert(holder, data, setter);
    }
}

/////////////////////////////////////////////////////
const ME: &str = "disk_readiops";

fn setter(disk: &mut Disk, v: i32) {
    disk.rdtps = v;
}
/////////////////////////////////////////////////////
