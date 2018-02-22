use std::sync::{Arc, Mutex};
use std::collections::HashMap;

use super::base;
use super::{MongoDB, Inner};
use super::super::DATA;

pub struct Data;

impl DATA for Data {
    type Holder = Arc<Mutex<HashMap<u64, MongoDB>>>;

    fn argv_new(&self, region: String) -> Vec<String> {
        let mut argv = base::argv_new(region);
        argv.push("ConnectionUtilization".to_owned());

        argv
    }

    fn insert(&self, holder: &Self::Holder, data: Vec<u8>) {
        let setter = |inner: &mut Inner, v: f64| inner.conn_rate = (v * 10.0) as i16;

        base::insert(holder, data, setter);
    }
}
