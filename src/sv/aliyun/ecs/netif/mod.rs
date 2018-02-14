pub mod rd;
pub mod wr;
pub mod rd_tps;
pub mod wr_tps;

use std::sync::{Arc, Mutex};
use std::collections::HashMap;

use ::serde_json;
use serde_json::Value;

use super::Ecs;

#[derive(Serialize, Deserialize)]
pub struct NetIf {
    pub rd: i32,  /* kbytes */
    pub wr: i32,
    pub rdtps: i32,
    pub wrtps: i32,
}

impl NetIf {
    fn new() -> NetIf {
        NetIf {
            rd: 0,
            wr: 0,
            rdtps: 0,
            wrtps: 0,
        }
    }
}

fn insert<F: Fn(&mut NetIf, i32)>(holder: &Arc<Mutex<HashMap<String, Ecs>>>, data: Vec<u8>, set: F) {
    let v: Value = serde_json::from_slice(&data).unwrap_or(Value::Null);
    if Value::Null == v {
        return;
    }

    let body = &v["Datapoints"];
    for i in 0.. {
        if Value::Null == body[i] {
            break;
        } else {
            let mut ecsid;
            let mut ts;
            let mut ip;

            if let Value::String(ref id) = body[i]["instanceId"] {
                ecsid = id;
            } else { continue; }

            if let Value::Number(ref t) = body[i]["timestamp"] {
                if let Some(t) = t.as_u64() {
                    ts = t;
                } else { continue; }
            } else { continue; }

            if let Value::String(ref ipaddr) = body[i]["IP"] {
                ip = ipaddr;
            } else { continue; }

            if let Some(ecs) = holder.lock().unwrap().get_mut(ecsid) {
                /* align with 15s */
                if let Some(inner) = ecs.data.get_mut(&(ts / 15000 * 15000)) {
                    if let Value::Number(ref v) = body[i]["Average"] {
                        if let Some(v) = v.as_u64() {
                            set(inner.netif.entry(ip.to_owned()).or_insert(NetIf::new()), v as i32);
                        } else { continue; }
                    } else { continue; }
                } else { continue; }
            }
        }
    }
}
