# VTall-T16E-A Vibration Sensor Gateway

A Rust-based IoT gateway for collecting vibration sensor data and uploading to MQTT.

## Sensor

![VTall-T16E-A](/Users/xiaosan/Library/Containers/com.tencent.xinWeChat/Data/Documents/xwechat_files/wxid_qaumxkjsr89722_9525/temp/RWTemp/2026-03/59e83feea4d2bd3c5be270310b897db1.jpg)

**VTall T16E-A** - Industrial triaxial vibration sensor with built-in temperature measurement.

## How It Works

```
VTall-T16E-A  →  TCP:22009  →  Gateway  →  MQTT Broker  →  Cloud
```

1. Sensor connects via TCP and sends 1416-byte binary data packets (116 samples × 3 axes)
2. Gateway parses packets, samples data, and converts to JSON
3. Data is published to MQTT broker

## Configuration

Edit `Settings.toml`:

```toml
port = 22009                    # TCP listening port

[mqtt_server]
url = "127.0.0.1:1883"          # MQTT broker
public_topic = "topic/tests"    # MQTT topic
client_id = "scada01"

[data_upload]
company_id = "your-company-id"
gateway_id = "your-gateway-id"
device_id = "your-device-id"

[ntp_server]
server = "ntp.aliyun.com"       # NTP sync
```

## Build & Run

```bash
cargo build --release
./target/release/rust_vibrating_sensor
```

## Cross Compile for ARMv7 musl

Use the local Docker image `rust-musl-cross:armv7-musleabihf`.
If your local tag is `messense/rust-musl-cross:armv7-musleabihf`, the script will pick it up automatically.

```bash
./scripts/build-armv7-musl.sh
```

Output:

```bash
./target/armv7-unknown-linux-musleabihf/release/rust_vibrating_sensor
```

If your local image uses another tag, override it temporarily:

```bash
RUST_MUSL_CROSS_IMAGE=<your-local-tag> ./scripts/build-armv7-musl.sh
```

## HTTP API

| Endpoint | Method | Description |
|----------|--------|-------------|
| `/read-settings` | GET | Read settings |
| `/write-settings` | POST | Write settings |
| `/ntp-sync` | POST | Sync NTP time |
| `/ui/*` | GET | Web configuration UI |

## Ports

| Port | Usage |
|------|-------|
| 22009 | TCP sensor data |
| 8082 | HTTP API & Web UI |

## Tech Stack

Rust · Tokio · Axum · MQTT v3.1.1
