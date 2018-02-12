use std::sync::{Arc, Mutex};
use std::collections::HashMap;

use super::NetIf;
use super::super::base;
use super::super::Ecs;
use super::super::super::{DATA, BASESTAMP, INTERVAL};

pub struct Data();

impl DATA for Data {
    type Holder = Arc<Mutex<HashMap<String, Ecs>>>;

    fn argv_new(&self, region: String) -> Vec<String> {
        let mut argv = base::argv_new(region);
        argv.push("networkout_rate".to_owned());

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
        let setter = |netif: &mut NetIf, v: i32| netif.wr = v / 8 / 1024;

        super::insert(holder, data, setter);
    }
}
