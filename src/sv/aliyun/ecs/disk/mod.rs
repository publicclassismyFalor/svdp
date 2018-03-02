use std::sync::{Arc, Mutex};
use std::collections::HashMap;

use ::serde_json;
use serde_json::Value;

use super::base;
//use super::{DT, META};
use super::{Ecs, MSPERIOD};
use super::super::DATA;

pub mod rd;
pub mod wr;
pub mod rd_tps;
pub mod wr_tps;

#[derive(Serialize, Deserialize, Clone)]
pub struct Disk {
    pub ratio: i32,  /* usage percent */

    pub rd: i32,  /* kbytes */
    pub wr: i32,
    pub rdtps: i32,
    pub wrtps: i32,
}

impl Disk {
    fn new() -> Disk {
        Disk {
            ratio: 0,
            rd: 0,
            wr: 0,
            rdtps: 0,
            wrtps: 0,
        }
    }
}

//pub struct Meta();

//impl META for Meta {
//    fn argv_new(&self, region: String) -> Vec<String> {
//        vec![
//            "-region".to_owned(),
//            region.to_owned(),
//            "-domain".to_owned(),
//            "ecs.aliyuncs.com".to_owned(),
//            "-apiName".to_owned(),
//            "DescribeDisks".to_owned(),
//            "-apiVersion".to_owned(),
//            "2014-05-26".to_owned(),
//            "Action".to_owned(),
//            "DescribeDisks".to_owned(),
//            "PageSize".to_owned(),
//            "100".to_owned(),
//        ]
//    }
//
//    fn insert(&self, holder: &Arc<Mutex<HashMap<u64, Ecs>>>, data: Vec<u8>) {
//        let v: Value = serde_json::from_slice(&data).unwrap_or(Value::Null);
//        if Value::Null == v {
//            return;
//        }
//
//        let body = &v["Disks"]["Disk"];
//        let mut diskid;
//        let mut device;
//        for i in 0.. {
//            if Value::Null == body[i] {
//                break;
//            } else {
//                if let Value::String(ref ecsid) = body[i]["InstanceId"] {
//                    if let Some(ecs) = holder.lock().unwrap().get_mut(ecsid) {
//                        if let Value::String(ref id) = body[i]["DiskId"] {
//                            diskid= id;
//                        } else {
//                            continue;
//                        }
//
//                        if let Value::String(ref dev) = body[i]["Device"] {
//                            device = dev;
//                        } else {
//                            continue;
//                        }
//
//                        ecs.disk.insert((*device).clone(), (*diskid).clone());
//                    }
//                }
//            }
//        }
//    }
//
//    fn reflect(&self) -> DT {
//        DT::Disk
//    }
//}

pub struct Data;  /* disk ratio */

impl DATA for Data {
    type Holder = Arc<Mutex<HashMap<u64, Ecs>>>;

    fn argv_new(&self, region: String) -> Vec<String> {
        let mut argv = base::argv_new(region);
        argv.push("diskusage_utilization".to_owned());

        argv
    }

    fn insert(&self, holder: &Arc<Mutex<HashMap<u64, Ecs>>>, data: Vec<u8>) {
        let setter = |disk: &mut Disk, v: i32| disk.ratio = v * 10;

        insert(holder, data, setter);
    }
}


fn insert<F: Fn(&mut Disk, i32)>(holder: &Arc<Mutex<HashMap<u64, Ecs>>>, data: Vec<u8>, set: F) {
    let v: Value = serde_json::from_slice(&data).unwrap_or(Value::Null);
    if Value::Null == v {
        return;
    }

    let body = &v["Datapoints"];
    for i in 0.. {
        if Value::Null == body[i] {
            break;
        } else {
            let ecsid;
            let ts;
            let dev;

            if let Value::String(ref id) = body[i]["instanceId"] {
                ecsid = id;
            } else { continue; }

            if let Value::Number(ref t) = body[i]["timestamp"] {
                if let Some(t) = t.as_u64() {
                    ts = t;
                } else { continue; }
            } else { continue; }

            if let Value::String(ref d) = body[i]["device"] {
                dev = d;
            } else { continue; }

            /* align with 15s */
            if let Some(ecs) = holder.lock().unwrap().get_mut(&(ts / MSPERIOD * MSPERIOD)) {
                if let Some(inner) = ecs.data.get_mut(ecsid) {
                    if let Value::Number(ref v) = body[i]["Average"] {
                        if let Some(v) = v.as_u64() {
                            set(inner.disk.entry(dev.to_owned()).or_insert(Disk::new()), v as i32);
                        } else { continue; }
                    } else { continue; }
                } else { continue; }
            }
        }
    }
}
