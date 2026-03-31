use std::sync::Arc;
use std::time::Duration;
use tokio::sync::Mutex;

use axum::routing::post;
use crate::config::Settings;
use crate::mqtt::MqttPublisher;
use crate::server::WaveServer;
use axum::{Router, routing::get, extract::State, Json};
use serde::Serialize;
use tokio::process::Command;
use tower_http::cors::{CorsLayer, Any};
use tower_http::services::{ServeDir, ServeFile};

mod server;
mod packet;
mod constants;
mod config;
mod mqtt;


#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {

    let settings = Settings::from_file("Settings.toml")?;
    let client = MqttPublisher::new(
        &*settings.mqtt_server.url, &*settings.mqtt_server.client_id,
        &*settings.mqtt_server.public_topic).await.expect("TODO: panic message");
    let mqtt_client = Arc::new(Mutex::new(client));

    // 全局日志初始化
    env_logger::init();

    let server = Arc::new(WaveServer::new(settings.port));
    let server_clone = server.clone();

    tokio::spawn(async move {
        server_clone.run(mqtt_client, Arc::new(settings)).await.unwrap();
    });

    let config = Arc::new(Settings::new(0));
    let serve_dir = ServeDir::new("assetsa").not_found_service(ServeFile::new("assetsa/index.html"));
    // 创建路由
    let app = Router::new()
        .route("/read-settings", get(read_settings))
        .route("/system_restart",get(system_restart))
        .route("/write-settings", post(write_settings))
        .nest_service("/assets", serve_dir.clone())
        .fallback_service(serve_dir)
        .with_state(Arc::clone(&config))
        .layer(CorsLayer::new()  // 添加这行来启用 CORS
        .allow_origin(Any)   // 允许任何源
        .allow_methods(Any)  // 允许任何 HTTP 方法
        .allow_headers(Any)); // 允许任何头部
    // 启动服务器
    let listener = tokio::net::TcpListener::bind("0.0.0.0:8082").await?;
    axum::serve(listener, app).await?;

    loop {
        tokio::time::sleep(Duration::from_secs(10)).await;
        let sensors = server.get_connected_sensors();
        log::info!("Active sensors: {}", sensors.await.clone().len());
    }
}

// 读取设置文件
async fn read_settings(State(_server): State<Arc<Settings>>) -> String {
    let settings=Settings::from_file("Settings.toml").unwrap();
    serde_json::to_string(&settings).unwrap()

}
async fn system_restart(State(_server):State<Arc<Settings>>) -> Json<Response> {
    // 使用 tokio::process::Command（异步非阻塞）
    let result = Command::new("reboot")
        .arg("-f")
        .status() // 或 .output()，但 status() 更轻量
        .await;

    // 可选：记录错误（即使忽略，也建议 log）
    match &result {
        Ok(status) if status.success() => {
            tracing::info!("Reboot command succeeded");
        }
        Ok(status) => {
            tracing::error!("Reboot failed with status: {}", status);
        }
        Err(e) => {
            tracing::error!("Failed to spawn reboot command: {}", e);
        }
    }

    // 无论成功失败，都返回 { "Ok": true }
    Json(Response { ok: true })
}
// 写入设置文件
async fn write_settings(State(_server): State<Arc<Settings>>, axum::extract::Json(payload): axum::extract::Json<Settings>) -> Json<Response> {
    use std::fs::File;
    use std::io::Write;

    let settings_path = "Settings.toml";
    let mut file = File::create(settings_path).expect("无法创建设置文件");

    let toml_str = toml::to_string_pretty(&payload).unwrap();

    if let Err(e) = writeln!(file, "{}", toml_str) {
        eprintln!("写入设置文件时出错: {}", e);
        return Json(Response { ok: false });
    }

    Json(Response { ok: true })
}
#[derive(Serialize)]
struct Response {
    ok: bool,
}
