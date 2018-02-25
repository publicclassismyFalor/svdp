# 接口规范
## 使用基于 tcp 的 jsonrpc 2.0 风格，样例如下：
    
#### 无子项目的请求:
{"method":"sv_ecs","params":{"item":["cpu"],"instance_id":"i-77777","ts_range":[15000000,1600000],"interval":600},"id":0}
#### 有子项目的请求:
{"method":"sv_ecs","params":{"item":["disk","rdtps"],"instance_id":"i-77777","ts_range":[15000000,1600000],"interval":600},"id":0}

#### 成功返回:
{"result":[[1519530310,10],...,[1519530390,20]],"id":0}
#### 出错返回：
{"err":"...","id":0}

## 说明
- 仅支持单实例查询，多实例查询可通过并发请求实现； 
- method 可选值有：sv_ecs/sv_rds/sv_slb/sv_memcache/sv_mongodb/sv_redis；    
- params 中的 instance_id 为可选项，不指定则表示查询全量数据（注：与空值不同，即指定了字段，但值为空，将返回错误）；
- params 中的 ts_range 用于指定时间区间，区间前后界限均闭合，仅支持 UNIX 时间戳格式（距 1970-01-01 00:00:00 的秒数）；     
- id 是由请求方指定的，会原样返回。    
