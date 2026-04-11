use std::collections::HashMap;
use std::net::Ipv4Addr;
use std::sync::{Arc};
use std::time::Duration;
use tokio::net::{TcpListener, TcpStream};
use tokio::io::AsyncReadExt;
use tokio::sync::Mutex;
use tokio::time::Instant;
use uuid::Uuid;
use crate::config::Settings;
use crate::constants::SensorConnection;
use crate::mqtt::MqttPublisher;
use crate::packet::{DataPacket, PACKET_TOTAL_SIZE};

type ConnectionMap = Arc<tokio::sync::Mutex<HashMap<Ipv4Addr, SensorConnection>>>;

/// WaveServer 结构体表示服务器实例，它监听指定端口并处理客户端连接。
pub struct WaveServer {
    /// 监听的端口号。
    port: u16,
    /// 存储所有已连接的传感器连接信息。
    connections: ConnectionMap,
}

impl WaveServer {
    /// 创建一个新的 WaveServer 实例。
    ///
    /// # 参数
    ///
    /// * `port` - 服务器监听的端口号。
    pub fn new(port: u16) -> Self {
        Self {
            port,
            // 初始化空连接映射。
            connections: Arc::new(tokio::sync::Mutex::new(HashMap::new())),
        }
    }

    pub fn listen_addr(&self) -> String {
        format!("0.0.0.0:{}", self.port)
    }

    pub async fn run_with_listener(
        &self,
        listener: TcpListener,
        mqtt_client: Arc<Mutex<Arc<MqttPublisher>>>,
        settings: Arc<Settings>,
    ) -> Result<(), Box<dyn std::error::Error>> {
        println!("WaveServer listening on {}", listener.local_addr()?);

        loop {
            // 接受新的客户端连接。
            let (socket, peer_addr) = listener.accept().await?;
            let ip = match peer_addr.ip() {
                std::net::IpAddr::V4(v4) => v4,
                _ => continue, // 如果不是 IPv4 地址，则跳过此连接。
            };

            // 克隆连接映射以便在异步任务中使用。
            let connections = self.connections.clone();
            let mqtt_client_clone = mqtt_client.clone();
            let settings_clone = settings.clone();
            // 在新的异步任务中处理客户端连接。
            tokio::spawn(async move {
                if let Err(e) = handle_client(socket, ip, connections, mqtt_client_clone, settings_clone).await {
                    eprintln!("Client error ({}): {}", ip, e);
                }
            });
        }
    }

    // /// 获取指定 IP 地址传感器的最新波形数据。
    // ///
    // /// # 参数
    // ///
    // /// * `ip` - 指定传感器的 IP 地址。
    // /// * `n` - 要获取的数据点数量。
    // ///
    // /// # 返回值
    // ///
    // /// * `Option<(Vec<f32>, Vec<f32>, Vec<f32>)>` - 如果找到对应的传感器连接，则返回其最新的 X、Y 和 Z 波形数据；否则返回 None。
    // pub fn get_waveforms(&self, ip: Ipv4Addr, n: usize) -> Option<(Vec<f32>, Vec<f32>, Vec<f32>)> {
    //     let coins = self.connections.lock().unwrap();
    //     coins.get(&ip).map(|conn| conn.get_latest_waveforms(n))
    // }

