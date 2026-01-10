use actix_web::{web, HttpResponse, Responder};
use serde::Serialize;
use std::sync::Mutex;
use sysinfo::{Disks, Networks, ProcessesToUpdate, System};

#[derive(Serialize)]
pub struct SystemStats {
    pub cpu_usage: f32,
    pub ram_total: u64,
    pub ram_used: u64,
    pub swap_total: u64,
    pub swap_used: u64,
    pub disk_total: u64,
    pub disk_used: u64,
    pub network_rx: u64,
    pub network_tx: u64,
    pub active_services: u32,
    pub failed_services: u32,
    pub active_containers: u32,
    pub upgradable_packages: u32,
    pub network_interface: String,
    pub network_max_speed: u64,
}

pub struct AppState {
    pub sys: Mutex<System>,
    pub networks: Mutex<Networks>,
}

fn get_default_interface() -> String {
    use std::process::Command;
    let output = Command::new("sh")
        .arg("-c")
        .arg("ip route | grep default | awk '{print $5}' | head -n1")
        .output();

    match output {
        Ok(o) => String::from_utf8_lossy(&o.stdout).trim().to_string(),
        Err(_) => String::new(),
    }
}

fn get_interface_speed(iface: &str) -> u64 {
    
    
    if iface.is_empty() {
        return 125_000_000; 
    }

    let path = format!("/sys/class/net/{}/speed", iface);
    match std::fs::read_to_string(path) {
        Ok(content) => {
            let mbits = content.trim().parse::<u64>().unwrap_or(1000);
            
            mbits * 125_000
        }
        Err(_) => 125_000_000, 
    }
}

fn count_services(state: &str) -> u32 {
    use std::process::Command;
    
    let output = Command::new("systemctl")
        .args(&[
            "list-units",
            "--type=service",
            &format!("--state={}", state),
            "--no-legend",
            "--no-pager",
        ])
        .output();

    match output {
        Ok(out) => {
            let stdout = String::from_utf8_lossy(&out.stdout);
            stdout.lines().count() as u32
        }
        Err(_) => 0,
    }
}

fn count_containers() -> u32 {
    use std::process::Command;
    
    let output = Command::new("docker").args(&["ps", "-q"]).output();

    match output {
        Ok(out) => {
            let stdout = String::from_utf8_lossy(&out.stdout);
            stdout.lines().count() as u32
        }
        Err(_) => 0,
    }
}

pub async fn get_system_stats(data: web::Data<AppState>) -> impl Responder {
    let mut sys = data.sys.lock().unwrap();
    sys.refresh_all();

    let cpu_usage = sys.global_cpu_usage();
    let ram_total = sys.total_memory();
    let ram_used = sys.used_memory();
    let swap_total = sys.total_swap();
    let swap_used = sys.used_swap();

    let disks = Disks::new_with_refreshed_list();
    let mut disk_total = 0;
    let mut disk_used = 0;

    for disk in disks.list() {
        disk_total += disk.total_space();
        
        disk_used += disk.total_space() - disk.available_space();
    }

    let mut networks = data.networks.lock().unwrap();
    
    networks.refresh(true);

    let mut network_rx = 0;
    let mut network_tx = 0;

    for (_interface_name, data) in &*networks {
        network_rx += data.received();
        network_tx += data.transmitted();
    }

    
    let active_services = count_services("running");
    let failed_services = count_services("failed");

    
    let active_containers = count_containers();
    
    let upgradable_packages = crate::api::pkgmgr::count_upgradable_packages();

    
    let network_interface = get_default_interface();
    let network_max_speed = get_interface_speed(&network_interface);

    let stats = SystemStats {
        cpu_usage,
        ram_total,
        ram_used,
        swap_total,
        swap_used,
        disk_total,
        disk_used,
        network_rx,
        network_tx,
        active_services,
        failed_services,
        active_containers,
        upgradable_packages,
        network_interface,
        network_max_speed,
    };

    HttpResponse::Ok().json(stats)
}

#[derive(Serialize)]
pub struct ProcessInfo {
    pub pid: u32,
    pub name: String,
    pub cpu_usage: f32,
    pub memory: u64,
}

pub async fn get_processes(data: web::Data<AppState>) -> impl Responder {
    let mut sys = data.sys.lock().unwrap();
    
    sys.refresh_processes(ProcessesToUpdate::All, true);

    let mut processes: Vec<ProcessInfo> = sys
        .processes()
        .iter()
        .map(|(pid, process)| ProcessInfo {
            pid: pid.as_u32(),
            name: process.name().to_string_lossy().into_owned(),
            cpu_usage: process.cpu_usage(),
            memory: process.memory(),
        })
        .collect();

    
    processes.sort_by(|a, b| b.cpu_usage.partial_cmp(&a.cpu_usage).unwrap());

    
    let top_processes: Vec<ProcessInfo> = processes.into_iter().take(50).collect();

    HttpResponse::Ok().json(top_processes)
}

#[derive(serde::Deserialize)]
pub struct ProcessAction {
    pub pid: i32,
    pub signal: String, 
}

pub async fn kill_process(body: web::Json<ProcessAction>) -> impl Responder {
    let pid = body.pid;
    let signal = match body.signal.as_str() {
        "SIGKILL" => 9,
        _ => 15, 
    };

    
    
    let output = std::process::Command::new("kill")
        .arg(format!("-{}", signal))
        .arg(pid.to_string())
        .output();

    match output {
        Ok(o) => {
            if o.status.success() {
                HttpResponse::Ok().json(format!("Process {} signal {} sent", pid, signal))
            } else {
                let err = String::from_utf8_lossy(&o.stderr);
                HttpResponse::InternalServerError().json(format!("Failed to kill process: {}", err))
            }
        }
        Err(e) => {
            HttpResponse::InternalServerError().json(format!("Failed to execute kill: {}", e))
        }
    }
}
