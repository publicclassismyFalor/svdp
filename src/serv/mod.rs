#[macro_use] mod sv;
mod sv_analyse;
mod dp;

use std::io::{Read, Write};
use std::io::{Error, ErrorKind};
use std::net::{TcpStream, TcpListener};

use std::time::Duration;

/* async http serv */
use iron::prelude::*;
use iron::status;

use threadpool::ThreadPool;

use regex::Regex;

/****************
 * http service *
 ****************/
pub fn http_serv() {
    let addr = ::CONF.sv_http_addr.clone();
    Iron::new(http_ops).http(&addr.unwrap())
        .unwrap_or_else(|e|{ errexit!(e); });
}

fn http_ops(request: &mut Request) -> IronResult<Response> {
    let mut buf = String::new();
    request
        .body
        .read_to_string(&mut buf)
        .map_err(|e| IronError::new(e, (status::InternalServerError, "request reading err")))?;

    match router(&buf) {
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

    let addr = ::CONF.sv_tcp_addr.clone();
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
    let mut buf = String::new();
    if let Err(e) = socket.read_to_string(&mut buf) {
        let errmsg = "{\"err\":\"socket read err\",\"id\":-1}";

        if let Err(ee) = socket.write(errmsg.as_bytes()) {
            err!(ee)
        }

        err!(e);
        return;
    }

    match router(&buf) {
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

/**********
 * ROUTER *
 **********/
lazy_static! {
    static ref RE: Regex = Regex::new(r#"method":"(\w{4})"#).unwrap_or_else(|e| errexit!(e) );
}

fn router(body: &str) -> Result<(String, i32), (String, i32)> {
    let capb;
    match RE.captures(body) {
        Some(cap) => {
            capb = cap.get(1).map_or("".as_bytes(), |m| m.as_str().as_bytes());
        },
        None => {
            err!(body);
            return Err(("method invalid".to_owned(), -1));
        }
    }

    if 3 < capb.len() {
        match &capb[0..3] {
            b"sv_" => return sv::worker(body),
            //b"SV_" => { },
            //b"dp_" => { },
            _ => {
                err!(body);
                return Err(("method invalid".to_owned(), -1));
            }
        }
    } else {
        err!(body);
        return Err(("method invalid".to_owned(), -1));
    }
}
