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
mod serv;
mod sv;
mod dp;

use std::thread;
use std::fs::File;
use std::io::Read;

use r2d2::Pool;
use r2d2_postgres::{TlsMode, PostgresConnectionManager};

#[derive(Deserialize)]
pub struct Config {
    pg_login_url: String,  // UNIX DOMAIN SOCKET: "postgres://jack@%2Fhome%2Fjack/svdp"
    sv_tcp_addr: String,  // "[::1]:20000"
    sv_http_addr: String,  // "[::1]:30000"
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
    thread::spawn(|| serv::http_serv());
    thread::spawn(|| serv::tcp_serv());

    sv::go();
    dp::go();
}
