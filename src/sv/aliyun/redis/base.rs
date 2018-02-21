use ::serde_json;
use serde_json::Value;

use std::collections::HashMap;
use std::sync::{Arc, Mutex};

use super::{Redis, Inner, MSPERIOD};
use ::sv::aliyun;

pub fn argv_new(region: String) -> Vec<String> {
    let mut argv = aliyun::argv_new_base(region);

    argv.push("Project".to_owned());
    argv.push("acs_redis".to_owned());  //
    argv.push("Period".to_owned());
    argv.push((MSPERIOD / 1000).to_string());
    argv.push("Metric".to_owned());

    argv
}

pub fn insert<F: Fn(&mut Inner, f64)>(holder: &Arc<Mutex<HashMap<u64, Redis>>>, data: Vec<u8>, set: F) {
    let v: Value = serde_json::from_slice(&data).unwrap_or(Value::Null);
    if Value::Null == v {
        return;
    }

    let body = &v["Datapoints"];
    for i in 0.. {
        if Value::Null == body[i] {
            break;
        } else {
            let redisid;
            if let Value::String(ref id) = body[i]["instanceId"] {
                redisid = id;
            } else { continue; }

            let ts;
            if let Value::Number(ref t) = body[i]["timestamp"] {
                if let Some(t) = t.as_u64() {
                    ts = t;
                } else { continue; }
            } else { continue; }

            /* align with 60s */
            if let Some(redis) = holder.lock().unwrap().get_mut(&(ts / MSPERIOD * MSPERIOD)) {
                if let Value::Number(ref v) = body[i]["Average"] {
                    if let Some(v) = v.as_f64() {  //
                        set(redis.data.entry(redisid.to_owned()).or_insert(Inner::new()), v);
                    } else { continue; }
                } else { continue; }
            }
        }
    }
}
