pub mod rd;
pub mod wr;
pub mod rd_tps;
pub mod wr_tps;

use std::collections::HashMap;

use super::{DT, META, Ecs};

pub struct NetIf {
    device: String,  /* device name: eth0/em0/bond0/enp2s0 */

    rd: u32,  /* kbytes */
    wr: u32,
    rdio: u32,  /* tps */
    wrio: u32,
}

pub struct Meta();

// FIXME
impl META for Meta {
    fn argv_new(&self, region: &str) -> Vec<String> {
        vec![
            "-region".to_owned(),
            region.to_owned(),
            "-domain".to_owned(),
            "metrics.aliyuncs.com".to_owned(),
            "-apiName".to_owned(),
            "QueryMetricLast".to_owned(),
            "-apiVersion".to_owned(),
            "2017-03-01".to_owned(),
            "Action".to_owned(),
            "QueryMetricLast".to_owned(),
            "Project".to_owned(),
            "acs_ecs_dashboard".to_owned(),
            "Length".to_owned(),
            "1000".to_owned(),
        ]
    }

    fn insert(&self, holder: &mut HashMap<String, Ecs>, data: Vec<u8>) {
    }

    fn reflect(&self) -> DT {
        DT::NetIf
    }
}
