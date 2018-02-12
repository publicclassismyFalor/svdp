use std::sync::{Arc, Mutex};
use std::collections::HashMap;

use super::base;
use super::{Ecs, Inner};
use super::super::{DATA, BASESTAMP, INTERVAL};

pub struct Data();

impl DATA for Data {
    type Holder = Arc<Mutex<HashMap<String, Ecs>>>;

    fn argv_new(&self, region: String) -> Vec<String> {
        let mut argv = self.argv_new_base(region);
        argv.push("load_5m".to_owned());

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

    fn insert(&self, holder: &Arc<Mutex<HashMap<String, Ecs>>>, data: Vec<u8>) {
        let setter = |inner: &mut Inner, v: f64| inner.load5m = (v * 1000.0) as i32;

        base::insert(holder, data, setter);
    }
}
