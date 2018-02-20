use std::sync::{Arc, Mutex};
use std::collections::HashMap;

use super::base;
use super::{Ecs, Inner};
use super::super::{DATA, BASESTAMP, INTERVAL};

pub struct Data;

impl DATA for Data {
    type Holder = Arc<Mutex<HashMap<u64, Ecs>>>;

    fn argv_new(&self, region: String) -> Vec<String> {
        let mut argv = self.argv_new_base(region);

        argv.push("Period".to_owned());
        argv.push("15".to_owned());
        argv.push("Metric".to_owned());
        argv.push("cpu_total".to_owned());

        argv
    }

    fn insert(&self, holder: &Arc<Mutex<HashMap<u64, Ecs>>>, data: Vec<u8>) {
        let setter = |inner: &mut Inner, v: f64| inner.cpu_rate = (v * 10.0) as i16;

        base::insert(holder, data, setter);
    }
}
