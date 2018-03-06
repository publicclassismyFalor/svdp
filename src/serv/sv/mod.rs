use ::sv::aliyun;

/// REQ example:
/// {"method":"sv_ecs","params":{"item":["disk","/dev/vda1","rdtps"],"instance_id":"i-77777","ts_range":[15000000,1600000],"interval":600,"algo":["sum","avg","max","min"]},"id":0}
///
/// RES example:
/// {"result":[[1519530310,10],...,[1519530390,20]],"id":0}
/// OR
/// {"err":"...","id":0}
#[derive(Serialize, Deserialize, Clone)]
struct Req {
    method: String,
    params: Params,
    id: i32,
}

#[derive(Serialize, Deserialize, Clone)]
struct Params {
    item: (String, Option<String>, Option<String>),
    instance_id: String,
    ts_range: [i32; 2],
    interval: Option<i32>,
    algo: Option<Vec<String>>,
}

macro_rules! cache_actor {
    ($final_k: expr, $final_v: expr,
        $req: expr, $cb: expr, $deque: expr,
        $condition: expr,
        $dev: expr, $item: expr) => {

        for x in $deque.read().unwrap().iter() {
            if x.0 > $req.params.ts_range[1] {
                break;
            }

            if x.0 >= $req.params.ts_range[0] && 0 == x.0 % $condition {
                if let Some(v) = x.1.get(&$req.params.instance_id) {
                    $final_k.push(x.0);
                    $final_v.push($cb(&v, $dev, $item) as i64);
                }
            }
        }
    }
}

macro_rules! cache_worker {
    ($req: expr, $get_cb: expr, $deque: expr) => {
        loop {
            let mut final_k = vec![];
            let mut final_v = vec![];

            let (has_itv, itv) = match $req.params.interval {
                Some(itv) => {
                    if aliyun::CACHEINTERVAL as i32 == itv {
                        (false, 0)
                    } else {
                        (true, itv)
                    }
                },
                None => unreachable!()
            };

            match $req.params.item {
                (item, None, None) => {
                    if let Some(handler) = $get_cb(&item) {
                        if has_itv {
                            cache_actor!(final_k, final_v, $req, handler, $deque, itv, "", "");
                        } else {
                            cache_actor!(final_k, final_v, $req, handler, $deque, 1, "", "");
                        }
                    } else {
                        err!("0");
                        return Err(("params invalid".to_owned(), $req.id));
                    }
                },
                (submethod, Some(dev), Some(item)) => {
                    if let Some(handler) = $get_cb(&submethod) {
                        if has_itv {
                            cache_actor!(final_k, final_v, $req, handler, $deque, itv, &dev, &item);
                        } else {
                            cache_actor!(final_k, final_v, $req, handler, $deque, 1, &dev, &item);
                        }
                    } else {
                        err!("10");
                        return Err(("params invalid".to_owned(), $req.id));
                    }
                },
                _ => {
                    err!("");
                    return Err(("params invalid".to_owned(), $req.id));
                }
            }

            /* return final tuple */
            break (final_k, final_v);
        }
    }
}

