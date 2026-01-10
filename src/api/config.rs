use actix_web::{web, HttpResponse, Responder};
use serde::{Deserialize, Serialize};
use std::fs;
use std::sync::Mutex;

const CONFIG_FILE: &str = "wadm-config.json";

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct AppConfig {
    #[serde(default)]
    pub developer_mode: bool,
}

impl Default for AppConfig {
    fn default() -> Self {
        Self {
            developer_mode: false,
        }
    }
}

pub fn load_config() -> AppConfig {
    match fs::read_to_string(CONFIG_FILE) {
        Ok(content) => serde_json::from_str(&content).unwrap_or_default(),
        Err(_) => AppConfig::default(),
    }
}

pub fn save_config(config: &AppConfig) -> std::io::Result<()> {
    let content = serde_json::to_string_pretty(config)?;
    fs::write(CONFIG_FILE, content)
}

pub async fn get_config(data: web::Data<Mutex<AppConfig>>) -> impl Responder {
    let config = data.lock().unwrap();
    HttpResponse::Ok().json(&*config)
}

#[derive(Deserialize)]
pub struct UpdateConfigReq {
    pub developer_mode: bool,
}

pub async fn update_config(
    body: web::Json<UpdateConfigReq>,
    data: web::Data<Mutex<AppConfig>>,
) -> impl Responder {
    let mut config = data.lock().unwrap();
    config.developer_mode = body.developer_mode;

    if let Err(e) = save_config(&config) {
        log::error!("Failed to save config: {}", e);
        return HttpResponse::InternalServerError().json("Failed to save configuration");
    }

    HttpResponse::Ok().json(&*config)
}
