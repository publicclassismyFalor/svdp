# 接口规范
## 使用基于 tcp 或 http/POST(请求信息作为 body) 的 jsonrpc 2.0 风格，样例如下：
    
#### 无子项目的请求:
{"method":"sv_ecs","params":{"item":["cpu_ratio",null,null],"instance_id":"i-77777","ts_range":[15000000,1600000],"interval":600},"id":0}
#### 有子项目的请求:
{"method":"sv_ecs","params":{"item":["disk","/dev/vda1","rdtps"],"instance_id":"i-77777","ts_range":[15000000,1600000],"interval":600},"id":0}

#### 成功返回:
{"result":[[1519530310,10],...,[1519530390,20]],"id":0}
#### 出错返回：
{"err":"...","id":0}

## 说明
- 仅支持单实例查询，多实例查询可通过并发请求实现； 
- method 可选值有：sv_ecs/sv_rds/sv_slb/sv_memcache/sv_mongodb/sv_redis；    
- params 中的 item 用于指定要查询的数据项目，除 ecs 中的 disk/netif 两项需要全部三个数组成员外，只需要填充第一个成员，后两上成员置为 null 即可；
- params 中的 instance_id 用于指定实例 ID；
- params 中的 ts_range 用于指定时间区间，区间前后界限均闭合，仅支持 UNIX 时间戳格式（距 1970-01-01 00:00:00 的秒数）；     
- params 中的 interval 用于指定数据样本的时间间隔，可以为任意能被 15 整除的非负整数，单位：秒；
- id 是由请求方指定的，会原样返回。    

#### sv_ecs
```
{
  "i-22vb6jnml": {
    "tcp": 57,
    "disk": {
      "/dev/vda1": {
        "rd": 103,
        "wr": 12,
        "ratio": 100,  // 磁盘使用率百分值 * 1000
        "rdtps": 0,
        "wrtps": 0
      }
    },
    "netif": {
      "10.45.65.169": {
        "rd": 0,
        "wr": 0,
        "rdtps": 0,
        "wrtps": 0
      },
      "47.88.189.118": {
        "rd": 1,
        "wr": 5,
        "rdtps": 11,
        "wrtps": 8
      }
    },
    "load5m": 30,  // 原始值 * 1000
    "load15m": 10,  // 同上
    "cpu_ratio": 85,  // 使用率百分值 * 1000
    "mem_ratio": 184  // 同上
  }
}
```
