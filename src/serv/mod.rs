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


/// REQ example:
/// {"method":"sv_ecs","params":{"item":["disk","/dev/vda1","rdtps"],"instance_id":"i-77777","ts_range":[15000000,1600000],"interval":600},"id":0}
///
/// RES example:
/// {"result":[[1519530310,10],...,[1519530390,20]],"id":0}
/// OR
/// {"err":"...","id":0}
#[derive(Serialize, Deserialize)]
struct Req {
    method: String,
    params: Params,
    id: i32,
}

#[derive(Serialize, Deserialize)]
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
fn worker(body: &Vec<u8>) -> Result<(String, i32), String> {
    let req: Req;
    match serde_json::from_slice(body) {
        Ok(r) => req = r,
        Err(e) => {
            err!(e);
            return Err("{\"err\":\"json parse err\",\"id\":-1}".to_owned());
        }
    }

    /*
     * cache 能够满足用户请求的
     * 时间区间与间隔要求的情况，
     * 直接从缓存中提取，不查询 DB
     */
    // if 9999999999 <= req.params.ts_range[0]
    //     && None != req.params.interval
    //     && 5 * 60 <= req.params.interval.unwrap() {
    //         return cache_worker(req);
    // }

    let pgconn;
    match DBPOOL.clone().get() {
        Ok(conn) => pgconn = conn,
        Err(e) => {
            err!(e);
            return Err(format!("{}\"err\":\"db_conn_pool busy\",\"id\":{}{}", "{", req.id, "}"));
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
            err!("invalid item");
            return Err(format!("{}\"err\":\"invalid item\",\"id\":{}{}", "{", req.id, "}"));
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

    let qrow;
    match pgconn.query(querysql.as_str(), &[]) {
        Ok(q) => {
            if q.is_empty() {
                return Ok(("[]".to_owned(), req.id));
            } else {
                qrow = q;
            }
        },
        Err(e) => {
            err!(e);
            return Err(format!("{}\"err\":\"db query err\",\"id\":{}{}", "{", req.id, "}"));
        }
    }

    let qres = qrow.get(0);
    let res: String;
    if let Some(orig) = qres.get(0) {
        let orig: String = orig;
        let mut finalres = (vec![], vec![]);
        if let Ok(mut r) = serde_json::from_str::<Vec<(i64, Option<i32>)>>(&orig) {
            r.sort_by(|a, b|a.0.cmp(&b.0));
            let len = r.len();
            for i in 0..len {
                if let Some(v) = r[i].1 {
                    // ** 需要 strftime 形式的时间？ **
                    // use ::time::{Timespec, strftime, at};
                    // finalres.0.push(strftime("%m-%d %H:%M:%S", &at(Timespec::new(r[i].0, 0))).unwrap_or("".to_owned()));
                    finalres.0.push(r[i].0);
                    finalres.1.push(v);
                }
            }
        } else {
            err!("server err");
            return Err(format!("{}\"err\":\"server err\",\"id\":{}{}", "{", req.id, "}"));
        }

        res = serde_json::to_string(&finalres).unwrap();
    } else {
        err!("server db err");
        return Err(format!("{}\"err\":\"server db err\",\"id\":{}{}", "{", req.id, "}"));
    }

    Ok((res, req.id))
}

// TODO
// fn cache_worker(req: Req) -> Result<(String, i32), String> {
//     Ok(("".to_owned(), 0))
// }
