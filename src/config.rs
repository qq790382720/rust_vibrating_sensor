use serde::{Deserialize, Serialize};
use std::fs::File;
use std::io::Read;
use std::path::Path;


#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct Settings {
    pub port: u16,
    pub sampling_length: usize,
    pub mqtt_server: ServerConfig,
    pub data_upload: DataUploadConfig,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct DataUploadConfig {
    pub company_id: String,
    pub gateway_id: String,
    pub device_id: String,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct ServerConfig {
    pub url: String,
    pub public_topic: String,
    pub client_id: String,
}
impl Settings {
    pub fn new(port: u16) -> Self {
        Self {
            port,
            sampling_length: 100,
            mqtt_server: ServerConfig {
                url: "".to_string(),
                public_topic: "".to_string(),
                client_id: "".to_string(),
            },
            data_upload: DataUploadConfig {
                company_id: "".to_string(),
                gateway_id: "".to_string(),
                device_id: "".to_string(),
            },
        }
    }
    /// 从给定路径加载配置文件
    pub fn from_file<P: AsRef<Path>>(path: P) -> Result<Self, Box<dyn std::error::Error>> {
        let mut file = File::open(path)?;
        let mut contents = String::new();
        file.read_to_string(&mut contents)?;

        let settings: Self = toml::from_str(&contents)?;
        Ok(settings)
    }
    pub fn _from_str(s: &str) -> Result<Self, Box<dyn std::error::Error>> {
        let settings: Self = toml::from_str(s)?;
        Ok(settings)
    }
}