    /// 获取所有已连接传感器的信息。
    ///
    /// # 返回值
    ///
    /// * `Vec<(Ipv4Addr, u32)>` - 包含每个已连接传感器的 IP 地址和传感器 ID 的向量。
    pub async fn get_connected_sensors(&self) -> Vec<(Ipv4Addr, u32)> {
        let coins = self.connections.lock().await;
        coins.iter().map(|(ip, c)| (*ip, c.sensor_id)).collect()
    }
}
/// 处理客户端连接。
///
/// 该函数负责读取客户端发送的数据包，并根据数据包内容更新传感器连接信息。
///
/// # 参数
///
/// * `socket` - 客户端的 TCP 连接。
/// * `ip` - 客户端的 IPv4 地址。
/// * `connections` - 存储所有已连接传感器的映射。
///
/// # 返回值
///
/// * `Result<(), Box<dyn std::error::Error>>` - 如果处理过程中发生错误，则返回错误。
async fn handle_client(
    mut socket: TcpStream,
    ip: Ipv4Addr,
    connections: ConnectionMap,
    mqtt_client: Arc<Mutex<Arc<MqttPublisher>>>,
    settings: Arc<Settings>,
) -> Result<(), Box<dyn std::error::Error>> {
    // 创建一个缓冲区来存储从客户端读取的数据。
    let mut buffer = vec![0u8; 8192];
    // 用于存储未处理完的数据。
    let mut leftover: Vec<u8> = Vec::new();
    // 用于记录每秒接收的数据量
    let mut _bytes_received = 0;
    let mut _bytes_count = 0;
    let mut last_time = Instant::now();

    loop {
        // 从客户端读取数据到缓冲区中。
        let n = socket.read(&mut buffer).await?;
        if n == 0 {
            // 如果读取到的数据长度为 0，表示客户端已断开连接，退出循环。
            break;
        }
        _bytes_received += n;
        _bytes_count += 1;
        if last_time.elapsed() >= Duration::from_secs(1) {
            _bytes_received = 0;
            _bytes_count = 0;
            last_time = Instant::now();
            let mut coins = connections.lock().await;
            coins.entry(ip).or_insert_with(|| SensorConnection::new(ip));
            if let Some(conn) = coins.get_mut(&ip) {
                let (x_data, y_data, z_data) = conn.read_buffers();

                let timestamp = std::time::SystemTime::now()
                    .duration_since(std::time::UNIX_EPOCH)
                    .unwrap()
                    .as_millis() as u64;

                let payload = build_upload_payload(
                    &x_data,
                    &y_data,
                    &z_data,
                    timestamp,
                    settings.sampling_length,
                    &settings.data_upload.company_id,
                    &settings.data_upload.gateway_id,
                    &settings.data_upload.device_id,
                );

                let message_bytes: Vec<u8> = serde_json::to_vec(&payload)?;
                let client = mqtt_client.lock().await;
                client.publish_bytes(&message_bytes).await?;
                let size_kb = message_bytes.len() as f64 / 1024.0;
                println!("上传数据大小: {:.2} KB", size_kb);
                conn.clear_buffers()
            }
        }
        // 将新读取的数据追加到 leftover 中。
        leftover.extend_from_slice(&buffer[..n]);

        // 当 leftover 中的数据长度大于等于一个数据包的总大小时，开始处理数据包。
        while leftover.len() >= PACKET_TOTAL_SIZE {
            let packet_bytes = &leftover[..PACKET_TOTAL_SIZE];
            // 尝试解析数据包。
            if let Some(packet) = DataPacket::parse(packet_bytes) {
                if packet.is_data_packet() {
                    // 获取可变引用的连接映射。
                    let mut coins = connections.lock().await;
                    // 确保连接映射中存在当前 IP 地址的条目。
                    coins.entry(ip).or_insert_with(|| SensorConnection::new(ip));
                    // 更新传感器连接信息。
                    if let Some(conn) = coins.get_mut(&ip) {
                        conn.update_from_packet(&packet);
                    }
                }
                // 移除已处理的数据包。
                leftover.drain(..PACKET_TOTAL_SIZE);
            } else {
                // 如果数据包解析失败，跳过一个字节重新对齐。
                leftover.remove(0);
            }
        }
    }

    // 可选：在连接关闭时进行清理操作。
    // connections.lock().unwrap().remove(&ip);
    Ok(())
}

