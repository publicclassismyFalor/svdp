macro_rules! errexit {
    ($x: expr) => {
        eprintln!("[{}, {}] ==> {}", file!(), line!(), $x);
        ::std::process::exit(1);
    }
}

macro_rules! err {
    ($x: expr) => {
        eprintln!("[{}, {}] ==> {}", file!(), line!(), $x);
    }
}
