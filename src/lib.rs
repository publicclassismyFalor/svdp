#[macro_use]
extern crate lazy_static;

extern crate serde;
extern crate serde_json;
#[macro_use]
extern crate serde_derive;

extern crate postgres;
extern crate toml;

mod sv;
mod dp;

use std::thread;
use std::fs::File;
use std::io::Read;

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
    let mut file = File::open("major.toml").unwrap_or_else(|e| {
        eprintln!("[{}, {}] ==> {}", file!(), line!(), e);
        std::process::exit(1);
    });

    let mut content = String::new();
    file.read_to_string(&mut content).unwrap_or_else(|e| {
        eprintln!("[{}, {}] ==> {}", file!(), line!(), e);
        std::process::exit(1);
    });

    toml::from_str::<Config>(&content).unwrap_or_else(|e| {
        eprintln!("[{}, {}] ==> {}", file!(), line!(), e);
        std::process::exit(1);
    })
}

/* json rpc service on tcp */
fn jsonrpc_serv() {
}


pub fn run() {
    thread::spawn(|| jsonrpc_serv());

    sv::go();
    dp::go();
}
