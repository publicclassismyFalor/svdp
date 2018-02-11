pub mod rd;
pub mod wr;
pub mod rd_tps;
pub mod wr_tps;

pub struct NetIf {
    rd: u32,  /* kbytes */
    wr: u32,
    rdio: u32,  /* tps */
    wrio: u32,
}
