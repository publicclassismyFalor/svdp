use std::io::{Read, Write};
use std::io::{Error, ErrorKind};
use std::net::{TcpStream, TcpListener};

use std::time::Duration;

/* async http serv */
use iron::prelude::*;
use iron::status;

use threadpool::ThreadPool;

use ::{CONF, DBPOOL};
use ::serde_json;
use ::sv::aliyun;
use super::CACHEINTERVAL;


/// REQ example:
/// {"method":"sv_ecs","params":{"item":["disk","/dev/vda1","rdtps"],"instance_id":"i-77777","ts_range":[15000000,1600000],"interval":600},"id":0}
///
/// RES example:
/// {"result":[[1519530310,10],...,[1519530390,20]],"id":0}
/// OR
/// {"err":"...","id":0}
#[derive(Serialize, Deserialize, Clone)]
pub struct Req {
    method: String,
    pub params: Params,
    pub id: i32,
}

#[derive(Serialize, Deserialize, Clone)]
pub struct Params {
    pub item: (String, Option<String>, Option<String>),
    pub instance_id: String,
    pub ts_range: [i32; 2],
    pub interval: Option<i32>,
}

/****************
 * http service *
 ****************/
pub fn http_serv() {
    let addr = CONF.sv_http_addr.clone();
    Iron::new(http_ops).http(&addr.unwrap())
        .unwrap_or_else(|e|{ errexit!(e); });
}

fn http_ops(request: &mut Request) -> IronResult<Response> {
    let mut buf = Vec::new();
    request
        .body
        .read_to_end(&mut buf)
        .map_err(|e| IronError::new(e, (status::InternalServerError, "request reading err")))?;

    match worker(&buf) {
        Ok((res, id)) => {
            return Ok(
                Response::with(
                    (status::Ok, format!("{}\"result\":{},\"id\":{}{}" , "{", res, id, "}").as_bytes())
                    )
                );
        },
        Err((e, id)) => {
            return Err(
                IronError::new(
                    Error::from(ErrorKind::Other),
                    (status::NotFound, format!("{}\"err\":{},\"id\":{}{}" , "{", e, id, "}").as_str())
                    )
                );
        }
    }
}

/*******************
 * raw tcp service *
 *******************/
pub fn tcp_serv() {
    let tdpool = ThreadPool::new(::num_cpus::get());

    let addr = CONF.sv_tcp_addr.clone();
    let listener = TcpListener::bind(&addr.unwrap())
        .unwrap_or_else(|e|{ errexit!(e); });

    loop {
        match listener.accept() {
            Ok((socket, _peeraddr)) => {
                tdpool.execute(move|| {
                    socket.set_read_timeout(Some(Duration::from_secs(3))).unwrap();
                    tcp_ops(socket);
                });
            },
            Err(e) => err!(e)
        }
    }
}

fn tcp_ops(mut socket: TcpStream) {
    let mut buf: Vec<u8> = Vec::new();
    if let Err(e) = socket.read_to_end(&mut buf) {
        let errmsg = "{\"err\":\"socket read err\",\"id\":-1}";

        if let Err(ee) = socket.write(errmsg.as_bytes()) {
            err!(ee)
        }

        err!(e);
        return;
    }

    match worker(&buf) {
        Ok((res, id)) => {
            if let Err(e) = socket.write(format!("{}\"result\":{},\"id\":{}{}" , "{", res, id, "}").as_bytes()) {
                err!(e);
            }
        },
        Err((e, id)) => {
            if let Err(ee) = socket.write(format!("{}\"err\":{},\"id\":{}{}" , "{", e, id, "}").as_bytes()) {
                err!(ee);
            }
        }
    }
}

/**************************************
 * common worker for http and raw tcp *
 **************************************/
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
                    $final_v.push($cb(&v, $dev, $item));
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
                    if CACHEINTERVAL as i32 == itv {
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
            match DBPOOL.clone().get() {
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
                if let Ok(mut r) = serde_json::from_str::<Vec<(i32, Option<i32>)>>(&orig) {
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
    ($res: expr, $reqid: expr) => {
        Ok((serde_json::to_string(&$res).unwrap(), $reqid))
    }
}

macro_rules! go {
    ($req: expr, $deque: expr, $get_cb: expr) => {
        {
            let reqid = $req.id;
            match $deque.read().unwrap().get(0) {
                None => {
                    let tuple = db_worker!($req);
                    return res!(tuple, reqid);
                },
                Some(dq) => {
                    if dq.0 > $req.params.ts_range[1]{
                        let tuple = db_worker!($req);
                        return res!(tuple, reqid);
                    } else if dq.0 < ($req.params.ts_range[0] + super::CACHEINTERVAL as i32) {
                        let tuple = cache_worker!($req, $get_cb, $deque);
                        return res!(tuple, reqid);
                    } else {
                        let mut req_db = $req.clone();
                        req_db.params.ts_range[1] = dq.0 - super::CACHEINTERVAL as i32;
                        let mut db_res = db_worker!(req_db);

                        let mut cache_res = cache_worker!($req, $get_cb, $deque);

                        let res;
                        if 0 < db_res.0.len() {
                            res = serde_json::to_string(&(
                                    db_res.0.append(&mut cache_res.0),
                                    db_res.1.append(&mut cache_res.1)
                                    )).unwrap();
                        } else {
                            res = serde_json::to_string(&cache_res).unwrap();
                        }

                        return Ok((res, reqid));
                    }
                }
            }
        }
    }
}

fn worker(body: &Vec<u8>) -> Result<(String, i32), (String, i32)> {
    let mut req: Req;
    match serde_json::from_slice(body) {
        Ok(r) => req = r,
        Err(e) => {
            err!(e);
            return Err(("json parse err".to_owned(), -1));
        }
    }

    match req.params.interval {
        Some(itv) => {
            let cache_itv = CACHEINTERVAL as i32;
            if cache_itv > itv {
                req.params.interval = Some(cache_itv);  // 低于 300s，自动提升为 300s
            } else {
                req.params.interval = Some(itv / cache_itv * cache_itv);  // 其余情况，按 300s 对齐
            }
        },
        None => {
            req.params.interval = Some(CACHEINTERVAL as i32);  // 不指定，默认 300s
        }
    }

    match req.method.as_str() {
        sv if sv.starts_with("sv_") => {
            match sv {
                "sv_ecs" => go!(req, aliyun::CACHE_ECS, aliyun::ecs::Inner::get_cb),
                "sv_slb" => go!(req, aliyun::CACHE_SLB, aliyun::slb::Inner::get_cb),
                "sv_rds" => go!(req, aliyun::CACHE_RDS, aliyun::rds::Inner::get_cb),
                "sv_mongodb" => go!(req, aliyun::CACHE_MONGODB, aliyun::mongodb::Inner::get_cb),
                "sv_redis" => go!(req, aliyun::CACHE_REDIS, aliyun::redis::Inner::get_cb),
                "sv_memcache" => go!(req, aliyun::CACHE_MEMCACHE, aliyun::memcache::Inner::get_cb),
                _ => {
                    err!(req.method);
                    return Err(("method invalid".to_owned(), req.id));
                }
            }
        },
        dp if dp.starts_with("dp_") => {
            match dp {
                _ => {
                    return Ok(("".to_owned(), 0));
                }
            }
        },
        _ => {
            err!(req.method);
            return Err(("method invalid".to_owned(), req.id));
        }
    }
}
