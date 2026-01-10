use actix_web::{HttpResponse, Responder};
use serde::Serialize;
use sysinfo::System;

#[derive(Serialize)]
pub struct DetailedSystemInfo {
    pub os_name: String,
    pub os_version: String,
    pub kernel_version: String,
    pub host_name: String,
    pub uptime: u64,
    pub cpu_arch: String,
    pub cpu_count: usize,
    pub total_memory: u64,
    pub used_memory: u64,
    pub total_swap: u64,
    pub used_swap: u64,
    pub username: String,
    pub has_sudo: bool,
}

pub async fn get_detailed_info() -> impl Responder {
    let mut sys = System::new_all();
    sys.refresh_all();

    
    let username = std::process::Command::new("whoami")
        .output()
        .map(|o| String::from_utf8_lossy(&o.stdout).trim().to_string())
        .unwrap_or_else(|_| "Unknown".to_string());

    
    let has_sudo = std::process::Command::new("sudo")
        .arg("-n")
        .arg("true")
        .status()
        .map(|s| s.success())
        .unwrap_or(false);

    let info = DetailedSystemInfo {
        os_name: System::name().unwrap_or_else(|| "Unknown".to_string()),
        os_version: System::os_version().unwrap_or_else(|| "Unknown".to_string()),
        kernel_version: System::kernel_version().unwrap_or_else(|| "Unknown".to_string()),
        host_name: System::host_name().unwrap_or_else(|| "Unknown".to_string()),
        uptime: System::uptime(),
        cpu_arch: System::cpu_arch(),
        cpu_count: sys.cpus().len(),
        total_memory: sys.total_memory(),
        used_memory: sys.used_memory(),
        total_swap: sys.total_swap(),
        used_swap: sys.used_swap(),
        username,
        has_sudo,
    };

    HttpResponse::Ok().json(info)
}

pub async fn reboot_system() -> impl Responder {
    let output = std::process::Command::new("sudo").arg("reboot").output();

    match output {
        Ok(o) => {
            if o.status.success() {
                HttpResponse::Ok().json("Reboot initiated")
            } else {
                let stderr = String::from_utf8_lossy(&o.stderr);
                HttpResponse::InternalServerError().json(format!("Failed to reboot: {}", stderr))
            }
        }
        Err(e) => HttpResponse::InternalServerError()
            .json(format!("Failed to execute reboot command: {}", e)),
    }
}
