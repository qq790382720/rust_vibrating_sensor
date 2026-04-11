use std::sync::Arc;
use std::time::Duration;
use std::io;
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

const HTTP_BIND_ADDR: &str = "0.0.0.0:8082";

fn bind_error(service: &str, addr: &str, error: io::Error) -> io::Error {
    io::Error::new(
        error.kind(),
        format!("{service} failed to bind {addr}: {error}"),
    )
}

/// NTP时间同步
async fn ntp_sync(server: &str) -> Result<String, String> {
    let output = Command::new("ntpdate")
        .arg(server)
        .output()
        .await
        .map_err(|e| format!("执行ntpdate失败: {}", e))?;

    if output.status.success() {
        let stdout = String::from_utf8_lossy(&output.stdout);
        tracing::info!("NTP同步成功: {}", stdout);
        Ok(format!("NTP同步成功: {}", stdout))
    } else {
        let stderr = String::from_utf8_lossy(&output.stderr);
        tracing::error!("NTP同步失败: {}", stderr);
        Err(format!("NTP同步失败: {}", stderr))
    }
}


#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // 全局日志初始化
    env_logger::init();

    let settings = Arc::new(Settings::from_file("Settings.toml")?);
    let client = MqttPublisher::new(
        &*settings.mqtt_server.url, &*settings.mqtt_server.client_id,
        &*settings.mqtt_server.public_topic,
        &*settings.mqtt_server.username, &*settings.mqtt_server.password
    ).await.map_err(|err| io::Error::other(format!("初始化 MQTT 客户端失败: {err}")))?;
    let mqtt_client = Arc::new(Mutex::new(client));

    let server = Arc::new(WaveServer::new(settings.port));
    let server_clone = server.clone();
    let sensor_bind_addr = server.listen_addr();
    let sensor_listener = tokio::net::TcpListener::bind(&sensor_bind_addr)
        .await
        .map_err(|err| bind_error("Sensor TCP server", &sensor_bind_addr, err))?;
    let sensor_settings = settings.clone();
    let sensor_mqtt_client = mqtt_client.clone();

    tokio::spawn(async move {
        if let Err(err) = server_clone
            .run_with_listener(sensor_listener, sensor_mqtt_client, sensor_settings)
            .await
        {
            tracing::error!("Sensor TCP server stopped: {}", err);
        }
    });

    let config = Arc::new(Settings::new(0));
    let serve_dir = ServeDir::new("ui").not_found_service(ServeFile::new("ui/index.html"));
    // 创建路由
    let app = Router::new()
        .route("/read-settings", get(read_settings))
        .route("/system_restart",get(system_restart))
        .route("/write-settings", post(write_settings))
        .route("/ntp-sync", post(ntp_sync_handler))
        .nest_service("/ui", serve_dir.clone())
        .fallback_service(serve_dir)
        .with_state(Arc::clone(&config))
        .layer(CorsLayer::new()  // 添加这行来启用 CORS
        .allow_origin(Any)   // 允许任何源
        .allow_methods(Any)  // 允许任何 HTTP 方法
        .allow_headers(Any)); // 允许任何头部
    // 启动服务器
    let listener = tokio::net::TcpListener::bind(HTTP_BIND_ADDR)
        .await
        .map_err(|err| bind_error("HTTP server", HTTP_BIND_ADDR, err))?;
    tracing::info!("HTTP server listening on {}", HTTP_BIND_ADDR);
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
    Json(Response { ok: true, message: None })
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
        return Json(Response { ok: false, message: None });
    }

    Json(Response { ok: true, message: None })
}

// NTP时间同步处理
async fn ntp_sync_handler(State(_server): State<Arc<Settings>>) -> Json<Response> {
    let settings = match Settings::from_file("Settings.toml") {
        Ok(s) => s,
        Err(e) => {
            tracing::error!("读取配置文件失败: {}", e);
            return Json(Response { ok: false, message: Some(format!("读取配置失败: {}", e)) });
        }
    };

    let ntp_server = &settings.ntp_server.server;
    if ntp_server.is_empty() {
        return Json(Response { ok: false, message: Some("NTP服务器未配置".to_string()) });
    }

    match ntp_sync(ntp_server).await {
        Ok(msg) => Json(Response { ok: true, message: Some(msg) }),
        Err(e) => Json(Response { ok: false, message: Some(e) }),
    }
}

#[derive(Serialize)]
struct Response {
    ok: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    message: Option<String>,
}
