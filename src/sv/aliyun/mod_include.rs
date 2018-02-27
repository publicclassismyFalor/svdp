pub trait DATA {
    type Holder;

    fn argv_new(&self, region: String) -> Vec<String>;

    fn get(&self, holder: Self::Holder, region: String) {
        let mut extra = self.argv_new(region);

        let (tx, rx) = mpsc::channel();

        thread::spawn(move || {
            if let Ok(ret) = cmd_exec(extra.clone()) {
                let v = serde_json::from_slice(&ret).unwrap_or(Value::Null);
                if Value::Null == v {
                    return;
                }

                tx.send(ret).unwrap();

                if let Value::String(ref cursor) = v["Cursor"] {
                    extra.push("Cursor".to_owned());
                    extra.push((*cursor).clone());

                    while let Ok(ret) = cmd_exec(extra.clone()) {
                        let v = serde_json::from_slice(&ret).unwrap_or(Value::Null);
                        if Value::Null == v {
                            return;
                        }

                        tx.send(ret).unwrap();

                        if let Value::String(ref cursor) = v["Cursor"] {
                            extra.pop();
                            extra.push((*cursor).clone());
                        } else {
                            break;
                        }
                    }
                }
            }
        });

        for r in rx {
            self.insert(&holder, r);
        }
    }

    fn insert(&self, holder: &Self::Holder, data: Vec<u8>);
}




pub fn argv_new_base(region: String) -> Vec<String> {
    let mut argv = vec![
        "-region".to_owned(),
        region,
        "-domain".to_owned(),
        "metrics.aliyuncs.com".to_owned(),
        "-apiName".to_owned(),
        "QueryMetricList".to_owned(),
        "-apiVersion".to_owned(),
        "2017-03-01".to_owned(),
        "Action".to_owned(),
        "QueryMetricList".to_owned(),
        "Length".to_owned(),
        "1000".to_owned(),
    ];

    argv.push("StartTime".to_owned());
    unsafe {
        argv.push(BASESTAMP.to_string());
    }

    argv.push("EndTime".to_owned());
    unsafe {
        argv.push((BASESTAMP + INTERVAL).to_string());
    }

    argv
}

fn get_region() -> Option<Vec<String>> {
    let mut res: Vec<String> = Vec::new();
    let extra = vec![
        "-domain".to_owned(),
        "ecs.aliyuncs.com".to_owned(),
        "-apiName".to_owned(),
        "DescribeRegions".to_owned(),
        "-apiVersion".to_owned(),
        "2014-05-26".to_owned(),
        "Action".to_owned(),
        "DescribeRegions".to_owned(),
    ];

    if let Ok(stdout) = cmd_exec(extra) {
        let v: Value = serde_json::from_slice(&stdout).unwrap_or(Value::Null);
        if Value::Null == v {
            return None;
        }

        for x in 0.. {
            if Value::Null == v["Regions"]["Region"][x] {
                break;
            } else {
                if let Value::String(ref s) = v["Regions"]["Region"][x]["RegionId"] {
                    res.push(s.to_string());
                } else {
                    return None;
                }
            }
        }
    } else {
        return None;
    }

    Some(res)
}

fn cmd_exec(mut extra: Vec<String>) -> Result<Vec<u8>, Error> {
    let mut argv: Vec<String> = Vec::new();

    for x in ARGV.iter() {
        argv.push((**x).to_string());
    }

    argv.append(&mut extra);

    let output = Command::new(CMD).args(argv).output() ?;

    if output.status.success() {
        return Ok(output.stdout);
    } else {
        return Err(Error::from_raw_os_error(output.status.code().unwrap_or(1)));
    }
}

/* read from /proc/meminfo */
pub fn mem_insufficient() -> bool {
    let mut content = String::new();
    File::open("/proc/meminfo").unwrap()
        .read_to_string(&mut content).unwrap();

    let re = Regex::new(r"\s*(MemAvailable):\s+(\d+)").unwrap();

    /* 匹配结果的索引是从 1 开始的，索引 0 的值指向原始字符串本身 */
    let caps = re.captures(&content).unwrap().get(2).unwrap().as_str();

    if *::MEM_MIN_KEEP > caps.parse::<u64>().unwrap() {
        true
    } else {
        false
    }
}
