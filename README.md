# rust_vibrating_sensor

Rust 编写的振动传感器数据采集网关，负责接收传感器 TCP 数据包、解析波形数据、采样后转成 JSON，并发布到 MQTT Broker，同时提供 Web 配置界面和基础运维接口。

## 功能概览

- 监听振动传感器 TCP 连接，默认端口 `22009`
- 解析 `1416` 字节二进制数据包
- 对 X/Y/Z 三轴数据按 `sampling_length` 进行采样
- 按统一 JSON 格式发布到 MQTT
- 提供配置读写、NTP 同步、设备重启接口
- 提供 Web UI，默认 HTTP 端口 `8082`
- 支持使用 Docker 交叉编译到 `armv7-unknown-linux-musleabihf`

## 项目结构

```text
.
├── src/
│   ├── main.rs                  # HTTP 服务入口 + 配置接口
│   ├── server.rs                # 传感器 TCP 服务、波形采样、上传 JSON 构建
│   ├── mqtt.rs                  # MQTT 连接与自动重连
│   ├── packet.rs                # 1416 字节传感器数据包解析
│   ├── config.rs                # Settings.toml 解析
│   └── constants.rs             # 传感器连接与缓冲区结构
├── ui/                          # 已构建的前端静态文件
├── web_ui/vibrating-config/     # Vue 3 前端源码
├── scripts/build-armv7-musl.sh  # ARMv7 musl 交叉编译脚本
├── docs/CROSS_COMPILE_ARM32.md  # 交叉编译说明
├── Settings.toml                # 运行配置
└── Cargo.toml
```

## 数据流

```text
Sensor TCP -> rust_vibrating_sensor -> MQTT Broker -> Upstream Platform
                  |
                  +-> HTTP API + Web UI
```

处理流程如下：

1. 传感器向网关发送 `1416` 字节数据包
2. 服务解析出三轴波形和温度等信息
3. 根据 `sampling_length` 对三轴数据做采样
4. 组装成平台侧需要的 JSON 后发布到 MQTT 主题

## 配置文件

编辑 [Settings.toml](/Users/xiaosan/WorkCode/Rust/rust_vibrating_sensor/Settings.toml)：

```toml
port = 22009
sampling_length = 100

[mqtt_server]
url = "127.0.0.1:1883"
public_topic = "topic/tests"
client_id = "scada01"
username = "ceshi"
password = "123"

[data_upload]
company_id = "cbd9ef26db814b58aa33fb0457eca8af"
gateway_id = "gfwg"
device_id = "weldingrobot_d15"

[ntp_server]
server = "ntp.aliyun.com"
```

配置项说明：

- `port`: 传感器 TCP 监听端口
- `sampling_length`: 每个轴最终上传的采样点数
- `mqtt_server.url`: MQTT Broker 地址
- `mqtt_server.public_topic`: MQTT 发布主题
- `mqtt_server.client_id`: MQTT 客户端 ID
- `mqtt_server.username` / `mqtt_server.password`: MQTT 认证信息
- `data_upload.*`: 上报平台所需的企业、网关、设备标识
- `ntp_server.server`: NTP 时间同步服务器

## 本地运行

开发模式：

```bash
cargo run
```

发布构建：

```bash
cargo build --release
./target/release/rust_vibrating_sensor
```

启动后可访问：

- Web UI: `http://127.0.0.1:8082/ui/`
- 根路径也会回退到前端首页: `http://127.0.0.1:8082/`

## Web API

| Endpoint | Method | Description |
|----------|--------|-------------|
| `/read-settings` | `GET` | 读取当前 `Settings.toml` |
| `/write-settings` | `POST` | 写入配置文件 |
| `/ntp-sync` | `POST` | 触发 NTP 时间同步 |
| `/system_restart` | `GET` | 执行设备重启 |
| `/ui/*` | `GET` | Web 配置页面 |
| `/` | `GET` | 回退到前端首页 |

## MQTT 上报格式

程序实际发布的 JSON 格式如下：

```json
{
  "requestId": "f3f4f2d6-0000-0000-0000-000000000000",
  "timestamp": 1746842768380,
  "data": {
    "companyId": "cbd9ef26db814b58aa33fb0457eca8af",
    "gatewayId": "gfwg",
    "deviceId": "weldingrobot_d15",
    "values": {
      "x1": { "value": 0.12, "timestamp": 1746842768381 },
      "x2": { "value": 0.08, "timestamp": 1746842768382 },
      "y1": { "value": -0.03, "timestamp": 1746842768381 },
      "z1": { "value": 0.21, "timestamp": 1746842768381 }
    }
  }
}
```

说明：

- `requestId` 为运行时生成的 UUID
- `timestamp` 为本次上传时间戳
- `values` 中会生成 `x1..xN`、`y1..yN`、`z1..zN`
- `N` 由 `sampling_length` 决定

## 前端说明

- 前端源码位于 [web_ui/vibrating-config](/Users/xiaosan/WorkCode/Rust/rust_vibrating_sensor/web_ui/vibrating-config)
- 运行时静态资源位于 [ui](/Users/xiaosan/WorkCode/Rust/rust_vibrating_sensor/ui)
- 更新前端后，需要重新构建并把产物同步到 `ui/`

## 交叉编译到 ARMv7 musl

仓库已提供 Docker 编译入口：

```bash
./scripts/build-armv7-musl.sh
```

默认输出：

```bash
./target/armv7-unknown-linux-musleabihf/release/rust_vibrating_sensor
```

当前脚本会自动识别本地这两个镜像标签之一：

- `rust-musl-cross:armv7-musleabihf`
- `messense/rust-musl-cross:armv7-musleabihf`

如果你本地镜像使用其他标签：

```bash
RUST_MUSL_CROSS_IMAGE=<your-local-tag> ./scripts/build-armv7-musl.sh
```

更详细说明见 [docs/CROSS_COMPILE_ARM32.md](/Users/xiaosan/WorkCode/Rust/rust_vibrating_sensor/docs/CROSS_COMPILE_ARM32.md)。

## 端口说明

| Port | Usage |
|------|-------|
| `22009` | 传感器 TCP 数据入口 |
| `8082` | HTTP API 与 Web UI |
| `1883` | 默认 MQTT Broker 端口 |

## 常见问题

端口占用：

- 如果启动时报 `Address already in use`，请检查 `22009` 或 `8082` 是否已被其他进程占用
- 当前程序会明确提示是 `Sensor TCP server` 还是 `HTTP server` 绑定失败

MQTT 连接：

- 程序会保持 MQTT 长连接
- 只有真正发送失败或连接丢失时才会进入重连流程
