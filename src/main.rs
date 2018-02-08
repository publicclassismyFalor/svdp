use std::process::Command;
use std::collections::HashMap;
use std::str;

const CMD: &str = "/tmp/aliyun_cmdb";
const ARGVBASE: &[&str] = &["-userId", "LTAIHYRtkSXC1uTl", "-userKey", "l1eLkvNkVRoPZwV9jwRpmq1xPOefGV"];

struct NetIfData {
    rd: u32,  /* kbytes */
    wr: u32,
    rdio: u32,  /* tps */
    wrio: u32,
}

/* key: net_if_id */
struct NetIf {
    device: String,  /* device name: eth0 */
    data: HashMap<i32, NetIfData>,
}

struct DiskData {
    total: u64,  /* M */
    spent: u64,
    rd: u32,  /* kbytes */
    wr: u32,
    rdio: u32,  /* tps */
    wrio: u32,
}

/* key: disk_id */
struct Disk {
    device: String,  /* device name: /dev/vda */
    data: HashMap<i32, DiskData>,
}

/* key: time_stamp */
struct EcsData {
    cpu_rate: u16,
    mem_rate: u16,
    load: [u16;2],
    tcp_conn: u32,
}

/* key: instance_id */
struct Ecs {
    data: HashMap<i32, EcsData>,

    disk: HashMap<String, Disk>,
    net_if: HashMap<String, NetIf>,
}

fn main() {
    let mut cmd = Command::new(CMD);
    let mut argv = Vec::new();

    for x in ARGVBASE.iter() {
        argv.push(*x);
    }

    argv.push("-region");
    argv.push("cn-beijing");
    argv.push("-domain");
    argv.push("metrics.aliyuncs.com");
    argv.push("-apiName");
    argv.push("QueryMetricList");
    argv.push("-apiVersion");
    argv.push("2017-03-01");
    argv.push("Action");
    argv.push("QueryMetricList");
    argv.push("Project");
    argv.push("acs_ecs_dashboard");
    argv.push("Metric");
    argv.push("load_1m");
    argv.push("Dimensions");
    argv.push(r#"[{"instanceId":"i-2zeheigehruk0tj7s83h"}]"#);
    argv.push("Length");
    argv.push("2");

    cmd.args(argv);

    match cmd.output() {
        Ok(o) => {
            println!("{}", o.stdout.len());
            //println!("{}", str::from_utf8(& o.stdout[0..12]).unwrap().split_whitespace().next().unwrap().parse::<i32>().unwrap_or_else(|e| {
            //    println!("fuck! {}", e);
            //    std::process::exit(1);
            //}));
            println!("{}", String::from_utf8_lossy(& o.stdout[12..]).len());
        },
        Err(e) => {
            println!("ERR: {}", e);
        }
    }
}