/// 对数据进行取样，确保包含开始值、结束值、最小值、最大值
fn sample_data(data: &[f32], sampling_length: usize) -> Vec<f32> {
    if data.is_empty() {
        return vec![];
    }
    if data.len() <= sampling_length {
        return data.to_vec();
    }

    // 如果采样数小于4，无法同时包含开始、结束、最小、最大值，直接返回原始数据
    if sampling_length < 4 {
        return data.to_vec();
    }

    let mut result = Vec::with_capacity(sampling_length);

    // 1. 添加开始值
    result.push(data[0]);

    // 2. 找到最小值和最大值的位置
    let mut min_idx = 0;
    let mut max_idx = 0;
    for (i, &val) in data.iter().enumerate().skip(1) {
        if val < data[min_idx] {
            min_idx = i;
        }
        if val > data[max_idx] {
            max_idx = i;
        }
    }

    // 3. 添加结束值
    result.push(data[data.len() - 1]);

    // 4. 在中间部分进行均匀取样（留出2个位置给最小值和最大值）
    let middle_count = sampling_length - 4; // 剩余可用于均匀取样的数量
    let inner_start = 1;
    let inner_end = data.len() - 2;

    if middle_count > 0 && inner_end > inner_start {
        let step = (inner_end - inner_start) as f64 / middle_count as f64;
        for i in 0..middle_count {
            let idx = inner_start + (i as f64 * step) as usize;
            let idx = idx.min(inner_end - 1);
            if result.len() < sampling_length - 2 {
                result.push(data[idx]);
            }
        }
    }

    // 5. 添加最小值和最大值（按原始顺序排列）
    let mut min_max: Vec<(usize, f32)> = vec![(min_idx, data[min_idx]), (max_idx, data[max_idx])];
    min_max.sort_by_key(|&(idx, _)| idx);

    for (_, val) in min_max {
        if result.len() < sampling_length {
            result.push(val);
        }
    }

    // 如果最终结果少于采样数，可能有重复值导致，用原始数据填充
    while result.len() < sampling_length {
        for &val in data.iter().rev() {
            if result.len() >= sampling_length {
                break;
            }
            if !result.contains(&val) {
                result.push(val);
                break;
            }
        }
        // 如果所有值都已包含，添加原始数据中未包含的值
        if result.len() < sampling_length {
            let last_val = *data.last().unwrap();
            if !result.contains(&last_val) {
                result.push(last_val);
            } else {
                break;
            }
        }
    }

    result.truncate(sampling_length);
    result
}

/// 构建上传数据payload
fn build_upload_payload(
    x_data: &[f32],
    y_data: &[f32],
    z_data: &[f32],
    timestamp: u64,
    sampling_length: usize,
    company_id: &str,
    gateway_id: &str,
    device_id: &str,
) -> serde_json::Value {
    let x_sampled = sample_data(x_data, sampling_length);
    let y_sampled = sample_data(y_data, sampling_length);
    let z_sampled = sample_data(z_data, sampling_length);

    let base_timestamp = timestamp / 10 * 10; // 抹去个位

    let mut values = serde_json::Map::new();

    // x轴数据: x1, x2, x3, ...
    for (i, &val) in x_sampled.iter().enumerate() {
        let key = format!("x{}", i + 1);
        let ts = base_timestamp + i as u64 + 1;
        values.insert(key, serde_json::json!({
            "value": val,
            "timestamp": ts
        }));
    }

    // y轴数据: y1, y2, y3, ...
    for (i, &val) in y_sampled.iter().enumerate() {
        let key = format!("y{}", i + 1);
        let ts = base_timestamp + i as u64 + 1;
        values.insert(key, serde_json::json!({
            "value": val,
            "timestamp": ts
        }));
    }

    // z轴数据: z1, z2, z3, ...
    for (i, &val) in z_sampled.iter().enumerate() {
        let key = format!("z{}", i + 1);
        let ts = base_timestamp + i as u64 + 1;
        values.insert(key, serde_json::json!({
            "value": val,
            "timestamp": ts
        }));
    }

    serde_json::json!({
        "requestId": Uuid::new_v4().to_string(),
        "timestamp": timestamp,
        "data": {
            "companyId": company_id,
            "gatewayId": gateway_id,
            "deviceId": device_id,
            "values": values
        }
    })
}


