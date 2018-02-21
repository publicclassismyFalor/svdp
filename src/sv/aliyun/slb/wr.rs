use std::sync::{Arc, Mutex};
use std::collections::HashMap;

use super::base;
use super::{Slb, Inner};
use super::super::DATA;

pub struct Data;

impl DATA for Data {
    type Holder = Arc<Mutex<HashMap<u64, Slb>>>;

    fn argv_new(&self, region: String) -> Vec<String> {
        let mut argv = base::argv_new(region);
        argv.push("TrafficTXNew".to_owned());

        argv
    }

    fn insert(&self, holder: &Self::Holder, data: Vec<u8>) {
        let setter = |inner: &mut Inner, v: i32| inner.wr = v / 8 / 1024;

        base::insert(holder, data, setter);
    }
}
