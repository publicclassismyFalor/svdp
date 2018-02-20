use ::serde_json;
use serde_json::Value;

use std::collections::HashMap;
use std::sync::{Arc, Mutex};

use super::{Ecs, Inner};
use ::sv::aliyun;

pub fn argv_new(region: String) -> Vec<String> {
    let mut argv = aliyun::argv_new_base(region);

    argv.push("Period".to_owned());
    argv.push("15".to_owned());
    argv.push("Metric".to_owned());

    argv
}

pub fn insert<F: Fn(&mut Inner, f64)>(holder: &Arc<Mutex<HashMap<u64, Ecs>>>, data: Vec<u8>, set: F) {
    let v: Value = serde_json::from_slice(&data).unwrap_or(Value::Null);
    if Value::Null == v {
        return;
    }

    let body = &v["Datapoints"];
    for i in 0.. {
        if Value::Null == body[i] {
            break;
        } else {
            let ts;
            let ecsid;

            if let Value::Number(ref t) = body[i]["timestamp"] {
                if let Some(t) = t.as_u64() {
                    ts = t;
                } else { continue; }
            } else { continue; }

            if let Value::String(ref id) = body[i]["instanceId"] {
                ecsid = id;
            } else { continue; }

            /* align with 15s */
            if let Some(ecs) = holder.lock().unwrap().get_mut(&(ts / 15000 * 15000)) {
                if let Some(mut inner) = ecs.data.get_mut(ecsid) {
                    if let Value::Number(ref v) = body[i]["Average"] {
                        if let Some(v) = v.as_f64() {
                            set(&mut inner, v);
                        } else { continue; }
                    } else { continue; }
                } else { continue; }
            }
        }
    }
}
