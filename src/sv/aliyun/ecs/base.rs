use ::serde_json;
use serde_json::Value;

use std::collections::HashMap;

use std::thread;
use std::sync::{mpsc, Arc, Mutex};

use super::{Ecs, Inner};
use super::super::{DATA, cmd_exec};

pub fn argv_new(region: String) -> Vec<String> {
    vec![
        "-region".to_owned(),
        region,
        "-domain".to_owned(),
        "metrics.aliyuncs.com".to_owned(),
        "-apiName".to_owned(),
        "QueryMetricList".to_owned(),
        "-apiVersion".to_owned(),
        "2017-03-01".to_owned(),
        "Action".to_owned(),
        "QueryMetricList".to_owned(),
        "Project".to_owned(),
        "acs_ecs_dashboard".to_owned(),
        "Period".to_owned(),
        "15".to_owned(),
        "Length".to_owned(),
        "1000".to_owned(),
        "Metric".to_owned(),
    ]
}

pub fn get<T: DATA>(holder: <T as DATA>::Holder, region: String, me: T) {
    let mut extra = me.argv_new(region);

    let (tx, rx) = mpsc::channel();

    thread::spawn(move || {
        if let Ok(ret) = cmd_exec(extra.clone()) {
            let v = serde_json::from_slice(&ret).unwrap_or(Value::Null);
            if Value::Null == v {
                return;
            }

            tx.send(ret).unwrap();

            if let Value::String(ref cursor) = v["Cursor"] {
                extra.push("Cursor".to_owned());
                extra.push((*cursor).clone());

                while let Ok(ret) = cmd_exec(extra.clone()) {
                    let v = serde_json::from_slice(&ret).unwrap_or(Value::Null);
                    if Value::Null == v {
                        return;
                    }

                    tx.send(ret).unwrap();

                    if let Value::String(ref cursor) = v["Cursor"] {
                        extra.pop();
                        extra.push((*cursor).clone());
                    } else {
                        break;
                    }
                }
            }
        }
    });

    for r in rx {
        me.insert(&holder, r);
    }
}

pub fn insert<F: Fn(&mut Inner, f64)>(holder: &Arc<Mutex<HashMap<String, Ecs>>>, data: Vec<u8>, set: F) {
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

            if let Value::String(ref id) = body[i]["instanceId"] {
                ecsid = id;
            } else { continue; }

            if let Value::Number(ref t) = body[i]["timestamp"] {
                if let Some(t) = t.as_u64() {
                    ts = t;
                } else { continue; }
            } else { continue; }

            if let Some(ecs) = holder.lock().unwrap().get_mut(ecsid) {
                /* align with 15s */
                if let Some(mut inner) = ecs.data.get_mut(&(ts / 15000 * 15000)) {
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
