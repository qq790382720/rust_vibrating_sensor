use mqtt_endpoint_tokio::mqtt_ep;
use mqtt_endpoint_tokio::mqtt_ep::role::Client;
use std::error::Error;
use std::sync::Arc;
use std::time::Duration;
use tokio::sync::RwLock;

/// MQTT客户端连接状态枚举
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ConnectionStatus {
    /// 已成功连接
    Connected,
    /// 连接断开
    Disconnected,
    /// 正在重连
    Reconnecting,
}
pub struct MqttPublisher {
    pub client_obj:  RwLock<Option<mqtt_ep::endpoint::GenericEndpoint<Client, u16>>>,
    pub topic: String,
    reconnect_base_interval: std::time::Duration,
    max_reconnect_interval: std::time::Duration,
    // 连接状态（RwLock保证多线程安全读写）
    status: RwLock<ConnectionStatus>,
    broker_url: String,
    client_id: String,
    username: String,
    password: String,
}

impl MqttPublisher{
    pub async fn new(broker_url: &str, client_id: &str, topic: &str, username: &str, password: &str) -> Result<Arc<Self>, Box<dyn Error + Send + Sync + 'static>> {
        // let endpoint = mqtt_ep::endpoint::Endpoint::new(mqtt_ep::Version::V3_1_1);
        // let tcp_stream = mqtt_ep::transport::connect_helper::connect_tcp(broker_url, None).await?;
        // let transport = mqtt_ep::transport::TcpTransport::from_stream(tcp_stream);
        // endpoint.attach(transport, mqtt_ep::endpoint::Mode::Client).await?;
        // // Send CONNECT packet
        // let connect = mqtt_ep::packet::v3_1_1::Connect::builder()
        //     .client_id(client_id)?
        //     .keep_alive(60)
        //     .clean_start(true)
        //     .build()?;
        //
        // endpoint.send(connect).await?;
        //
        // // Receive CONNACK
        // let packet = endpoint.recv().await?;
        // println!("Received: {packet:?}");

        // 初始化实例
        let publisher = Arc::new(Self {
            client_obj: RwLock::new(None),
            status: RwLock::new(ConnectionStatus::Disconnected),
            broker_url: broker_url.to_string(),
            client_id: client_id.to_string(),
            username: username.to_string(),
            password: password.to_string(),
            topic: topic.to_string(),
            reconnect_base_interval: std::time::Duration::from_secs(1), // 初始重连间隔1秒
            max_reconnect_interval: std::time::Duration::from_secs(30), // 最大重连间隔30秒
        });

        // 启动后台重连循环任务（独立tokio任务，不阻塞主线程）
        let publisher_clone = publisher.clone();
        tokio::spawn(async move {
            if let Err(err) = publisher_clone.reconnect_loop().await {
                eprintln!("[MQTT] 重连任务异常退出: {err}");
            }
        });

        Ok(publisher)
    }

    pub async fn publish_bytes(&self,payload:&[u8])->Result<(),Box<dyn Error>>{
        let status = self.get_status().await;
        if status != ConnectionStatus::Connected{
            return Ok(())
        }

        // 构建 PUBLISH 包
        let publish = mqtt_ep::packet::v3_1_1::Publish::builder()
            .topic_name(&self.topic)?
            .payload(payload)
            .build()?;

        // 发送 PUBLISH 包
        let client_obj= self.client_obj.read().await;
        let Some(endpoint) = &*client_obj else {
            return Err("发布失败：客户端端点未初始化".into());
        };
        let send_result = endpoint.send(publish).await;
        drop(client_obj);

        if let Err(err) = send_result {
            eprintln!("[MQTT] 发布失败，连接已标记为断开: {err}");
            if let Err(mark_err) = self.mark_disconnected().await {
                eprintln!("[MQTT] 标记断开状态失败: {mark_err}");
            }
            return Err(Box::new(err));
        }

        Ok(())
    }

    async fn reconnect_loop(self: &Arc<Self>) -> Result<(), Box<dyn Error + Send + Sync + 'static>> {
        let connected_poll_interval = Duration::from_secs(1);
        let mut current_retry_interval = self.reconnect_base_interval;

        loop {
            if self.get_status().await == ConnectionStatus::Connected {
                tokio::time::sleep(connected_poll_interval).await;
                continue;
            }

            self.set_status(ConnectionStatus::Reconnecting).await?;
            eprintln!("[MQTT] 尝试重连 (重试间隔: {:?})...", current_retry_interval);
            match self.connect_inner().await {
                Ok(endpoint) => {
                    let mut client_obj = self.client_obj.write().await;
                    *client_obj = Some(endpoint);
                    drop(client_obj);

                    self.set_status(ConnectionStatus::Connected).await?;
                    eprintln!("[MQTT] 成功连接到 {}", self.broker_url);
                    current_retry_interval = self.reconnect_base_interval;
                }
                Err(e) => {
                    self.set_status(ConnectionStatus::Disconnected).await?;
                    eprintln!("[MQTT] 重连失败: {e}");
                    current_retry_interval = std::cmp::min(
                        current_retry_interval * 2,
                        self.max_reconnect_interval
                    );
                    tokio::time::sleep(current_retry_interval).await;
                }
            }
        }

    }

    /// 内部连接实现（封装原始连接逻辑）
    async fn connect_inner(&self) -> Result<mqtt_ep::endpoint::GenericEndpoint<Client, u16>, Box<dyn std::error::Error + Send + Sync + 'static>> {
        let endpoint = mqtt_ep::endpoint::Endpoint::new(mqtt_ep::Version::V3_1_1);
        let tcp_stream = mqtt_ep::transport::connect_helper::connect_tcp(&self.broker_url, None).await?;
        let transport = mqtt_ep::transport::TcpTransport::from_stream(tcp_stream);
        endpoint.attach(transport, mqtt_ep::endpoint::Mode::Client).await?;
        // Send CONNECT _packet
        let connect = mqtt_ep::packet::v3_1_1::Connect::builder()
            .client_id(self.client_id.clone())?
            .keep_alive(60)
            .clean_start(true)
            .user_name(self.username.as_str())?
            .password(self.password.as_bytes().to_vec())?
            .build()?;

        endpoint.send(connect).await?;
        // 等待并验证CONNACK响应
        let packet = endpoint.recv().await?;
        match packet {
            mqtt_ep::packet::Packet::V3_1_1Connack(_packet) => {
                println!("[MQTT] 成功连接到 broker: {}", self.broker_url);
                Ok(endpoint)
            }
            _ => Err(format!("连接失败：预期CONNACK，实际收到 {:?}", packet).into()),
        }
    }

    /// 设置连接状态（内部使用）
    async fn set_status(&self, new_status: ConnectionStatus)   -> Result<(), Box<dyn Error + Send + Sync + 'static>> {
        let mut status = self.status.write().await;
        *status = new_status;
        Ok(())
    }

    async fn mark_disconnected(&self) -> Result<(), Box<dyn Error + Send + Sync + 'static>> {
        let mut client_obj = self.client_obj.write().await;
        *client_obj = None;
        drop(client_obj);
        self.set_status(ConnectionStatus::Disconnected).await
    }

    /// 获取当前连接状态（对外提供）
    pub async fn get_status(&self) -> ConnectionStatus {
        let status = self.status.read().await;
        *status
    }
}
