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
use std::net::TcpListener;

use threadpool::ThreadPool;
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
    instance_id: Option<Vec<String>>,
    ts_range: [i32; 2],
}

/// REQ example:
/// {"method":"sv_ecs","params":{"instance_id":"i-123456","ts_range":[15000000,1600000]},"id":0}
///
/// RES example:
/// {"result":"{...}","id":0}
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
            Ok((mut socket, _peeraddr)) => {
                let pgpool = pgpool.clone();
                tdpool.execute(move|| {
                    let mut buf = String::new();
                    while 0 < socket.read_to_string(&mut buf)
                        .unwrap_or_else(|e|{ errexit!(e); }) {}

                    let req: Req = serde_json::from_str(&buf).unwrap_or_else(|e| {
                        socket.write("{\"err\":\"json parse err\",\"id\":-1}".as_bytes())
                            .unwrap_or_else(|err|{ errexit!(err); });
                        errexit!(e);
                    });

                    let querysql = String::new();
                    match req.params.instance_id {
                        None => {
                        },
                        _ => {
                        }
                    }

                    let pgconn = pgpool.get().unwrap_or_else(|e| {
                        socket.write(format!("{}\"err\":\"db_conn_pool busy\",\"id\":{}{}", "{", req.id, "}").as_bytes())
                            .unwrap_or_else(|err|{ errexit!(err); });
                        errexit!(e);
                    });

                    let qres = pgconn.query(querysql.as_str(), &[]).unwrap_or_else(|e| {
                        socket.write(format!("{}\"err\":\"db query err\",\"id\":{}{}", "{", req.id, "}").as_bytes())
                            .unwrap_or_else(|err|{ errexit!(err); });
                        errexit!(e);
                    });

                    let resrow = qres.get(0);
                    let res = resrow.get_bytes(0).unwrap_or_else(|| {
                        socket.write(format!("{}\"err\":\"empty result\",\"id\":{}{}", "{", req.id, "}").as_bytes())
                            .unwrap_or_else(|err|{ errexit!(err); });
                        errexit!("empty result");
                    });

                    socket.write(res).unwrap_or_else(|e|{ errexit!(e); });
                });
            },

            Err(e) => err!(e)
        }
    }
}


pub fn run() {
    thread::spawn(|| jsonrpc_serv());

    sv::go();
    dp::go();
}