macro_rules! db_worker {
    ($req: expr) => {
        loop {
            let mut final_k = vec![];
            let mut final_v = vec![];

            if $req.params.ts_range[0] > $req.params.ts_range[1] {
                break (final_k, final_v);
            }

            let pgconn;
            match ::DBPOOL.clone().get() {
                Ok(conn) => pgconn = conn,
                Err(e) => {
                    err!(e);
                    return Err(("db_conn_pool busy".to_owned(), $req.id));
                }
            }

            let queryfilter;
            match $req.params.item {
                (item, None, None) => {
                    queryfilter = format!("'{}{},{}{}'", "{", $req.params.instance_id, item, "}");
                },
                (submethod, Some(dev), Some(item))=> {
                    queryfilter = format!("'{}{},{},{},{}{}'", "{", $req.params.instance_id, submethod, dev, item, "}");
                },
                _ => {
                    err!("20");
                    return Err(("invalid item".to_owned(), $req.id));
                }
            }

            let itvfilter;
            if let Some(itv) = $req.params.interval {
                itvfilter = format!("AND (ts % {}) = 0", itv);
            } else {
                itvfilter = "".to_owned();
            }

            let querysql = format!("SELECT array_to_json(array_agg(json_build_array(ts, sv#>{})))::text FROM {} WHERE ts >= {} AND ts <= {} {}",
                                   queryfilter, $req.method, $req.params.ts_range[0], $req.params.ts_range[1], itvfilter);

            let rows;
            match pgconn.query(querysql.as_str(), &[]) {
                Ok(q) => {
                    if q.is_empty() {
                        err!("30 empty res");
                        break (final_k, final_v);
                    } else {
                        rows = q;
                    }
                },
                Err(e) => {
                    err!(e);
                    return Err(("db query err".to_owned(), $req.id));
                }
            }

            let row = rows.get(0);
            if let Some(orig) = row.get(0) {
                let orig: String = orig;
                if let Ok(mut r) = ::serde_json::from_str::<Vec<(i32, Option<i64>)>>(&orig) {
                    r.sort_by(|a, b|a.0.cmp(&b.0));
                    let len = r.len();
                    for i in 0..len {
                        if let Some(v) = r[i].1 {
                            final_k.push(r[i].0);
                            final_v.push(v);
                        }
                    }
                } else {
                    err!("40");
                    return Err(("server err".to_owned(), $req.id));
                }
            } else {
                err!("50 empty res");
            }

            break (final_k, final_v);
        }
    }
}
macro_rules! res {
    ($req: expr, $res: expr, $reqid: expr) => {
        if None == $req.params.algo {
            Ok((::serde_json::to_string(&$res).unwrap(), $reqid))
        } else {
            let mut algores = (vec![], vec![]);
            for x in $req.params.algo.unwrap() {
                match x.as_str() {
                    "sum" => {
                        algores.0.push("sum");
                        algores.1.push($res.1.iter().sum::<i64>());
                    },
                    "avg" => {
                        algores.0.push("avg");
                        if 0 == $res.1.len() {
                            algores.1.push(-1);
                        } else {
                            algores.1.push($res.1.iter().sum::<i64>() / $res.1.len() as i64);
                        }
                    },
                    "min" => {
                        algores.0.push("min");
                        algores.1.push(*$res.1.iter().min().unwrap_or(&-1));
                    },
                    "max" => {
                        algores.0.push("max");
                        algores.1.push(*$res.1.iter().max().unwrap_or(&-1));
                    },
                    _ => {
                        err!("60");
                        return Err(("algo invalid".to_owned(), $req.id));
                    }
                }
            }

            Ok((::serde_json::to_string(&algores).unwrap(), $reqid))
        }
    }
}

macro_rules! go {
    ($req: expr, $deque: expr, $get_cb: expr) => {
        {
            let reqid = $req.id;
            match $deque.read().unwrap().get(0) {
                None => {
                    let tuple = db_worker!($req);
                    return res!($req, tuple, reqid);
                },
                Some(dq) => {
                    if dq.0 > $req.params.ts_range[1]{
                        let tuple = db_worker!($req);
                        return res!($req, tuple, reqid);
                    } else if dq.0 < ($req.params.ts_range[0] + aliyun::CACHEINTERVAL as i32) {
                        let tuple = cache_worker!($req, $get_cb, $deque);
                        return res!($req, tuple, reqid);
                    } else {
                        let mut req_db = $req.clone();
                        req_db.params.ts_range[1] = dq.0 - aliyun::CACHEINTERVAL as i32;
                        let mut db_res = db_worker!(req_db);

                        let mut cache_res = cache_worker!($req, $get_cb, $deque);

                        let res;
                        if 0 == db_res.0.len() {
                            res = cache_res;
                        } else if 0 == cache_res.0.len() {
                            res = db_res;
                        } else {
                            db_res.0.append(&mut cache_res.0);
                            db_res.1.append(&mut cache_res.1);
                            res = db_res;
                        }

                        return res!($req, res, reqid);
                    }
                }
            }
        }
    }
}

pub fn worker(body: &str) -> Result<(String, i32), (String, i32)> {
    let mut req: Req;
    match ::serde_json::from_str(body) {
        Ok(r) => req = r,
        Err(e) => {
            err!(e);
            return Err(("json parse err".to_owned(), -1));
        }
    }

    match req.params.interval {
        Some(itv) => {
            let cache_itv = aliyun::CACHEINTERVAL as i32;
            if cache_itv > itv {
                req.params.interval = Some(cache_itv);  // 低于 300s，自动提升为 300s
            } else {
                req.params.interval = Some(itv / cache_itv * cache_itv);  // 其余情况，按 300s 对齐
            }
        },
        None => {
            req.params.interval = Some(aliyun::CACHEINTERVAL as i32);  // 不指定，默认 300s
        }
    }

    match &req.method.as_bytes()[3..] {
        b"ecs" => go!(req, aliyun::CACHE_ECS, aliyun::ecs::Inner::get_cb),
        b"slb" => go!(req, aliyun::CACHE_SLB, aliyun::slb::Inner::get_cb),
        b"rds" => go!(req, aliyun::CACHE_RDS, aliyun::rds::Inner::get_cb),
        b"mongodb" => go!(req, aliyun::CACHE_MONGODB, aliyun::mongodb::Inner::get_cb),
        b"redis" => go!(req, aliyun::CACHE_REDIS, aliyun::redis::Inner::get_cb),
        b"memcache" => go!(req, aliyun::CACHE_MEMCACHE, aliyun::memcache::Inner::get_cb),
        _ => {
            err!(req.method.as_str());
            return Err(("method invalid".to_owned(), req.id));
        }
    };
}
