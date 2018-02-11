use std::sync::{Arc, RwLock};
use std::collections::HashMap;

use super::super::{DATA, Ecs};

pub struct Data();

impl DATA for Data {
    fn argv_new(&self, region: String) -> Vec<String> {
        let mut argv = self.argv_new_base(region);
        argv.push("disk_writeiops".to_owned());

        argv.push("StartTime".to_owned());
        unsafe {
            argv.push(::BASESTAMP.to_string());
        }

        argv.push("EndTime".to_owned());
        unsafe {
            argv.push((::BASESTAMP + ::INTERVAL).to_string());
        }

        argv
    }

    fn insert(&self, holder: &Arc<RwLock<HashMap<String, Ecs>>>, data: Vec<u8>) {
    }
}
