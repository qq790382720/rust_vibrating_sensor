# Project: rust_vibrating_sensor

振动传感器数据采集服务器，使用 Rust 开发。

## 项目概述

- **类型**: 物联网(IoT)数据采集后端服务
- **功能**: 接收传感器TCP连接，解析振动数据包，通过MQTT转发数据，提供Web配置界面

## 技术栈

| 组件 | 技术 |
|------|------|
| Web框架 | Axum 0.8.7 |
| 异步运行时 | Tokio (full features) |
| MQTT客户端 | mqtt-endpoint-tokio 0.6.0 |
| 序列化 | Serde |
| 配置格式 | TOML |
| 日志 | Tracing + EnvLogger |
| Web服务 | Tower-HTTP (CORS, Static Files, Trace) |
| 前端 | Vue 3 (位于 `web_ui/vibrating-config/`) |

## 项目结构

```
rust_vibrating_sensor/
├── src/
│   ├── main.rs        # 主入口，Axiom HTTP服务器 + TCP传感器服务器
│   ├── server.rs      # WaveServer TCP监听器，处理传感器连接
│   ├── packet.rs      # DataPacket二进制数据包解析 (1416 bytes)
│   ├── mqtt.rs        # MqttPublisher MQTT发布客户端，带自动重连
│   ├── config.rs      # Settings配置结构体 (TOML解析)
│   └── constants.rs  # SensorConnection传感器连接管理
├── assetsa/          # 编译后的Web UI静态资源
├── web_ui/           # Vue 3前端源码 (vibrating-config)
├── Settings.toml     # 配置文件
└── Cargo.toml
```

## 核心模块

### server.rs - WaveServer
- 监听TCP端口接收传感器数据
- 解析二进制DataPacket数据包
- 数据包格式: 头(24字节) + 数据(116样本×3轴×4字节) + 尾(4字节) = 1416字节
- 维护传感器连接映射和环形缓冲区

### mqtt.rs - MqttPublisher
- MQTT v3.1.1客户端
- 自动重连机制 (指数退避: 1s-30s)
- 心跳保活 (PINGREQ/PINGRESP)
- 线程安全Arc+RwLock

### packet.rs - DataPacket
- 包类型: `PUDT` (数据包)
- 字段: sensor_id, fs(采样率), temperature, pack_counter, data[116][3]
- 字节序: Little Endian

### main.rs - HTTP API
- `GET /read-settings` - 读取Settings.toml
- `POST /write-settings` - 写入Settings.toml
- `GET /system_restart` - 重启系统
- `GET /assets/...` - 静态文件服务

## 配置 (Settings.toml)

```toml
port = 22009                    # TCP传感器监听端口

[mqtt_server]
url = "127.0.0.1:1883"          # MQTT Broker地址
public_topic = "topic/tests"    # MQTT主题
client_id = "scada01"           # MQTT客户端ID
```

## 运行

```bash
# 开发
cargo run

# 构建
cargo build --release
```

**注意**: 需要先部署 `web_ui/vibrating-config` 并将构建结果复制到 `assetsa/`

## 端口

- `22009`: TCP传感器数据入口
- `8082`: HTTP Web API和静态文件服务

## 推送数据给客户
采用json格式
{
    "requestId": "5661e8e8-2d43-11f0-865a-84699353769a",
    "timestamp": 1746842768380,
    "data": {
        "companyId": "cbd9ef26db814b58aa33fb0457eca8af",
        "gatewayId": "gfwg",
        "deviceId": "weldingrobot_d15",
        "values": {
            "status": {
                "value": "作业",
                "timestamp": 1746842768380
            },
            "part_cnt": {
                "value": 47,
                "timestamp": 1746842768380
            },
            "cyc_sec": {
                "value": 894,
                "timestamp": 1746842768380
            }
        }
    }
}