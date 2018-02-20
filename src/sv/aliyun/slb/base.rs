use ::serde_json;
use serde_json::Value;

use std::collections::HashMap;
use std::sync::{Arc, Mutex};

use super::{Slb, Inner};
use ::sv::aliyun;

pub fn argv_new(region: String) -> Vec<String> {
    let mut argv = aliyun::argv_new_base(region);

    argv.push("Period".to_owned());
    argv.push("60".to_owned());
    argv.push("Metric".to_owned());

    argv
}

pub fn insert<F: Fn(&mut Inner, i32)>(holder: &Arc<Mutex<HashMap<u64, Slb>>>, data: Vec<u8>, set: F) {
}
