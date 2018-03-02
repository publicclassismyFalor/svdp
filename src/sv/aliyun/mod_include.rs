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

fn http_req(argv: Vec<[String; 2]>) -> Result<Vec<u8>, reqwest::Error> {
    let domain = &argv[0][1];

    let mut params = vec![];
    for i in 1..argv.len() {
        params.push(argv[i].clone());
    }

    params.push(["AccessKeyId".to_owned(), ACCESSID.to_owned()]);
    params.push(["SignatureMethod".to_owned(), "HMAC-SHA1".to_owned()]);
    params.push(["SignatureVersion".to_owned(), "1.0".to_owned()]);
    params.push(["SignatureNonce".to_owned(), ::rand::thread_rng().gen::<i32>().to_string()]);
    params.push(["Format".to_owned(), "JSON".to_owned()]);
    params.push(["Timestamp".to_owned(), strftime("%Y-%m-%dT%H:%M:%SZ", &now_utc()).unwrap()]);
    params.sort();

    let mut mid_str = String::new();
    let last_id = params.len() - 1;

    for i in 0..last_id {
        mid_str.push_str(&byte_serialize(params[i][0].as_bytes()).collect::<String>());
        mid_str.push_str("=");
        mid_str.push_str(&byte_serialize(params[i][1].as_bytes()).collect::<String>());
        mid_str.push_str("&");
    }
    mid_str.push_str(&byte_serialize(params[last_id][0].as_bytes()).collect::<String>());
    mid_str.push_str("=");
    mid_str.push_str(&byte_serialize(params[last_id][1].as_bytes()).collect::<String>());

    let str_to_sig = format!("GET&%2F&{}", byte_serialize(mid_str.as_bytes()).collect::<String>());

    let mid_str = mid_str.replace("+", "%20").replace("*", "%2A").replace("%7E", "~");
    let str_to_sig = str_to_sig.replace("+", "%20").replace("*", "%2A").replace("%7E", "~");

    let sigkey = hmac::SigningKey::new(&digest::SHA1, SIGKEY.as_bytes());
    let sig = hmac::sign(&sigkey, str_to_sig.as_bytes());

    let final_url_sig = byte_serialize(BASE64.encode(sig.as_ref()).as_bytes()).collect::<String>();
    let final_url_sig = final_url_sig.replace("+", "%20").replace("*", "%2A").replace("%7E", "~");

    let mut requrl = format!("http://{}?", domain);
    requrl.push_str(&mid_str);
    requrl.push_str("&");
    requrl.push_str("Signature");
    requrl.push_str("=");
    requrl.push_str(&final_url_sig);

    let mut resp = SV_CLIENT.get(&requrl).send()?;
    let mut ret = vec![];
    match resp.status() {
        reqwest::StatusCode::Ok => {
            if let Err(e) = resp.read_to_end(&mut ret) {
                err!(e);
            }
        },
        s => err!(s)
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
