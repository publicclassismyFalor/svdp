#[macro_use] extern crate lazy_static;
extern crate serde;
extern crate serde_json;
#[macro_use] extern crate serde_derive;
extern crate r2d2;
extern crate r2d2_postgres;
extern crate postgres;
extern crate toml;
extern crate threadpool;
extern crate iron;
extern crate num_cpus;

#[macro_use] mod zmacro;
mod sv;
mod dp;

use std::thread;
use std::fs::File;
use std::io::{Read, Write};
use std::io::{Error, ErrorKind};
use std::net::{TcpStream, TcpListener};

/* async http serv */
use iron::prelude::*;
use iron::status;

use threadpool::ThreadPool;
use r2d2::Pool;
use r2d2_postgres::{TlsMode, PostgresConnectionManager};

#[derive(Deserialize)]
pub struct Config {
    pg_login_url: String,  // UNIX DOMAIN SOCKET: "postgres://jack@%2Fhome%2Fjack/svdp"
    sv_serv_addr: String,  // "[::1]:30000"
}

lazy_static! {
    pub static ref CONF: Config = conf_parse();
}

lazy_static! {
    pub static ref DBPOOL: Pool<PostgresConnectionManager> = {
        let pgmg = PostgresConnectionManager::new(CONF.pg_login_url.as_str(), TlsMode::None)
            .unwrap_or_else(|e|{ errexit!(e); });

        r2d2::Pool::builder()
            .max_size((::num_cpus::get() * 2) as u32)
            .build(pgmg)
            .unwrap_or_else(|e|{ errexit!(e); })
    };
}

/* parse config file */
fn conf_parse() -> Config {
    let mut content = String::new();

    File::open("major.toml")
        .unwrap_or_else(|e|{ errexit!(e); })
        .read_to_string(&mut content)
        .unwrap_or_else(|e|{ errexit!(e); });

    toml::from_str::<Config>(&content)
        .unwrap_or_else(|e|{ errexit!(e); })
}

pub fn run() {
    thread::spawn(|| http_serv());
    thread::spawn(|| tcp_serv());

    sv::go();
    dp::go();
}


/// REQ example:
/// {"method":"sv_ecs","params":{"instance_id":"i-123456","ts_range":[15000000,1600000]},"id":0}
///
/// RES example:
/// {"result":["ts":1519379068,"data":{...}],"id":0}
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
    instance_id: Option<String>,
    ts_range: [i32; 2],
}

/****************
 * http service *
 ****************/
fn http_serv() {
    Iron::new(http_ops).http(&CONF.sv_serv_addr)
        .unwrap_or_else(|e|{ errexit!(e); });
}

fn http_ops(request: &mut iron::Request) -> IronResult<Response> {
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
            return Err(iron::IronError::new(Error::from(ErrorKind::Other), (status::InternalServerError, e)));
        }
    }
}

/*******************
 * raw tcp service *
 *******************/
fn tcp_serv() {
    let tdpool = ThreadPool::new(::num_cpus::get());

    let listener = TcpListener::bind(&CONF.sv_serv_addr)
        .unwrap_or_else(|e|{ errexit!(e); });

    loop {
        match listener.accept() {
            Ok((socket, _peeraddr)) => {
                tdpool.execute(move|| {
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

    let pgconn;
    match DBPOOL.clone().get() {
        Ok(conn) => pgconn = conn,
        Err(e) => {
            err!(e);
            return Err(format!("{}\"err\":\"db_conn_pool busy\",\"id\":{}{}", "{", req.id, "}"));
        }
    }

    let querysql;
    match req.params.instance_id {
        None => {
            querysql = format!("SELECT array_to_json(array_agg(row_to_json(d)))::text FROM
                               (SELECT ts, sv FROM {} WHERE ts >= {} AND ts <= {}) d", req.method, req.params.ts_range[0], req.params.ts_range[1]);
        },
        Some(insid) => {
            querysql = format!("SELECT array_to_json(array_agg(row_to_json(d)))::text FROM
                               (SELECT ts, sv->'{}' AS sv FROM {} WHERE ts >= {} AND ts <= {}) d",
                               insid, req.method, req.params.ts_range[0], req.params.ts_range[1]);
        }
    }

    let qrow;
    match pgconn.query(querysql.as_str(), &[]) {
        Ok(q) => {
            qrow = q;
        },
        Err(e) => {
            err!(e);
            return Err(format!("{}\"err\":\"db query err\",\"id\":{}{}", "{", req.id, "}"));
        }
    }

    let qres = qrow.get(0);
    let res: String;
    match qres.get(0) {
        Some(r) => res = r,
        None => {
            err!("empty result");
            return Err(format!("{}\"err\":\"empty result\",\"id\":{}{}", "{", req.id, "}"));
        }
    }

    Ok((res, req.id))
}
