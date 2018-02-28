use std::io::{Read, Write};
use std::io::{Error, ErrorKind};
use std::net::{TcpStream, TcpListener};

use std::time::Duration;
use ::time::{Timespec, strftime, at};

/* async http serv */
use iron::prelude::*;
use iron::status;

use threadpool::ThreadPool;

use ::{CONF, DBPOOL};
use ::serde_json;
use ::sv::aliyun;


/// REQ example:
/// {"method":"sv_ecs","params":{"item":["disk","/dev/vda1","rdtps"],"instance_id":"i-77777","ts_range":[15000000,1600000],"interval":600},"id":0}
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
            return Ok( Response::with( (status::Ok, format!("{}\"result\":{},\"id\":{}{}" , "{", res, id, "}").as_bytes()) ) );
        },
        Err(e) => {
            return Err(IronError::new(Error::from(ErrorKind::Other), (status::NotFound, e)));
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
        socket.write(errmsg.as_bytes()).unwrap_or_default();

        err!(e);
        return;
    }

    match worker(&buf) {
        Ok((res, id)) => {
            //let res = res.replace("\": ", "\":");
            if let Err(e) = socket.write(format!("{}\"result\":{},\"id\":{}{}" , "{", res, id, "}").as_bytes()) {
                err!(e);
            }
        },
        Err(e) => {
            socket.write(e.as_bytes()).unwrap_or_default();
        }
    }
}

/**************************************
 * common worker for http and raw tcp *
 **************************************/
macro_rules! get_tuple {
    ($req: expr, $myworker: expr) => {
        {
            let reqid = $req.id;

            let res;
            match $myworker($req) {
                Ok(r) => res = r,
                Err(e) => return Err(format!("{}\"err\":\"{}\",\"id\":{}{}", "{", e, reqid, "}"))
            }

            res
        }
    }
}

macro_rules! res {
    ($res: expr, $reqid: expr) => {
        Ok((serde_json::to_string(&($res.0, $res.1)).unwrap(), $reqid))
    }
}

macro_rules! worker {
    ($req: expr, $queue: expr) => {
        {
            let reqid = $req.id;
            match $queue.read().unwrap().get(0) {
                None => {
                    let vectp = get_tuple!($req, db_worker);
                    return res!(vectp, reqid);
                },

                Some(vecdq) if vecdq.0 > $req.params.ts_range[1] => {
                    let vectp = get_tuple!($req, db_worker);
                    return res!(vectp, reqid);
                },

                Some(vecdq) if vecdq.0 < ($req.params.ts_range[0] + super::CACHEINTERVAL as i32)=> {
                    let vectp = get_tuple!($req, cache_worker);
                    return res!(vectp, reqid);
                },

                Some(vecdq) => {
                    /*
                     * first, split params' ts_range;
                     * then, get data from db;
                     * last, get data from cache.
                     **/
                    let mut req_db = $req.clone();
                    req_db.params.ts_range[1] = vecdq.0 - super::CACHEINTERVAL as i32;
                    let mut dbtp = get_tuple!(req_db, db_worker);

                    let mut cachetp = get_tuple!($req, cache_worker);

                    let res = serde_json::to_string(
                            &(dbtp.0.append(&mut cachetp.0), dbtp.1.append(&mut cachetp.1))
                        ).unwrap();

                    return Ok((res, reqid));
                }
            }
        }
    }
}

fn worker(body: &Vec<u8>) -> Result<(String, i32), String> {
    let req: Req;
    match serde_json::from_slice(body) {
        Ok(r) => req = r,
        Err(e) => {
            err!(e);
            return Err(r#"{"err":"json parse err","id":-1}"#.to_owned());
        }
    }

    match req.method.as_str() {
        "sv_ecs" => worker!(req, aliyun::CACHE_ECS),
        "sv_slb" => worker!(req, aliyun::CACHE_SLB),
        "sv_rds" => worker!(req, aliyun::CACHE_RDS),
        "sv_mongodb" => worker!(req, aliyun::CACHE_MONGODB),
        "sv_redis" => worker!(req, aliyun::CACHE_REDIS),
        "sv_memcache" => worker!(req, aliyun::CACHE_MEMCACHE),
        _ => unreachable!()
    }
}

// get data from memory cache
fn cache_worker(req: Req) -> Result<(Vec<String>, Vec<i32>), String> {
    Ok((vec![], vec![]))
}

// get data from postgres
fn db_worker(req: Req) -> Result<(Vec<String>, Vec<i32>), String> {
    let pgconn;
    match DBPOOL.clone().get() {
        Ok(conn) => pgconn = conn,
        Err(e) => {
            err!(e);
            return Err("db_conn_pool busy".to_owned());
        }
    }

    let queryfilter;
    match req.params.item {
        (item, None, None) => {
            queryfilter = format!("'{}{},{}{}'", "{", req.params.instance_id, item, "}");
        },
        (submethod, Some(dev), Some(item))=> {
            queryfilter = format!("'{}{},{},{},{}{}'", "{", req.params.instance_id, submethod, dev, item, "}");
        },
        _ => {
            err!("");
            return Err("invalid item".to_owned());
        }
    }

    let itvfilter;
    if let Some(itv) = req.params.interval {
        itvfilter = format!("AND (ts % {}) = 0", itv);
    } else {
        itvfilter = "".to_owned();
    }

    let querysql = format!("SELECT array_to_json(array_agg(json_build_array(ts, sv#>{})))::text FROM {} WHERE ts >= {} AND ts <= {} {}",
                           queryfilter, req.method, req.params.ts_range[0], req.params.ts_range[1], itvfilter);

    let rows;
    match pgconn.query(querysql.as_str(), &[]) {
        Ok(q) => {
            if q.is_empty() {
                return Ok((vec![], vec![]));
            } else {
                rows = q;
            }
        },
        Err(e) => {
            err!(e);
            return Err("db query err".to_owned());
        }
    }

    let mut final_k = vec![];
    let mut final_v = vec![];

    let row = rows.get(0);
    if let Some(orig) = row.get(0) {
        let orig: String = orig;
        if let Ok(mut r) = serde_json::from_str::<Vec<(i64, Option<i32>)>>(&orig) {
            r.sort_by(|a, b|a.0.cmp(&b.0));
            let len = r.len();
            for i in 0..len {
                if let Some(v) = r[i].1 {
                    //final_k.push(r[i].0);
                    final_k.push(
                        strftime("%m-%d %H:%M:%S", &at(Timespec::new(r[i].0, 0)))
                        .unwrap_or("".to_owned())
                        );
                    final_v.push(v);
                }
            }
        } else {
            err!("");
            return Err("server err".to_owned());
        }
    } else {
        err!("");
        return Err("server db err".to_owned());
    }

    Ok((final_k, final_v))
}
