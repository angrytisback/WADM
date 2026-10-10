use actix_web::{web, HttpResponse, Responder};
use serde::{Deserialize, Serialize};
use std::fs;
use std::sync::Mutex;

const CONFIG_FILE: &str = "wadm-config.json";

#[derive(Serialize, Deserialize, Clone, Debug, Default)]
pub struct AppConfig {
    #[serde(default)]
    pub developer_mode: bool,
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
    let config = data.lock().unwrap_or_else(|e| e.into_inner());
    HttpResponse::Ok().json(&*config)
}

#[derive(Deserialize)]
pub struct UpdateConfigReq {
    pub developer_mode: bool,
}

pub async fn update_config(
    user: crate::auth::AuthenticatedUser,
    audit: web::Data<std::sync::Arc<crate::audit::AuditLogger>>,
    body: web::Json<UpdateConfigReq>,
    data: web::Data<Mutex<AppConfig>>,
) -> impl Responder {
    if user.require_admin().is_err() {
        audit.log_denied(&user, "CONFIG_UPDATE", None, Some("Requires Admin role"));
        return HttpResponse::Forbidden().json(serde_json::json!({
            "error": "Insufficient permissions"
        }));
    }

    let mut config = data.lock().unwrap_or_else(|e| e.into_inner());
    config.developer_mode = body.developer_mode;

    if let Err(e) = save_config(&config) {
        log::error!("Failed to save config: {}", e);
        audit.log(
            &user.username,
            user.role.as_str(),
            "CONFIG_UPDATE",
            None,
            &user.client_ip,
            "FAILED",
            Some(&e.to_string()),
        );
        return HttpResponse::InternalServerError().json("Failed to save configuration");
    }

    audit.log(
        &user.username,
        user.role.as_str(),
        "CONFIG_UPDATE",
        None,
        &user.client_ip,
        "SUCCESS",
        Some(&format!("developer_mode: {}", body.developer_mode)),
    );

    HttpResponse::Ok().json(&*config)
}
