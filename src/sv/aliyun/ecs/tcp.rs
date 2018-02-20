use std::sync::{Arc, Mutex};
use std::collections::HashMap;

use ::serde_json;
use serde_json::Value;

use super::base;
use super::{MSPERIOD, Ecs};
use super::super::DATA;

pub struct Data;

impl DATA for Data {
    type Holder = Arc<Mutex<HashMap<u64, Ecs>>>;

    fn argv_new(&self, region: String) -> Vec<String> {
        let mut argv = base::argv_new(region);
        argv.push("net_tcpconnection".to_owned());

        argv
    }

    fn insert(&self, holder: &Arc<Mutex<HashMap<u64, Ecs>>>, data: Vec<u8>) {
        let v: Value = serde_json::from_slice(&data).unwrap_or(Value::Null);
        if Value::Null == v {
            return;
        }

        let body = &v["Datapoints"];
        for i in 0.. {
            if Value::Null == body[i] {
                break;
            } else {
                /* take TCP_TOTAL only! */
                if let Value::String(ref s) = body[i]["state"] {
                    if "TCP_TOTAL" != s { continue; }
                } else { continue; }

                let ecsid;
                let ts;

                if let Value::String(ref id) = body[i]["instanceId"] {
                    ecsid = id;
                } else { continue; }

                if let Value::Number(ref t) = body[i]["timestamp"] {
                    if let Some(t) = t.as_u64() {
                        ts = t;
                    } else { continue; }
                } else { continue; }

                /* align with 15s */
                if let Some(ecs) = holder.lock().unwrap().get_mut(&(ts / MSPERIOD * MSPERIOD)) {
                    if let Some(inner) = ecs.data.get_mut(ecsid) {
                        if let Value::Number(ref v) = body[i]["Average"] {
                            if let Some(v) = v.as_u64() {
                                inner.tcp = v as i32;
                            } else { continue; }
                        } else { continue; }
                    } else { continue; }
                }
            }
        }
    }
}
