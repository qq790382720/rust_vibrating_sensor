use std::collections::VecDeque;
use std::net::Ipv4Addr;
use std::sync::{Arc, Mutex};
use crate::packet::DataPacket;

/// 定义传感器连接的最大缓冲深度。
pub const MAX_BUF_DEPTH: usize = 20_000;

/// 传感器连接结构体，用于存储和管理传感器的相关信息。
pub struct SensorConnection {
    /// 传感器的 IPv4 地址。
    pub _ip: Ipv4Addr,
    /// 传感器的 ID。
    pub sensor_id: u32,
    /// 传感器的采样率。
    pub sample_rate: f32,
    /// 传感器的温度。
    pub temperature: f32,
    /// 上一个数据包的计数器值。
    pub last_pack_counter: u32,

    // 双缓冲或环形缓冲（简化为 VecDeque）
    /// X 轴数据缓冲区。
    pub x_buffer: Arc<Mutex<VecDeque<f32>>>,
    /// Y 轴数据缓冲区。
    pub y_buffer: Arc<Mutex<VecDeque<f32>>>,
    /// Z 轴数据缓冲区。
    pub z_buffer: Arc<Mutex<VecDeque<f32>>>,
}

impl SensorConnection {
    /// 创建一个新的传感器连接实例。
    ///
    /// # 参数
    ///
    /// * `ip` - 传感器的 IPv4 地址。
    pub fn new(ip: Ipv4Addr) -> Self {
        Self {
            _ip:ip,
            sensor_id: 0,
            sample_rate: 0.0,
            temperature: 0.0,
            last_pack_counter: 0,
            x_buffer: Arc::new(Mutex::new(VecDeque::with_capacity(MAX_BUF_DEPTH))),
            y_buffer: Arc::new(Mutex::new(VecDeque::with_capacity(MAX_BUF_DEPTH))),
            z_buffer: Arc::new(Mutex::new(VecDeque::with_capacity(MAX_BUF_DEPTH))),
        }
    }
    /// 读取缓冲区中的数据。
   pub fn read_buffers(&self) -> (Vec<f32>, Vec<f32>, Vec<f32>) {
        let read_buffer = |buf: &Arc<Mutex<VecDeque<f32>>>| -> Vec<f32> {
            let b = buf.lock().unwrap();
            b.iter().cloned().collect()
        };
        (read_buffer(&self.x_buffer), read_buffer(&self.y_buffer), read_buffer(&self.z_buffer))
    }
    /// 根据接收到的数据包更新传感器连接信息。
    ///
    /// # 参数
    ///
    /// * `packet` - 接收到的数据包。
    pub fn update_from_packet(&mut self, packet: &DataPacket) {
        self.sensor_id = packet.sensor_id;
        self.sample_rate = packet.fs;
        self.temperature = packet.temperature;
        self.last_pack_counter = packet.pack_counter;

        // 推入数据（注意容量限制）
        let push_safe = |buf: &Arc<Mutex<VecDeque<f32>>>, values: &[f32]| {
            let mut b = buf.lock().unwrap();
            for &v in values {
                if b.len() >= MAX_BUF_DEPTH {
                    b.pop_front(); // 丢弃最旧的数据
                }
                b.push_back(v);
            }
        };

        let (mut xs, mut ys, mut zs) = (Vec::new(), Vec::new(), Vec::new());
        for sample in &packet.data {
            xs.push(sample[0]);
            ys.push(sample[1]);
            zs.push(sample[2]);
        }

        push_safe(&self.x_buffer, &xs);
        push_safe(&self.y_buffer, &ys);
        push_safe(&self.z_buffer, &zs);
    }
    
    /// 清空所有缓冲区。
    pub fn clear_buffers(&self) {
        let clear_buffer = |buf: &Arc<Mutex<VecDeque<f32>>>| {
            let mut b = buf.lock().unwrap();
            b.clear();
        };
        clear_buffer(&self.x_buffer);
        clear_buffer(&self.y_buffer);
        clear_buffer(&self.z_buffer);
    }
    // /// 获取最新的波形数据。
    // ///
    // /// # 参数
    // ///
    // /// * `n` - 要获取的数据点数量。
    // ///
    // /// # 返回值
    // ///
    // /// * `(Vec<f32>, Vec<f32>, Vec<f32>)` - 最新的 X、Y 和 Z 轴波形数据。
    // pub fn get_latest_waveforms(&self, n: usize) -> (Vec<f32>, Vec<f32>, Vec<f32>) {
    //     let read_buffer = |buf: &Arc<Mutex<VecDeque<f32>>>| -> Vec<f32> {
    //         let b = buf.lock().unwrap();
    //         b.iter().rev().take(n).copied().collect::<Vec<_>>().into_iter().rev().collect()
    //     };
    //     (read_buffer(&self.x_buffer), read_buffer(&self.y_buffer), read_buffer(&self.z_buffer))
    // }
}