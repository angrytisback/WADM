use actix_web::{HttpResponse, Responder};
use chrono::Local;
use log::{Level, Metadata, Record};
use serde::{Deserialize, Serialize};
use std::collections::VecDeque;
use std::sync::Mutex;

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct LogEntry {
    pub timestamp: String,
    pub level: String,
    pub message: String,
}

pub struct LogStore {
    pub entries: Mutex<VecDeque<LogEntry>>,
    pub max_entries: usize,
}

impl LogStore {
    pub fn new(max_entries: usize) -> Self {
        Self {
            entries: Mutex::new(VecDeque::with_capacity(max_entries)),
            max_entries,
        }
    }

    pub fn add_entry(&self, level: String, message: String) {
        if message.is_empty() {
            return;
        }

        let mut entries = self.entries.lock().unwrap_or_else(|e| e.into_inner());
        if entries.len() >= self.max_entries {
            entries.pop_front();
        }
        entries.push_back(LogEntry {
            timestamp: Local::now().format("%Y-%m-%d %H:%M:%S").to_string(),
            level,
            message,
        });
    }
}

pub struct GlobalLogger {
    pub store: &'static LogStore,
}

fn humanize_log(msg: &str) -> String {
    // Completely ignore automatic HTTP request logs as requested
    if msg.starts_with("REQ|") {
        return String::new();
    }
    msg.to_string()
}

impl log::Log for GlobalLogger {
    fn enabled(&self, metadata: &Metadata) -> bool {
        // System-wide enabled level (for console/internal)
        metadata.level() <= Level::Debug
    }

    fn log(&self, record: &Record) {
        if self.enabled(record.metadata()) {
            let level = record.level().to_string();
            let raw_message = format!("{}", record.args());
            let message = humanize_log(&raw_message);

            if !message.is_empty() {
                // Always print to console for debugging
                println!(
                    "[{}] {} - {}",
                    Local::now().format("%Y-%m-%d %H:%M:%S"),
                    level,
                    message
                );

                // Only save Info and higher to the UI log store to prevent spam
                if record.metadata().level() <= Level::Info {
                    self.store.add_entry(level, message);
                }
            }
        }
    }

    fn flush(&self) {}
}

pub static LOG_STORE: once_cell::sync::Lazy<LogStore> =
    once_cell::sync::Lazy::new(|| LogStore::new(1000));

pub fn init() {
    let logger = GlobalLogger { store: &LOG_STORE };
    log::set_logger(Box::leak(Box::new(logger)))
        .map(|()| log::set_max_level(log::LevelFilter::Debug))
        .expect("Failed to set logger");
}

pub async fn get_logs() -> impl Responder {
    let entries = LOG_STORE.entries.lock().unwrap_or_else(|e| e.into_inner());
    HttpResponse::Ok().json(&*entries)
}

pub async fn clear_logs(
    user: crate::auth::AuthenticatedUser,
    audit: actix_web::web::Data<std::sync::Arc<crate::audit::AuditLogger>>,
) -> impl Responder {
    if user.require_admin().is_err() {
        audit.log_denied(
            &user,
            "SYSTEM_CLEAR_LOGS",
            None,
            Some("Requires Admin role"),
        );
        return HttpResponse::Forbidden().json(serde_json::json!({
            "error": "Insufficient permissions"
        }));
    }

    let mut entries = LOG_STORE.entries.lock().unwrap_or_else(|e| e.into_inner());
    entries.clear();

    audit.log(
        &user.username,
        user.role.as_str(),
        "SYSTEM_CLEAR_LOGS",
        None,
        &user.client_ip,
        "SUCCESS",
        None,
    );

    HttpResponse::Ok().json("Logs cleared")
}
