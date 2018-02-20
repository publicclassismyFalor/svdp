use std::sync::{Arc, Mutex};
use std::collections::HashMap;

use super::NetIf;
use super::super::base;
use super::super::Ecs;
use super::super::super::{DATA, BASESTAMP, INTERVAL};

pub struct Data;

impl DATA for Data {
    type Holder = Arc<Mutex<HashMap<u64, Ecs>>>;

    fn argv_new(&self, region: String) -> Vec<String> {
        let mut argv = base::argv_new(region);
        argv.push("networkin_rate".to_owned());

        argv
    }

    fn insert(&self, holder: &Arc<Mutex<HashMap<u64, Ecs>>>, data: Vec<u8>) {
        let setter = |netif: &mut NetIf, v: i32| netif.rd = v / 8 / 1024;

        super::insert(holder, data, setter);
    }
}
