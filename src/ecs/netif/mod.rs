pub mod rd;
pub mod wr;
pub mod rd_tps;
pub mod wr_tps;

pub struct NetIf {
    rd: i32,  /* kbytes */
    wr: i32,
    rdtps: i32,
    wrtps: i32,
}

impl NetIf {
    fn new() -> NetIf {
        NetIf {
            rd: 0,
            wr: 0,
            rdtps: 0,
            wrtps: 0,
        }
    }
}
