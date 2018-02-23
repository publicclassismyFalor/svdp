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
use std::io::Read;
use std::net::{TcpListener, TcpStream};

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
            Ok((_socket, _addr)) => {
                let pgpool = pgpool.clone();

                tdpool.execute(move|| {
                    let pgconn = pgpool.get().unwrap_or_else(|e| { errexit!(e); });
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
