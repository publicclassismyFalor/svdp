use std::sync::{Arc, Mutex};
use std::collections::HashMap;

use super::base;
use super::{Ecs, Inner};
use super::super::{DATA, BASESTAMP, INTERVAL};

pub struct Data();

impl DATA for Data {
    type Holder = Arc<Mutex<HashMap<String, Ecs>>>;

    fn argv_new(&self, region: String) -> Vec<String> {
        let mut argv = base::argv_new(region);
        argv.push("memory_usedutilization".to_owned());

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
        let setter = |inner: &mut Inner, v: f64| inner.mem_rate = (v * 10.0) as i16;

        base::insert(holder, data, setter);
    }
}
