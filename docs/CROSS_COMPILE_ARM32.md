# Rust 项目交叉编译 ARM32 静态二进制

## 编译环境

- 宿主机: macOS (Apple Silicon)
- 目标平台: Linux ARM32 (ARMv7)
- C 库: musl (无 glibc 依赖)
- 编译工具: Docker + `rust-musl-cross:armv7-musleabihf`

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

### 4. 准备本地交叉编译镜像

```bash
docker image inspect rust-musl-cross:armv7-musleabihf || \
docker image inspect messense/rust-musl-cross:armv7-musleabihf
```

如果你本地镜像标签是 `messense/rust-musl-cross:armv7-musleabihf`，项目脚本也会自动识别。
如果是其他标签，可以在执行脚本时通过 `RUST_MUSL_CROSS_IMAGE` 覆盖。

## 推荐方式: 使用项目脚本

项目已经提供了可复用入口：

```bash
./scripts/build-armv7-musl.sh
```

如果你本地镜像标签不同：

```bash
RUST_MUSL_CROSS_IMAGE=<your-local-tag> ./scripts/build-armv7-musl.sh
```

脚本会自动完成：

1. 检查 Docker daemon 是否已启动
2. 检查本地是否存在支持的镜像标签
3. 在容器内执行 `cargo build --release --target armv7-unknown-linux-musleabihf`
4. 校验产物是否为 ARM 静态链接二进制

输出位置:

```
target/armv7-unknown-linux-musleabihf/release/rust_vibrating_sensor
```

## 手动 Docker 命令

如果你想直接执行 Docker 命令，也可以使用下面的等价写法：

```bash
docker run --rm \
  -v "$(pwd)":/home/rust/src \
  -w /home/rust/src \
  messense/rust-musl-cross:armv7-musleabihf \
  cargo build --release --target armv7-unknown-linux-musleabihf
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

### Q: 本地镜像标签不同或不存在

**解决**:

```bash
docker images
RUST_MUSL_CROSS_IMAGE=<your-local-tag> ./scripts/build-armv7-musl.sh
```

### Q: 编译报错 "toolchain may not be able to run on this system"

**解决**: 使用项目脚本或 Docker 方法，避免直接在 macOS 上本机链接 musl 目标。

### Q: 二进制在目标系统上无法运行

**解决**:
1. 确认目标系统是 ARM32 (非 ARM64)
2. 确认使用 musl 静态链接
3. 检查文件系统权限

## 编译输出信息

- 二进制大小: ~4.8MB
- 链接方式: 静态链接 (musl libc)
- 优化级别: release (-O3)
