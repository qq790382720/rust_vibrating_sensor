# Rust 项目交叉编译 ARM32 静态二进制

## 编译环境

- 宿主机: macOS (Apple Silicon)
- 目标平台: Linux ARM32 (ARMv7)
- C 库: musl (无 glibc 依赖)
- 编译工具: Docker + messense/rust-musl-cross

## 前置条件

### 1. 安装 Rust

```bash
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh
source ~/.cargo/env
rustc --version
```

### 2. 安装 Docker

```bash
# macOS
brew install --cask docker

# 启动 Docker Desktop
open -a Docker
```

### 3. 验证 Docker 运行

```bash
docker run --rm hello-world
```

## 添加 ARM32 目标平台

```bash
rustup target add armv7-unknown-linux-musleabihf
```

## 编译步骤

### 方法一: 使用 Docker + messense/rust-musl-cross (推荐)

1. **拉取交叉编译镜像**

```bash
docker pull messense/rust-musl-cross:armv7-musleabihf
```

2. **执行交叉编译**

```bash
docker run --rm \
  -v "$(pwd)":/code \
  -w /code \
  messense/rust-musl-cross:armv7-musleabihf \
  cargo build --release --target armv7-unknown-linux-musleabihf
```

3. **输出位置**

```
target/armv7-unknown-linux-musleabihf/release/rust_vibrating_sensor
```

### 方法二: 使用 cross 工具

```bash
# 安装 cross
cargo install cross

# 执行交叉编译
cross build --release --target armv7-unknown-linux-musleabihf
```

## 验证二进制

### 1. 检查文件类型

```bash
file target/armv7-unknown-linux-musleabihf/release/rust_vibrating_sensor
```

输出应包含:
```
ELF 32-bit LSB executable, ARM, EABI5 version 1 (SYSV), statically linked
```

### 2. 检查是否静态链接

```bash
# 在 Linux 系统上执行
ldd target/armv7-unknown-linux-musleabihf/release/rust_vibrating_sensor
```

输出应包含:
```
not a dynamic executable
```

## 部署到 BuildRoot 系统

1. **复制二进制文件**

```bash
scp target/armv7-unknown-linux-musleabihf/release/rust_vibrating_sensor root@<设备IP>:/usr/local/bin/
```

2. **设置执行权限**

```bash
ssh root@<设备IP> chmod +x /usr/local/bin/rust_vibrating_sensor
```

3. **验证运行**

```bash
ssh root@<设备IP> /usr/local/bin/rust_vibrating_sensor --version
```

## 常见问题

### Q: Docker 镜像拉取失败

**问题**: `net/http: request canceled while waiting for connection`

**解决**:
```bash
# 检查网络
ping docker.io

# 重试拉取
docker pull messense/rust-musl-cross:armv7-musleabihf
```

### Q: 编译报错 "toolchain may not be able to run on this system"

**解决**: 使用 `cross` 工具或 Docker 方法，避免直接在 macOS 上交叉编译。

### Q: 二进制在目标系统上无法运行

**解决**:
1. 确认目标系统是 ARM32 (非 ARM64)
2. 确认使用 musl 静态链接
3. 检查文件系统权限

## 其他目标平台

| 平台 | Target | Docker 镜像 |
|------|--------|-------------|
| ARM64 | aarch64-unknown-linux-musleabihf | messense/rust-musl-cross:aarch64-musleabihf |
| ARMv7 | armv7-unknown-linux-musleabihf | messense/rust-musl-cross:armv7-musleabihf |
| x86_64 | x86_64-unknown-linux-musl | messense/rust-musl-cross:x86_64-musl |

## 编译输出信息

- 二进制大小: ~4.8MB
- 链接方式: 静态链接 (musl libc)
- 优化级别: release (-O3)
