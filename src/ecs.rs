// extern crate serde_json;
// 
// use serde_json::Value;
// 
// use std::str;
// use std::process::Command;
// use std::collections::HashMap;
// 
// use std::thread;
// use std::sync::mpsc;
// //use std::time::Duration;
// 
// use std::io::Error;
// 
// const CMD: &str = "/tmp/aliyun_cmdb";
// const ARGV: &[&str] = &["-userId", "LTAIHYRtkSXC1uTl", "-userKey", "l1eLkvNkVRoPZwV9jwRpmq1xPOefGV"];
// 
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
//     netif: Vec<NetIf>,
// }
// 
// /* key: instance_id */
// struct ECS {
//     data: HashMap<i32, Data>,  /* K: time_stamp, V: Data */
// 
//     disk: HashMap<String, String>,  /* K: device, V: device_id */
//     netif: HashMap<String, String>,
// }
// 
// impl ECS {
//     fn new() -> ECS {
//         ECS {
//             data: HashMap::new(),
//             disk: HashMap::new(),
//             netif: HashMap::new(),
//         }
//     }
// }
// 
// enum DataT {
//     Ecs,
//     Disk,
//     NetIf,
// }
// 
// impl DataT {
//     fn cmd_new(&self, region: String) -> Vec<String> {
//         match *self {
//             DataT::Ecs => {
//                 vec![
//                     "-region".to_owned(),
//                     region,
//                     "-domain".to_owned(),
//                     "ecs.aliyuncs.com".to_owned(),
//                     "-apiName".to_owned(),
//                     "DescribeInstances".to_owned(),
//                     "-apiVersion".to_owned(),
//                     "2014-05-26".to_owned(),
//                     "Action".to_owned(),
//                     "DescribeInstances".to_owned(),
//                     "PageSize".to_owned(),
//                     "100".to_owned(),
//                 ]
//             },
//             DataT::Disk => {
//                 vec![
//                     "-region".to_owned(),
//                     region,
//                     "-domain".to_owned(),
//                     "ecs.aliyuncs.com".to_owned(),
//                     "-apiName".to_owned(),
//                     "DescribeDisks".to_owned(),
//                     "-apiVersion".to_owned(),
//                     "2014-05-26".to_owned(),
//                     "Action".to_owned(),
//                     "DescribeDisks".to_owned(),
//                     "PageSize".to_owned(),
//                     "100".to_owned(),
//                 ]
//             },
//             DataT::NetIf => {
//                 vec![]
//             },
//         }
//     }
// 
//     fn data_insert(&self, holder: &mut HashMap<String, ECS>, data: Vec<u8>) {
//         match *self {
//             DataT::Ecs => {
//                 DataT::ecs_data_insert(holder, data);
//             },
//             DataT::Disk => {
//                 DataT::disk_data_insert(holder, data);
//             },
//             DataT::NetIf => {
//                 DataT::netif_data_insert(holder, data);
//             },
//         }
//     }
// 
//     fn ecs_data_insert(holder: &mut HashMap<String, ECS>, data: Vec<u8>) {
//         let v: Value = serde_json::from_slice(&data).unwrap_or(Value::Null);
//         if Value::Null == v {
//             return;
//         }
// 
//         let body = &v["Instances"]["Instance"];
//         for i in 0.. {
//             if Value::Null == body[i] {
//                 break;
//             } else {
//                 if let Value::String(ref id) = body[i]["InstanceId"] {
//                     holder.insert((*id).clone(), ECS::new());
//                 }
//             }
//         }
//     }
// 
//     fn disk_data_insert(holder: &mut HashMap<String, ECS>, data: Vec<u8>) {
//         let v: Value = serde_json::from_slice(&data).unwrap_or(Value::Null);
//         if Value::Null == v {
//             return;
//         }
// 
//         let body = &v["Disks"]["disk"];
//         let mut diskid;
//         let mut device;
//         for i in 0.. {
//             if Value::Null == body[i] {
//                 break;
//             } else {
//                 if let Value::String(ref ecsid) = body[i]["InstanceId"] {
//                     if let Some(ecs) = holder.get_mut(ecsid) {
//                         if let Value::String(ref id) = body[i]["DiskId"] {
//                             diskid= id;
//                         } else {
//                             continue;
//                         }
// 
//                         if let Value::String(ref dev) = body[i]["Device"] {
//                             device = dev;
//                         } else {
//                             continue;
//                         }
// 
//                         ecs.disk.insert((*device).clone(), (*diskid).clone());
//                     }
//                 }
//             }
//         }
//     }
// 
//     fn netif_data_insert(holder: &mut HashMap<String, ECS>, data: Vec<u8>) {
//     }
// }
// 
// fn main() {
//     let regions;
// 
//     match get_region() {
//         Ok(res) => regions = res,
//         Err(e) => {
//             eprintln!("{}", e);
//             std::process::exit(1);
//         },
//     }
// 
//     let mut tids = Vec::new();
//     for region in regions.into_iter() {
//         tids.push(thread::spawn(move || get_meta(region)));
//     }
// 
//     for tid in tids {
//         tid.join();
//     }
// 
// //    let mut cmd = Command::new(CMD);
// //    let mut argv = Vec::new();
// //
// //    for x in ARGV.iter() {
// //        argv.push(*x);
// //    }
// //
// //    let mut argv = argv.clone();
// //    argv.push("-domain");
// //    argv.push("ecs.aliyuncs.com");
// //    argv.push("-apiName");
// //    argv.push("DescribeRegions");
// //    argv.push("-apiVersion");
// //    argv.push("2014-05-26");
// //    argv.push("Action");
// //    argv.push("DescribeRegions");
// //
// //    //argv.push("-region");
// //    //argv.push("cn-beijing");
// //    //argv.push("-domain");
// //    //argv.push("metrics.aliyuncs.com");
// //    //argv.push("-apiName");
// //    //argv.push("QueryMetricList");
// //    //argv.push("-apiVersion");
// //    //argv.push("2017-03-01");
// //    //argv.push("Action");
// //    //argv.push("QueryMetricList");
// //    //argv.push("Project");
// //    //argv.push("acs_ecs_dashboard");
// //    //argv.push("Metric");
// //    //argv.push("load_1m");
// //    //argv.push("Dimensions");
// //    //argv.push(r#"[{"instanceId":"i-2zeheigehruk0tj7s83h"}]"#);
// //    //argv.push("Length");
// //    //argv.push("2");
// //
// //    cmd.args(argv);
// //
// //    match cmd.output() {
// //        Ok(o) => {
// //            let v: Value = serde_json::from_slice(& o.stdout).unwrap();
// //            //let v: Value = serde_json::from_slice(str::from_utf8(& o.stdout).unwrap()).unwrap();
// //
// //            for x in 0.. {
// //                if Value::Null == v["Regions"]["Region"][x] {
// //                    break;
// //                } else {
// //                    println!("{}", v["Regions"]["Region"][x]["RegionId"]);
// //                }
// //            }
// //
// //            //println!("{}", o.stdout.len());
// //            //println!("{}", str::from_utf8(& o.stdout[0..12]).unwrap().split_whitespace().next().unwrap().parse::<i32>().unwrap_or_else(|e| {
// //            //    println!("fuck! {}", e);
// //            //    std::process::exit(1);
// //            //}));
// //            //println!("{}", String::from_utf8_lossy(& o.stdout));
// //        },
// //        Err(e) => {
// //            println!("ERR: {}", e);
// //        }
// //    }
// }
// 
// fn cmd_exec(mut extra: Vec<String>) -> Result<Vec<u8>, Error> {
//     let mut argv: Vec<String> = Vec::new();
// 
//     for x in ARGV.iter() {
//         argv.push((**x).to_string());
//     }
// 
//     argv.append(&mut extra);
// 
//     let output = Command::new(CMD).args(argv).output() ?;
// 
//     if output.status.success() {
//         return Ok(output.stdout);
//     } else {
//         return Err(Error::from_raw_os_error(output.status.code().unwrap_or(1)));
//     }
// }
// 
// fn get_region() -> Result<Vec<String>, String> {
//     let mut res: Vec<String> = Vec::new();
//     let extra = vec![
//         "-domain".to_owned(),
//         "ecs.aliyuncs.com".to_owned(),
//         "-apiName".to_owned(),
//         "DescribeRegions".to_owned(),
//         "-apiVersion".to_owned(),
//         "2014-05-26".to_owned(),
//         "Action".to_owned(),
//         "DescribeRegions".to_owned(),
//     ];
// 
//     match cmd_exec(extra) {
//         Ok(o) => {
//             let v: Value = serde_json::from_slice(&o).unwrap_or(Value::Null);
//             if Value::Null == v {
//                 return Err("E0!".to_string());
//             }
// 
//             for x in 0.. {
//                 if Value::Null == v["Regions"]["Region"][x] {
//                     break;
//                 } else {
//                     /* map 方式解析出来的 json string 是带引号的，需要处理掉 */
//                     if let Value::String(ref s) = v["Regions"]["Region"][x]["RegionId"] {
//                         res.push(s.to_string());
//                     } else {
//                         return Err("json parse err".to_string());
//                     }
//                 }
//             }
//         },
//         Err(e) => {
//             return Err(e.to_string());
//         }
//     }
// 
//     Ok(res)
// }
// 
// /*
//  * return HashMap(contains meta info of all ecs+disk+netif)
//  * @param start_time: unix time_stamp
//  */
// fn get_meta(region: String) {
//     let mut holder = HashMap::new();
// 
//     let mut extra = vec![
//         "-domain".to_owned(),
//         "ecs.aliyuncs.com".to_owned(),
//         "-apiName".to_owned(),
//         "DescribeInstances".to_owned(),
//         "-apiVersion".to_owned(),
//         "2014-05-26".to_owned(),
//         "-region".to_owned(),
//         region,
//         "Action".to_owned(),
//         "DescribeInstances".to_owned(),
//         "PageSize".to_owned(),
//         "100".to_owned(),
//     ];
// 
//     let ret: Vec<u8>;
//     if let Ok(cmd_ret) = cmd_exec(extra.clone()) {
//         ret = cmd_ret;
//     } else {
//         return;
//     }
// 
//     let v: Value = serde_json::from_slice(&ret).unwrap_or(Value::Null);
//     if Value::Null == v {
//         return;
//     }
// 
//     let mut total_pages: u64 = 0;
//     if let Value::Number(ref total) = v["TotalCount"] {
//         total_pages = total.as_u64().unwrap_or(0);
//         if 0 == total_pages {
//             return;
//         } else if 0 == total_pages % 100 {
//             total_pages = total_pages / 100;
//         } else {
//             total_pages = 1 + total_pages / 100;
//         }
//     } else {
//         return;
//     }
// 
//     meta_insert(&mut holder, ret);
// 
//     if 1 < total_pages {
//         extra.push("PageNumber".to_owned());
// 
//         let worker = |tx: mpsc::Sender<Vec<u8>>, page: u64| {
//             let mut extra_ = extra.clone();
//             thread::spawn(move || {
//                 extra_.push(page.to_string());
// 
//                 if let Ok(ret) = cmd_exec(extra_) {
//                     tx.send(ret).unwrap_or_else(|e| eprintln!("{}", e));
//                 }
//             });
//         };
// 
//         let (tx, rx) = mpsc::channel();
//         for i in 3..(total_pages + 1) {
//             worker(mpsc::Sender::clone(&tx), i);
//         }
// 
//         worker(tx, 2);
// 
//         for i in rx {
//             meta_insert(&mut holder, i);
//         }
//     }
// 
//     get_meta_disk(&mut holder);
//     get_meta_netif(&mut holder);
// }
// 
// fn get_meta_disk(holder: &mut HashMap<String, ECS>) {
//     //let mut extra = vec![
//     //    "-domain".to_owned(),
//     //    "ecs.aliyuncs.com".to_owned(),
//     //    "-apiName".to_owned(),
//     //    "DescribeDisks".to_owned(),
//     //    "-apiVersion".to_owned(),
//     //    "2014-05-26".to_owned(),
//     //    "-region".to_owned(),
//     //    "".to_owned(),
//     //    "Action".to_owned(),
//     //    "DescribeDisks".to_owned(),
//     //    "PageSize".to_owned(),
//     //    "100".to_owned(),
//     //];
// 
//     //let ret: Vec<u8>;
//     //if let Ok(cmd_ret) = cmd_exec(extra.clone()) {
//     //    ret = cmd_ret;
//     //} else {
//     //    return;
//     //}
// 
//     //let v: Value = serde_json::from_slice(&ret).unwrap_or(Value::Null);
//     //if Value::Null == v {
//     //    return;
//     //}
// 
//     //let mut total_pages: u64 = 0;
//     //if let Value::Number(ref total) = v["TotalCount"] {
//     //    total_pages = total.as_u64().unwrap_or(0);
//     //    if 0 == total_pages {
//     //        return;
//     //    } else if 0 == total_pages % 100 {
//     //        total_pages = total_pages / 100;
//     //    } else {
//     //        total_pages = 1 + total_pages / 100;
//     //    }
//     //} else {
//     //    return;
//     //}
// 
//     //meta_insert(&mut holder, ret);
// 
//     //if 1 < total_pages {
//     //    extra.push("PageNumber".to_owned());
// 
//     //    let worker = |tx: mpsc::Sender<Vec<u8>>, page: u64| {
//     //        let mut extra_ = extra.clone();
//     //        thread::spawn(move || {
//     //            extra_.push(page.to_string());
// 
//     //            if let Ok(ret) = cmd_exec(extra_) {
//     //                tx.send(ret).unwrap_or_else(|e| eprintln!("{}", e));
//     //            }
//     //        });
//     //    };
// 
//     //    let (tx, rx) = mpsc::channel();
//     //    for i in 3..(total_pages + 1) {
//     //        worker(mpsc::Sender::clone(&tx), i);
//     //    }
// 
//     //    worker(tx, 2);
// 
//     //    for i in rx {
//     //        meta_insert(&mut holder, i);
//     //    }
//     //}
// 
// }
// 
// fn get_meta_netif(holder: &mut HashMap<String, ECS>) {
// 
// }
// 
// fn meta_insert(holder: &mut HashMap<String, ECS>, data: Vec<u8>) {
//     let v: Value = serde_json::from_slice(&data).unwrap_or(Value::Null);
//     if Value::Null == v {
//         return;
//     }
// 
//     for x in 0.. {
//         if Value::Null == v["Instances"]["Instance"][x] {
//             break;
//         } else {
//             /* map 方式解析出来的 json string 是带引号的，需要处理掉 */
//             if let Value::String(ref id) = v["Instances"]["Instance"][x]["InstanceId"] {
//                 holder.insert((*id).clone(), ECS::new());
//             } else {
//                 eprintln!("InstanceId: json parse err!");
//             }
//         }
//     }
// }
// 
// //fn meta_insert_disk(holder: &mut HashMap<String, ECS>) {
// //
// //}
// //
// //fn meta_insert_netif(holder: &mut HashMap<String, ECS>) {
// //
// //}
// 

pub fn sv() {
    println!("{}\n{:?}", ::CMD, ::ARGV);
}