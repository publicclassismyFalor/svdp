use ::serde_json;
use serde_json::Value;

use std::collections::HashMap;
use std::sync::{Arc, Mutex};

use super::{Ecs, Inner, MSPERIOD, ACSITEM};
use ::sv::aliyun;

pub fn argv_new() -> Vec<[String; 2]> {
    let mut argv = aliyun::argv_new_base();

    argv.push(["Project".to_owned(), ACSITEM.to_owned()]);
    argv.push(["Period".to_owned(), (MSPERIOD / 1000).to_string()]);

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
            if let Some(ecs) = holder.lock().unwrap().get_mut(&(ts / MSPERIOD * MSPERIOD)) {
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
