pub mod rd;
pub mod wr;
pub mod rd_tps;
pub mod wr_tps;

use std::collections::HashMap;

use super::{DT, META, Ecs};

pub struct NetIf {
    device: String,  /* device name: eth0 */

    rd: u32,  /* kbytes */
    wr: u32,
    rdio: u32,  /* tps */
    wrio: u32,
}

pub struct Meta();

// FIXME
impl META for Meta {
    fn argv_new(&self, region: &str) -> Vec<String> {
        vec![]
    }

    fn insert(&self, holder: &mut HashMap<String, Ecs>, data: Vec<u8>) {
    }

    fn reflect(&self) -> DT {
        DT::NetIf
    }
}
