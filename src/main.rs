extern crate serde_json;

use serde_json::Value;

use std::process::Command;
//use std::collections::HashMap;
use std::str;

const CMD: &str = "/tmp/aliyun_cmdb";
const ARGV: &[&str] = &["-userId", "LTAIHYRtkSXC1uTl", "-userKey", "l1eLkvNkVRoPZwV9jwRpmq1xPOefGV"];

// struct Disk {
//     device: String,  /* device name: /dev/vda */
// 
//     total: u64,  /* M */
//     spent: u64,
//     rd: u32,  /* kbytes */
//     wr: u32,
//     rdio: u32,  /* tps */
//     wrio: u32,
// }
// 
// struct NetIf {
//     device: String,  /* device name: eth0 */
// 
//     rd: u32,  /* kbytes */
//     wr: u32,
//     rdio: u32,  /* tps */
//     wrio: u32,
// }
// 
// struct Data {
//     cpu_rate: u16,
//     mem_rate: u16,
//     load: [u16;2],  /* load_5m/load_15m */
//     tcp_conn: u32,
// 
//     disk: Vec<Disk>,
//     net_if: Vec<NetIf>,
// }
// 
// /* key: instance_id */
// struct Ecs {
//     data: HashMap<i32, Data>,  /* K: time_stamp, V: Data */
// 
//     disk: HashMap<String, String>,  /* K: device, V: device_id */
//     net_if: HashMap<String, String>,
// }

fn main() {
    let regions;

    match region_parse() {
        Ok(res) => regions = res,
        Err(e) => {
            eprintln!("{}", e);
            std::process::exit(1);
        },
    }

    println!("{:#?}", regions);

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

fn region_parse() -> Result<Vec<String>, String> {
    let mut res = Vec::new();

    let mut cmd = Command::new(CMD);
    let mut argv = Vec::new();
    let mut suffix = vec![
        "-domain",
        "ecs.aliyuncs.com",
        "-apiName",
        "DescribeRegions",
        "-apiVersion",
        "2014-05-26",
        "Action",
        "DescribeRegions",
    ];

    for x in ARGV.iter() {
        argv.push(*x);
    }

    argv.append(&mut suffix);

    cmd.args(argv);

    if let Ok(o) = cmd.output() {
        let v = serde_json::from_slice::<Value>(& o.stdout).unwrap_or(Value::Null); 
        if Value::Null == v {
            return Err("!!!!".to_string());
        }

        for x in 0.. {
            if Value::Null == v["Regions"]["Region"][x] {
                break;
            } else {
                res.push(v["Regions"]["Region"][x]["RegionId"].to_string());
            }
        }

    } else {
        return Err("!!!!".to_string());
    }

    Ok(res)
}
