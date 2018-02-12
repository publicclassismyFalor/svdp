use std::sync::{Arc, Mutex};
use std::collections::HashMap;

use super::{DATA, Ecs, Inner};
use super::super::{BASESTAMP, INTERVAL};

pub struct Data();

impl DATA for Data {
    fn argv_new(&self, region: String) -> Vec<String> {
        let mut argv = self.argv_new_base(region);
        argv.push("load_15m".to_owned());

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
        let setter = |inner: &mut Inner, v: f64| inner.load15m = (v * 1000.0) as i32;

        super::insert(holder, data, setter);
    }
}
