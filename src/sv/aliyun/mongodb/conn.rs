use std::sync::{Arc, Mutex};
use std::collections::HashMap;

use super::base;
use super::{MongoDB, Inner};
use super::super::DATA;

pub struct Data;

impl DATA for Data {
    type Holder = Arc<Mutex<HashMap<u64, MongoDB>>>;

    fn argv_new(&self) -> Vec<[String; 2]> {
        let mut argv = base::argv_new();
        argv.push(["Metric".to_owned(), ME.to_owned()]);

        argv
    }

    fn insert(&self, holder: &Self::Holder, data: Vec<u8>) {
        base::insert(holder, data, setter);
    }
}

/////////////////////////////////////////////////////
const ME: &str = "ConnectionUtilization";

fn setter(inner: &mut Inner, v: f64) {
    inner.conn_ratio = (v * 10.0) as i16;
}
/////////////////////////////////////////////////////
