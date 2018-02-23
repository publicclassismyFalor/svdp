#[macro_use] extern crate lazy_static;
extern crate serde;
extern crate serde_json;
#[macro_use] extern crate serde_derive;
extern crate r2d2;
extern crate r2d2_postgres;
extern crate postgres;
extern crate toml;
extern crate threadpool;

#[macro_use] mod zmacro;
mod sv;
mod dp;

use std::thread;
use std::fs::File;
use std::io::{Read, Write};
use std::net::{TcpStream, TcpListener};

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

/* json rpc service on tcp */
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

/// REQ example:
/// {"method":"sv_ecs","params":{"instance_id":"i-123456","ts_range":[15000000,1600000]},"id":0}
///
/// RES example:
/// {"result":["ts":1519379068,"data":{...}],"id":0}
/// OR
/// {"err":"...","id":0}
fn jsonrpc_serv() {
    let tdpool = ThreadPool::new(20);

    let pgmg = PostgresConnectionManager::new(CONF.pg_login_url.as_str(), TlsMode::None)
        .unwrap_or_else(|e|{ errexit!(e); });
    let pgpool = r2d2::Pool::builder()
        .max_size(20)
        .build(pgmg)
        .unwrap_or_else(|e|{ errexit!(e); });

    let listener = TcpListener::bind(&CONF.sv_serv_addr)
        .unwrap_or_else(|e|{ errexit!(e); });

    loop {
        match listener.accept() {
            Ok((socket, _peeraddr)) => {
                let pgpool = pgpool.clone();
                tdpool.execute(move|| {
                    worker(socket, pgpool);
                });
            },

            Err(e) => err!(e)
        }
    }
}

fn worker(mut socket: TcpStream, pgpool: Pool<PostgresConnectionManager>) {
    let mut buf = String::new();
    loop {
        match socket.read_to_string(&mut buf) {
            Ok(cnt) if 0 == cnt => break,
            Err(e) => {
                err!(e);
                return;
            },
            _ => continue
        }
    }

    let req: Req;
    match serde_json::from_str(&buf) {
        Ok(r) => req = r,
        Err(e) => {
            let errmsg = "{\"err\":\"json parse err\",\"id\":-1}";
            socket.write(errmsg.as_bytes()).unwrap_or_default();

            err!(e);
            return;
        }
    }

    let pgconn;
    match pgpool.get() {
        Ok(conn) => pgconn = conn,
        Err(e) => {
            let errmsg = format!("{}\"err\":\"db_conn_pool busy\",\"id\":{}{}", "{", req.id, "}");
            socket.write(errmsg.as_bytes()).unwrap_or_default();

            err!(e);
            return;
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
                               (SELECT ts, (SELECT row_to_json(t) FROM (SELECT sv->'{}' as {}) t) AS sv FROM {} WHERE ts >= {} AND ts <= {}) d", insid, insid, req.method, req.params.ts_range[0], req.params.ts_range[1]);
        }
    }

    let qrow;
    match pgconn.query(querysql.as_str(), &[]) {
        Ok(q) => {
            qrow = q;
        },
        Err(e) => {
            let errmsg = format!("{}\"err\":\"db query err\",\"id\":{}{}", "{", req.id, "}");
            socket.write(errmsg.as_bytes()).unwrap_or_default();

            err!(e);
            return;
        }
    }

    let qres = qrow.get(0);
    let res: String;
    match qres.get(0) {
        Some(r) => res = r,
        None => {
            let errmsg = format!("{}\"err\":\"empty result\",\"id\":{}{}", "{", req.id, "}");
            socket.write(errmsg.as_bytes()).unwrap_or_default();

            err!("empty result");
            return;
        }
    }

    if let Err(e) = socket.write(format!("{}\"result\":{},\"id\":{}{}" , "{", res, req.id, "}").as_bytes()) {
        err!(e);
    }
}

pub fn run() {
    thread::spawn(|| jsonrpc_serv());

    sv::go();
    dp::go();
}
