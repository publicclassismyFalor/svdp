use std::env;
use std::process;
use std::io;
use std::io::prelude::*;
use std::net::{TcpStream, Shutdown};
//use std::time::Duration;


fn main() {
    let args: Vec<String> = env::args().collect();

    let mut port: &str = "30000";
    let content: &str;

    if 3 == args.len() {
        content = &args[2];
    } else if 4 == args.len() {
        port = &args[2];
        content = &args[3];
    } else {
        println! ("\n\nUsage:\n\tnotice <serv_ip> [serv_port, default 20000] <content_to_send>");
        process::exit(1);
    }

    let mut addr = args[1].clone();
    addr.push_str(":");
    addr.push_str(port);

    let mut recv_buf = [0];

    if let Ok(mut stream) = TcpStream::connect(addr) {
//        stream.set_read_timeout(Some(Duration::from_secs(3))).unwrap();

        stream.write(content.as_bytes()).unwrap();
        stream.shutdown(Shutdown::Write).unwrap();

        while 0 < stream.read(&mut recv_buf).unwrap() {
            io::stdout().write(&recv_buf).unwrap();
        }
    } else {
        eprintln! ("connect err!");
    }
}
