use ::sv::aliyun;

/// REQ example:
/// {"method":"Sv_ecs","params":{"item":["disk","/dev/vda1","rdtps"],"ts_range":[15000000,1600000],"standard":"<5"},"id":0}
/// {"method":"Sv_ecs","params":{"item":["cpu_ratio",null,null],"ts_range":[15000000,1600000],"standard":">80"},"id":0}
///
/// RES example:
/// {"result":[[i-abcdefg,i-123456,...],...,[1519530390,20]],"id":0}
/// OR
/// {"err":"...","id":0}




pub fn worker(_body: &str) -> Result<(String, i32), (String, i32)> {
    Ok(("".to_owned(), 0))
}
