extern crate serde;
extern crate serde_json;

extern crate postgres;

#[macro_use]
extern crate serde_derive;

#[macro_use]
extern crate lazy_static;

mod sv;
mod dp;

pub fn run() {
    sv::go();
    dp::go();
}
