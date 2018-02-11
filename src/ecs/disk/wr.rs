use std::sync::{Arc, RwLock};
use std::collections::HashMap;

use super::super::{DATA, Ecs};

pub struct Data();

impl DATA for Data {
    fn argv_new(&self, region: &str, dimensions: String) -> Vec<String> {
        let mut argv = self.argv_new_base(region);

        argv.push("Metric".to_owned());
        argv.push("disk_writebytes".to_owned());
        argv.push("Dimensions".to_owned());
        argv.push(dimensions);

        argv
    }

    fn insert(&self, holder: &Arc<RwLock<HashMap<String, Ecs>>>, data: Vec<u8>) {
    }
}
