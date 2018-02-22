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
const ME: &str = "load_15m";

fn setter(inner: &mut Inner, v: f64) {
    inner.load15m = (v * 1000.0) as i32;
}
/////////////////////////////////////////////////////
