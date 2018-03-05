# 版本更新
## v0.2.5
- 修复：缓存命中率为零的情况，会返回 result:[null, null] 的问题
## v0.2.4
- 使用 rust 原生代码代替 aliyun SDK，以提升数据同步效率
## v0.2.3
- 修复：缓存动态更新逻辑漏洞
## v0.2.2
- 返回结果中的 key 值恢复使用 UNIX 时间戳格式
## v0.2.1
- 用户指定的时间间隔会被自动修正为与 300s 对齐，如 700s 会被修正为 600s
- 低于 300s 的会自动提升为 300s
## v0.2.0
- 启用缓存机制，提升响应速度
- 所有项目的原始数据，统一使用 300 秒间隔
- 300 秒间隔以上的数据响应，是对原始数据的分层抽样结果，并非平均值
- 返回的 [[key...],[value...]] 结果中 key，已经转换成可读的时间格式，形如：02-28 08:30:00
- 键、值均不存在的情况，将返回空数据 [[],[]]
- 请求的数据项目或设备不存在时，将返回正常的键，但值会被统一置为 -1，以与常规的 0 值区分，形如：[[1519829958, ...],[-1, ...]]
       
# 接口规范
## 使用基于 tcp 或 http/POST(请求信息作为 body) 的 jsonrpc 2.0 风格，样例如下：
    
#### 无子项目的请求:
{"method":"sv_ecs","params":{"item":["cpu_ratio",null,null],"instance_id":"i-77777","ts_range":[15000000,1600000],"interval":600},"id":0}
#### 有子项目的请求:
{"method":"sv_ecs","params":{"item":["disk","/dev/vda1","rdtps"],"instance_id":"i-77777","ts_range":[15000000,1600000],"interval":600},"id":0}

#### 成功返回:
{"result":[[1519530310, ... ,1519530390],[10, ... ,20]],"id":0}
#### 出错返回：
{"err":"error msg","id":0}

## 说明
- 仅支持单实例查询，多实例查询可通过并发请求实现； 
- method 可选值有：sv_ecs/sv_rds/sv_slb/sv_memcache/sv_mongodb/sv_redis；    
- params 中的 item 用于指定要查询的数据项目，除 ecs 中的 disk/netif 两项需要全部三个数组成员外，只需要填充第一个成员，后两上成员置为 null 即可；
- params 中的 instance_id 用于指定实例 ID；
- params 中的 ts_range 用于指定时间区间，区间前后界限均闭合，仅支持 UNIX 时间戳格式（距 1970-01-01 00:00:00 的秒数）；     
- params 中的 interval 用于指定数据样本的时间间隔，可以为任意能被 15 整除的非负整数，单位：秒；
- id 是由请求方指定的，会原样返回。    

## item 说明
#### 注：所有以 ratio 为后缀的指标，含义均为 使用率百分值 * 1000
    
#### sv_ecs
```
  "cpu_ratio": 85
  "mem_ratio": 184
  "load5m": 30  // 原始值 * 1000
  "load15m": 10  // 同上
  "tcp": 57

  "disk": {
    "/dev/vda1": {
      "rd": 103  // 单位：KB
      "wr": 12  // 单位：KB
      "ratio": 100
      "rdtps": 0
      "wrtps": 0
    }
  }

  "netif": {
    "47.88.189.118": {
      "rd": 1  // 单位：KB
      "wr": 5  // 单位：KB
      "rdtps": 11
      "wrtps": 8
    }
  }
```
#### sv_slb
```
  "rd": 0  // 单位：KB
  "wr": 0  // 单位：KB
  "conn": 0  // 连接总数
  "rdtps": 1
  "wrtps": 1
```
#### sv_rds
```
  "delay": 0  // 单位：秒
  "cpu_ratio": 0
  "mem_ratio": 93
  "conn_ratio": 0
  "disk_ratio": 29
  "disktps_ratio": 0
```
#### sv_mongodb
```
  "rd": 4817  // 单位：KB
  "wr": 30746  // 单位：KB
  "cpu_ratio": 45
  "mem_ratio": 413
  "conn_ratio": 2
  "disk_ratio": 327
  "disktps_ratio": 0
```
#### sv_memcache
```
  "rd": 19  // 单位：KB
  "wr": 126 // 单位：KB
  "cpu_ratio": 13
  "mem_ratio": 720
  "conn_ratio": 0
```
#### sv_redis
```
  "rd": 19  // 单位：KB
  "wr": 126 // 单位：KB
  "cpu_ratio": 13
  "mem_ratio": 720
  "conn_ratio": 0
```
