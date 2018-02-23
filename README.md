# 接口规范
## 使用基于 tcp 的 jsonrpc 2.0 风格，样例如下：
#### 请求:
{"method":"sv_ecs","params":{"instance_id":"i-123456","ts_range":[15000000,1600000]},"id":0}

#### 成功返回:
{"result":["ts":1519379068,"data":{...}],"id":0}
#### 出错返回：
{"err":"...","id":0}

- 目前仅支持全量查询与单实例查询，多实例查询可通过并发请求实现；
- method 可选值有：sv_ecs/sv_rds/sv_slb/sv_memcache/sv_mongodb/sv_redis；    
- params 留空表示查询全量数据；
- ts_range 用于指定时间区间，区间前后界限均闭合，仅支持 UNIX 时间戳格式（距 1970-01-01 00:00:00 的秒数）；     
- id 是由请求方指定的，会原样返回；    
