extern crate serde_json;

use serde_json::Value;

use std::str;
use std::process::Command;
use std::collections::HashMap;

use std::thread;
use std::sync::{Mutex, Arc};

use std::io::{Error, ErrorKind};

const CMD: &str = "/tmp/aliyun_cmdb";
const ARGV: &[&str] = &["-userId", "LTAIHYRtkSXC1uTl", "-userKey", "l1eLkvNkVRoPZwV9jwRpmq1xPOefGV"];

struct Disk {
    device: String,  /* device name: /dev/vda */

    total: u64,  /* M */
    spent: u64,
    rd: u32,  /* kbytes */
    wr: u32,
    rdio: u32,  /* tps */
    wrio: u32,
}

struct NetIf {
    device: String,  /* device name: eth0 */

    rd: u32,  /* kbytes */
    wr: u32,
    rdio: u32,  /* tps */
    wrio: u32,
}

struct Data {
    cpu_rate: u16,
    mem_rate: u16,
    load: [u16;2],  /* load_5m/load_15m */
    tcp_conn: u32,

    disk: Vec<Disk>,
    net_if: Vec<NetIf>,
}

/* key: instance_id */
struct ECS {
    data: HashMap<i32, Data>,  /* K: time_stamp, V: Data */

    disk: HashMap<String, String>,  /* K: device, V: device_id */
    net_if: HashMap<String, String>,
}

fn main() {
    let regions;

    match region_parse() {
        Ok(res) => regions = res,
        Err(e) => {
            eprintln!("{}", e);
            std::process::exit(1);
        },
    }

    for region in regions.into_iter() {
        meta_parse(region);
    }

//    let mut cmd = Command::new(CMD);
//    let mut argv = Vec::new();
//
//    for x in ARGV.iter() {
//        argv.push(*x);
//    }
//
//    let mut argv = argv.clone();
//    argv.push("-domain");
//    argv.push("ecs.aliyuncs.com");
//    argv.push("-apiName");
//    argv.push("DescribeRegions");
//    argv.push("-apiVersion");
//    argv.push("2014-05-26");
//    argv.push("Action");
//    argv.push("DescribeRegions");
//
//    //argv.push("-region");
//    //argv.push("cn-beijing");
//    //argv.push("-domain");
//    //argv.push("metrics.aliyuncs.com");
//    //argv.push("-apiName");
//    //argv.push("QueryMetricList");
//    //argv.push("-apiVersion");
//    //argv.push("2017-03-01");
//    //argv.push("Action");
//    //argv.push("QueryMetricList");
//    //argv.push("Project");
//    //argv.push("acs_ecs_dashboard");
//    //argv.push("Metric");
//    //argv.push("load_1m");
//    //argv.push("Dimensions");
//    //argv.push(r#"[{"instanceId":"i-2zeheigehruk0tj7s83h"}]"#);
//    //argv.push("Length");
//    //argv.push("2");
//
//    cmd.args(argv);
//
//    match cmd.output() {
//        Ok(o) => {
//            let v: Value = serde_json::from_slice(& o.stdout).unwrap();
//            //let v: Value = serde_json::from_slice(str::from_utf8(& o.stdout).unwrap()).unwrap();
//
//            for x in 0.. {
//                if Value::Null == v["Regions"]["Region"][x] {
//                    break;
//                } else {
//                    println!("{}", v["Regions"]["Region"][x]["RegionId"]);
//                }
//            }
//
//            //println!("{}", o.stdout.len());
//            //println!("{}", str::from_utf8(& o.stdout[0..12]).unwrap().split_whitespace().next().unwrap().parse::<i32>().unwrap_or_else(|e| {
//            //    println!("fuck! {}", e);
//            //    std::process::exit(1);
//            //}));
//            //println!("{}", String::from_utf8_lossy(& o.stdout));
//        },
//        Err(e) => {
//            println!("ERR: {}", e);
//        }
//    }
}

fn cmd_exec(mut extra: Vec<&str>) -> Result<Vec<u8>, Error> {
    let mut argv = Vec::new();

    for x in ARGV.iter() {
        argv.push(*x);
    }

    argv.append(&mut extra);

    let output = Command::new(CMD).args(argv).output() ?;

    if output.status.success() {
        return Ok(output.stdout);
    } else {
        return Err(Error::from_raw_os_error(output.status.code().unwrap_or(1)));
    }
}

fn region_parse() -> Result<Vec<String>, String> {
    let mut res = Vec::new();
    let extra = vec![
        "-domain",
        "ecs.aliyuncs.com",
        "-apiName",
        "DescribeRegions",
        "-apiVersion",
        "2014-05-26",
        "Action",
        "DescribeRegions",
    ];

    match cmd_exec(extra) {
        Ok(o) => {
            let v: Value = serde_json::from_slice(&o).unwrap_or(Value::Null);
            if Value::Null == v {
                return Err("E0!".to_string());
            }

            for x in 0.. {
                if Value::Null == v["Regions"]["Region"][x] {
                    break;
                } else {
                    /* map 方式解析出来的 json string 是带引号的，需要处理掉 */
                    if let Value::String(ref s) = v["Regions"]["Region"][x]["RegionId"] {
                        res.push(s.to_string());
                    } else {
                        return Err("json parse err".to_string());
                    }
                }
            }
        },
        Err(e) => {
            return Err(e.to_string());
        }
    }

    Ok(res)
}

/*
 * return HashMap(contains meta info of all ecs+disk+netif)
 * @param start_time: unix time_stamp
 */
fn meta_parse(region: String) -> Result<HashMap<String, ECS>, Error> {
    let holder = HashMap::new();
    let holder_arc = Arc::new(Mutex::new(&holder));

    let extra = vec![
        "-domain",
        "ecs.aliyuncs.com",
        "-apiName",
        "DescribeInstances",
        "-apiVersion",
        "2014-05-26",
        "-region",
        &region,
        "Action",
        "DescribeInstances",
        "PageSize",
        "100",
    ];

    let cmd_ret: Vec<u8> = cmd_exec(extra) ?;

    let v: Value = serde_json::from_slice(&cmd_ret).unwrap_or(Value::Null);
    if Value::Null == v {
        return Err(Error::new(ErrorKind::Other, "E1!".to_string()));
    }

    let mut total_pages: i32 = 0;
    if let Value::String(ref total) = v["TotalCount"] {
        total_pages = total.parse().unwrap_or(0);
    } else {
        return Err(Error::new(ErrorKind::Other, "E2!".to_string()));
    }

    //if 0 < total_pages {

    //    for x in 2..total_pages {

    //    }
    //}

    //for x in 0.. {
    //    if Value::Null == v["Regions"]["Region"][x] {
    //        break;
    //    } else {
    //        /* map 方式解析出来的 json string 是带引号的，需要处理掉 */
    //        if let Value::String(ref s) = v["Regions"]["Region"][x]["RegionId"] {
    //            res.push(s.to_string());
    //        } else {
    //            return Err("json parse err".to_string());
    //        }
    //    }
    //}

    holder_arc.lock();
    Ok(holder)
}

//fn sv_parse(region &str, meta &mut HashMap<String, ECS>, start_time: i32) -> Result<(), String> {
//
//}
//
//fn write_db(data &mut HashMap<String, ECS>) -> Result<(), String> {
//
//}
