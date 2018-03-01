macro_rules! errexit {
    ($x: expr) => {
        {
            use colored::Colorize;
            eprintln!("{} [{}, {}] ==> {}", ::time::strftime("%m-%d %H:%M:%S", &::time::now()).unwrap().red().bold(), file!(), line!(), $x);
            ::std::process::exit(1);
        }
    }
}

macro_rules! err {
    ($x: expr) => {
        {
            use colored::Colorize;
            eprintln!("{} [{}, {}] ==> {}", ::time::strftime("%m-%d %H:%M:%S", &::time::now()).unwrap().red().bold(), file!(), line!(), $x);
        }
    }
}
