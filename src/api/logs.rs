use actix_web::{HttpResponse, Responder};
use serde::{Deserialize, Serialize};
use std::sync::Mutex;
use chrono::Local;
use log::{Level, Metadata, Record};

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct LogEntry {
    pub timestamp: String,
    pub level: String,
    pub message: String,
}

pub struct LogStore {
    pub entries: Mutex<Vec<LogEntry>>,
    pub max_entries: usize,
}

impl LogStore {
    pub fn new(max_entries: usize) -> Self {
        Self {
            entries: Mutex::new(Vec::with_capacity(max_entries)),
            max_entries,
        }
    }

    pub fn add_entry(&self, level: String, message: String) {
        if message.is_empty() { return; }
        
        let mut entries = self.entries.lock().unwrap();
        if entries.len() >= self.max_entries {
            entries.remove(0);
        }
        entries.push(LogEntry {
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
                println!("[{}] {} - {}", Local::now().format("%Y-%m-%d %H:%M:%S"), level, message);
                
                // Only save Info and higher to the UI log store to prevent spam
                if record.metadata().level() <= Level::Info {
                    self.store.add_entry(level, message);
                }
            }
        }
    }

    fn flush(&self) {}
}

pub static LOG_STORE: once_cell::sync::Lazy<LogStore> = once_cell::sync::Lazy::new(|| {
    LogStore::new(1000)
});

pub fn init() {
    let logger = GlobalLogger { store: &LOG_STORE };
    log::set_logger(Box::leak(Box::new(logger)))
        .map(|()| log::set_max_level(log::LevelFilter::Debug))
        .expect("Failed to set logger");
}

pub async fn get_logs() -> impl Responder {
    let entries = LOG_STORE.entries.lock().unwrap();
    HttpResponse::Ok().json(&*entries)
}

pub async fn clear_logs() -> impl Responder {
    let mut entries = LOG_STORE.entries.lock().unwrap();
    entries.clear();
    HttpResponse::Ok().json("Logs cleared")
}
