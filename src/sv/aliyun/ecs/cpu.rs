use std::sync::{Arc, Mutex};
use std::collections::HashMap;

use super::base;
use super::{Ecs, Inner};
use super::super::DATA;

pub struct Data;

impl DATA for Data {
    type Holder = Arc<Mutex<HashMap<u64, Ecs>>>;

    fn argv_new(&self, region: String) -> Vec<String> {
        let mut argv = base::argv_new(region);
        argv.push(ME.to_owned());

        argv
    }

    fn insert(&self, holder: &Arc<Mutex<HashMap<u64, Ecs>>>, data: Vec<u8>) {
        base::insert(holder, data, setter);
    }
}

/////////////////////////////////////////////////////
const ME: &str = "cpu_total";

fn setter(inner: &mut Inner, v: f64) {
    inner.cpu_ratio = (v * 10.0) as i16;
}
/////////////////////////////////////////////////////
