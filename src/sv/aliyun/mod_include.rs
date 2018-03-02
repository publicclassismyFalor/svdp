pub trait DATA {
    type Holder;

    fn argv_new(&self) -> Vec<[String; 2]>;

    fn get(&self, holder: Self::Holder) {
        let mut argv = self.argv_new();

        let (tx, rx) = mpsc::channel();

        thread::spawn(move || {
            if let Ok(ret) = http_req(argv.clone()) {
                let v = serde_json::from_slice(&ret).unwrap_or(Value::Null);
                if Value::Null == v {
                    return;
                }

                tx.send(ret).unwrap();

                if let Value::String(ref cursor) = v["Cursor"] {
                    argv.push(["Cursor".to_owned(), (*cursor).clone()]);

                    while let Ok(ret) = http_req(argv.clone()) {
                        let v = serde_json::from_slice(&ret).unwrap_or(Value::Null);
                        if Value::Null == v {
                            return;
                        }

                        tx.send(ret).unwrap();

                        if let Value::String(ref cursor) = v["Cursor"] {
                            argv.pop();
                            argv.push(["Cursor".to_owned(), (*cursor).clone()]);
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




pub fn argv_new_base() -> Vec<[String; 2]> {
    let mut argv = vec![
        ["Domain".to_owned(), "metrics.aliyuncs.com".to_owned()],
        ["Version".to_owned(), "2017-03-01".to_owned()],
        ["Format".to_owned(), "JSON".to_owned()],
        ["Timestamp".to_owned(), strftime("%Y-%m-%dT%H:%M:%SZ", &now_utc()).unwrap()],
        ["SignatureMethod".to_owned(), "HMAC-SHA1".to_owned()],
        ["SignatureVersion".to_owned(), "1.0".to_owned()],
        ["SignatureNonce".to_owned(), ::rand::thread_rng().gen::<i32>().to_string()],
        ["Action".to_owned(), "QueryMetricList".to_owned()],
        ["Length".to_owned(), "1000".to_owned()],
    ];

    unsafe {
        argv.push(["StartTime".to_owned(), BASESTAMP.to_string()]);
        argv.push(["EndTime".to_owned(), (BASESTAMP + INTERVAL).to_string()]);
    }

    argv
}

fn get_region() -> Option<Vec<String>> {
    let mut res: Vec<String> = Vec::new();
    let argv = vec![
        ["Domain".to_owned(), "ecs.aliyuncs.com".to_owned()],
        ["Version".to_owned(), "2014-05-26".to_owned()],
        ["Action".to_owned(), "DescribeRegions".to_owned()],
    ];

    if let Ok(ret) = http_req(argv) {
        let v: Value = serde_json::from_slice(&ret).unwrap_or(Value::Null);
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

fn http_req(mut argv: Vec<[String; 2]>) -> Result<Vec<u8>, reqwest::Error> {
    argv.push(["AccessKeyId".to_owned(), ACCESSID.to_owned()]);

// TODO

    let mut ret = vec![];
    if let Err(e) = SV_CLIENT.get("...FIXME...").send()?.read(&mut ret) {
        err!(e);
    }

    Ok(ret)
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

fn start_serv() {
    if None != ::CONF.sv_http_addr {
        thread::spawn(|| serv::http_serv());
    }

    if None != ::CONF.sv_tcp_addr {
        thread::spawn(|| serv::tcp_serv());
    }
}
