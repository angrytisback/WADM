use actix_web::{HttpResponse, Responder, web};
use serde::{Deserialize, Serialize};
use std::process::Command;
use sysinfo::System;

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct SmartDisk {
    pub name: String,
    pub model: String,
    pub serial: String,
    pub health: String,
    pub temperature: Option<i64>,
    pub power_on_hours: Option<u64>,
}

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
    pub is_root: bool,
    pub smart: Option<Vec<SmartDisk>>,
    pub cpu_temp: Option<f32>,
    pub gpu_temp: Option<f32>,
    pub gpus: Vec<crate::api::monitor::GpuStats>,
}

#[derive(Deserialize)]
pub struct PowerAction {
    pub action: String, // "reboot", "shutdown", "schedule", "schedule_shutdown", "schedule_reboot", "cancel"
    pub minutes: Option<u32>,
    pub hours: Option<u32>,
}

#[derive(Serialize)]
pub struct PowerStatusResponse {
    pub scheduled: bool,
    pub mode: Option<String>,
    pub scheduled_time_usec: Option<u64>,
    pub raw: Option<String>,
}

#[derive(Deserialize)]
pub struct MaintenanceAction {
    pub action: String, // "cache_clean", "trim", "memory_flush", "swap_flush", "full_clean"
}

#[derive(Serialize)]
pub struct MaintenanceResult {
    pub success: bool,
    pub action: String,
    pub message: String,
    pub details: String,
    pub freed_mb: Option<f64>,
}

#[derive(Serialize)]
pub struct DnsInfo {
    pub stats: String,
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

    let smart = fetch_smart_data();
    
    // Fetch CPU Temp
    let cpu_temp = Command::new("sh").arg("-c")
        .arg("cat /sys/class/thermal/thermal_zone0/temp 2>/dev/null || cat /sys/class/thermal/thermal_zone1/temp 2>/dev/null || echo 0")
        .output()
        .map(|o| {
            let temp_str = String::from_utf8_lossy(&o.stdout).trim().to_string();
            let temp_val = temp_str.parse::<f32>().unwrap_or(0.0) / 1000.0;
            if temp_val > 0.0 { Some(temp_val) } else { None }
        }).unwrap_or(None);

    // Fetch GPU Temp (checks multiple vendors)
    let gpus = crate::api::monitor::get_gpu_stats();
    let gpu_temp = gpus.iter()
        .map(|g| g.temp)
        .filter(|t| *t > 0.0)
        .max_by(|a, b| a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal));



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
        username: username.clone(),
        has_sudo,
        is_root: username == "root",
        smart,
        cpu_temp,
        gpu_temp,
        gpus,
    };

    HttpResponse::Ok().json(info)
}

fn fetch_smart_data() -> Option<Vec<SmartDisk>> {
    let output = Command::new("sudo")
        .args(&["-n", "smartctl", "--scan", "--json"])
        .output()
        .ok()?;

    if !output.status.success() {
        return None;
    }

    let stdout = String::from_utf8_lossy(&output.stdout);
    let scan_result: serde_json::Value = serde_json::from_str(&stdout).ok()?;

    let devices = scan_result.get("devices")?.as_array()?;
    let mut disks = Vec::new();

    for device in devices {
        if let Some(name) = device.get("name").and_then(|n| n.as_str()) {
            if let Ok(detail_output) = Command::new("sudo")
                .args(&["-n", "smartctl", "--all", "--json", name])
                .output()
            {
                if let Ok(detail_json) =
                    serde_json::from_slice::<serde_json::Value>(&detail_output.stdout)
                {
                    let model = detail_json
                        .get("model_name")
                        .or_else(|| detail_json.get("model_family"))
                        .and_then(|v| v.as_str())
                        .unwrap_or("Unknown")
                        .to_string();

                    let serial = detail_json
                        .get("serial_number")
                        .and_then(|v| v.as_str())
                        .unwrap_or("Unknown")
                        .to_string();

                    let health = if let Some(passed) = detail_json
                        .get("smart_status")
                        .and_then(|s| s.get("passed"))
                        .and_then(|b| b.as_bool())
                    {
                        if passed { "Passed".to_string() } else { "Failed".to_string() }
                    } else { "Unknown".to_string() };

                    let temperature = detail_json
                        .get("temperature")
                        .and_then(|t| t.get("current"))
                        .and_then(|v| v.as_i64());

                    let power_on_hours = detail_json
                        .get("power_on_time")
                        .and_then(|p| p.get("hours"))
                        .and_then(|v| v.as_u64());

                    disks.push(SmartDisk {
                        name: name.to_string(),
                        model,
                        serial,
                        health,
                        temperature,
                        power_on_hours,
                    });
                }
            }
        }
    }

    if disks.is_empty() { None } else { Some(disks) }
}

pub async fn reboot_system() -> impl Responder {
    actix_web::rt::spawn(async {
        tokio::time::sleep(tokio::time::Duration::from_millis(800)).await;
        let _ = Command::new("sudo").args(&["-n", "shutdown", "-r", "now"]).output();
        let _ = Command::new("sudo").args(&["-n", "reboot"]).output();
    });
    HttpResponse::Ok().json("Reboot initiated. Server is restarting now.")
}

pub fn get_scheduled_power_status() -> PowerStatusResponse {
    let path = std::path::Path::new("/run/systemd/shutdown/scheduled");
    if path.exists() {
        if let Ok(content) = std::fs::read_to_string(path) {
            let mut mode = None;
            let mut usec = None;
            for line in content.lines() {
                if let Some(m) = line.strip_prefix("MODE=") {
                    mode = Some(m.trim().to_string());
                } else if let Some(u) = line.strip_prefix("USEC=") {
                    usec = u.trim().parse::<u64>().ok();
                }
            }
            return PowerStatusResponse {
                scheduled: true,
                mode,
                scheduled_time_usec: usec,
                raw: Some(content),
            };
        }
    }
    PowerStatusResponse {
        scheduled: false,
        mode: None,
        scheduled_time_usec: None,
        raw: None,
    }
}

pub async fn get_power_status() -> impl Responder {
    HttpResponse::Ok().json(get_scheduled_power_status())
}

pub async fn handle_power_action(payload: web::Json<PowerAction>) -> impl Responder {
    let total_minutes = payload.hours.unwrap_or(0) * 60 + payload.minutes.unwrap_or(0);

    match payload.action.as_str() {
        "reboot" => {
            actix_web::rt::spawn(async {
                tokio::time::sleep(tokio::time::Duration::from_millis(800)).await;
                let _ = Command::new("sudo").args(&["-n", "shutdown", "-r", "now"]).output();
                let _ = Command::new("sudo").args(&["-n", "reboot"]).output();
            });
            HttpResponse::Ok().json("Reboot initiated. Server is restarting now.")
        }
        "shutdown" => {
            actix_web::rt::spawn(async {
                tokio::time::sleep(tokio::time::Duration::from_millis(800)).await;
                let _ = Command::new("sudo").args(&["-n", "shutdown", "-h", "now"]).output();
            });
            HttpResponse::Ok().json("Shutdown initiated. Server is powering down now.")
        }
        "schedule" | "schedule_shutdown" => {
            let mins = if total_minutes > 0 { total_minutes } else { 60 };
            let output = Command::new("sudo").args(&["-n", "shutdown", "-h", &format!("+{}", mins)]).output();
            match output {
                Ok(o) if o.status.success() => {
                    HttpResponse::Ok().json(format!("Shutdown scheduled in {} minutes.", mins))
                }
                Ok(o) => HttpResponse::InternalServerError().json(String::from_utf8_lossy(&o.stderr)),
                Err(e) => HttpResponse::InternalServerError().json(e.to_string()),
            }
        }
        "schedule_reboot" => {
            let mins = if total_minutes > 0 { total_minutes } else { 60 };
            let output = Command::new("sudo").args(&["-n", "shutdown", "-r", &format!("+{}", mins)]).output();
            match output {
                Ok(o) if o.status.success() => {
                    HttpResponse::Ok().json(format!("Reboot scheduled in {} minutes.", mins))
                }
                Ok(o) => HttpResponse::InternalServerError().json(String::from_utf8_lossy(&o.stderr)),
                Err(e) => HttpResponse::InternalServerError().json(e.to_string()),
            }
        }
        "cancel" => {
            let output = Command::new("sudo").args(&["-n", "shutdown", "-c"]).output();
            match output {
                Ok(o) if o.status.success() => {
                    HttpResponse::Ok().json("Scheduled power sequence successfully cancelled.")
                }
                Ok(o) => HttpResponse::InternalServerError().json(String::from_utf8_lossy(&o.stderr)),
                Err(e) => HttpResponse::InternalServerError().json(e.to_string()),
            }
        }
        _ => HttpResponse::BadRequest().json("Invalid action specified."),
    }
}

fn get_memory_metrics_kb() -> (u64, u64, u64, u64) {
    let mut total = 0;
    let mut available = 0;
    let mut swap_total = 0;
    let mut swap_free = 0;

    if let Ok(content) = std::fs::read_to_string("/proc/meminfo") {
        for line in content.lines() {
            let parts: Vec<&str> = line.split_whitespace().collect();
            if parts.len() >= 2 {
                match parts[0] {
                    "MemTotal:" => total = parts[1].parse().unwrap_or(0),
                    "MemAvailable:" => available = parts[1].parse().unwrap_or(0),
                    "SwapTotal:" => swap_total = parts[1].parse().unwrap_or(0),
                    "SwapFree:" => swap_free = parts[1].parse().unwrap_or(0),
                    _ => {}
                }
            }
        }
    }
    (total, available, swap_total, swap_free)
}

fn do_memory_flush() -> MaintenanceResult {
    let (_, avail_before, _, _) = get_memory_metrics_kb();

    let _ = Command::new("sync").status();
    let drop_res = Command::new("sh")
        .arg("-c")
        .arg("echo 3 | sudo -n tee /proc/sys/vm/drop_caches")
        .output();
    let _ = Command::new("sh")
        .arg("-c")
        .arg("echo 1 | sudo -n tee /proc/sys/vm/compact_memory 2>/dev/null || true")
        .status();

    let (_, avail_after, _, _) = get_memory_metrics_kb();
    let freed_kb = if avail_after > avail_before { avail_after - avail_before } else { 0 };
    let freed_mb = (freed_kb as f64) / 1024.0;

    match drop_res {
        Ok(output) if output.status.success() => {
            let msg = if freed_mb > 1.0 {
                format!("RAM Flushed: {:.1} MB cache memory reclaimed.", freed_mb)
            } else {
                "RAM Flushed successfully. PageCache, dentries, and inodes cleared.".to_string()
            };
            let details = format!(
                "Kernel drop_caches (mode 3) executed successfully.\nAvailable RAM before: {:.1} MB\nAvailable RAM after: {:.1} MB\nReclaimed: {:.1} MB",
                avail_before as f64 / 1024.0,
                avail_after as f64 / 1024.0,
                freed_mb
            );
            MaintenanceResult {
                success: true,
                action: "memory_flush".to_string(),
                message: msg,
                details,
                freed_mb: Some(freed_mb),
            }
        }
        Ok(output) => {
            let err = String::from_utf8_lossy(&output.stderr).to_string();
            MaintenanceResult {
                success: false,
                action: "memory_flush".to_string(),
                message: "Failed to drop caches. Sudo privileges may be required.".to_string(),
                details: err,
                freed_mb: None,
            }
        }
        Err(e) => MaintenanceResult {
            success: false,
            action: "memory_flush".to_string(),
            message: format!("Failed to execute drop_caches: {}", e),
            details: e.to_string(),
            freed_mb: None,
        },
    }
}

fn do_cache_clean() -> MaintenanceResult {
    let (_, avail_before, _, _) = get_memory_metrics_kb();
    let mut logs = Vec::new();

    // 1. Package manager cache cleaning
    if Command::new("which").arg("apt-get").output().map(|o| o.status.success()).unwrap_or(false) {
        let _ = Command::new("sudo").args(&["-n", "apt-get", "clean"]).output();
        let _ = Command::new("sudo").args(&["-n", "apt-get", "autoclean"]).output();
        logs.push("Cleaned APT package cache archives.".to_string());
    } else if Command::new("which").arg("dnf").output().map(|o| o.status.success()).unwrap_or(false) {
        let _ = Command::new("sudo").args(&["-n", "dnf", "clean", "all"]).output();
        logs.push("Cleaned DNF package cache archives.".to_string());
    } else if Command::new("which").arg("pacman").output().map(|o| o.status.success()).unwrap_or(false) {
        let _ = Command::new("sudo").args(&["-n", "pacman", "-Sc", "--noconfirm"]).output();
        logs.push("Cleaned Pacman package cache archives.".to_string());
    }

    // 2. Systemd journal logs vacuum (> 3 days)
    if Command::new("which").arg("journalctl").output().map(|o| o.status.success()).unwrap_or(false) {
        let j_res = Command::new("sudo").args(&["-n", "journalctl", "--vacuum-time=3d"]).output();
        if let Ok(j) = j_res {
            let s = String::from_utf8_lossy(&j.stdout).trim().to_string();
            if !s.is_empty() {
                logs.push(format!("Journalctl vacuum: {}", s.lines().last().unwrap_or(&s)));
            } else {
                logs.push("Vacuumed systemd journal logs older than 3 days.".to_string());
            }
        }
    }

    // 3. Drop filesystem caches
    let _ = Command::new("sync").status();
    let _ = Command::new("sh").arg("-c").arg("echo 3 | sudo -n tee /proc/sys/vm/drop_caches").output();
    logs.push("Dropped kernel PageCache, dentries, and inode caches.".to_string());

    let (_, avail_after, _, _) = get_memory_metrics_kb();
    let freed_kb = if avail_after > avail_before { avail_after - avail_before } else { 0 };
    let freed_mb = (freed_kb as f64) / 1024.0;

    let msg = if freed_mb > 1.0 {
        format!("System Cache Cleaned: Package archives, journal logs, and {:.1} MB RAM freed.", freed_mb)
    } else {
        "System Cache Cleaned: Package archives and system journal logs vacuumed.".to_string()
    };

    MaintenanceResult {
        success: true,
        action: "cache_clean".to_string(),
        message: msg,
        details: logs.join("\n"),
        freed_mb: Some(freed_mb),
    }
}

fn do_swap_flush() -> MaintenanceResult {
    let (_, avail_kb, swap_total, swap_free) = get_memory_metrics_kb();
    let swap_used = swap_total.saturating_sub(swap_free);

    if swap_used == 0 {
        return MaintenanceResult {
            success: true,
            action: "swap_flush".to_string(),
            message: "Swap is currently empty. No flush required.".to_string(),
            details: "Used swap is 0 kB.".to_string(),
            freed_mb: None,
        };
    }

    if avail_kb < swap_used + (150 * 1024) {
        return MaintenanceResult {
            success: false,
            action: "swap_flush".to_string(),
            message: format!(
                "Swap flush skipped: Available RAM ({:.0} MB) is too low to safely hold Swap ({:.0} MB).",
                avail_kb as f64 / 1024.0,
                swap_used as f64 / 1024.0
            ),
            details: "Flushing swap requires sufficient free physical memory to avoid triggering OOM (Out Of Memory).".to_string(),
            freed_mb: None,
        };
    }

    let res = Command::new("sh")
        .arg("-c")
        .arg("sudo -n swapoff -a && sudo -n swapon -a")
        .output();

    match res {
        Ok(o) if o.status.success() => MaintenanceResult {
            success: true,
            action: "swap_flush".to_string(),
            message: format!("Swap Flushed: {:.1} MB returned to RAM.", swap_used as f64 / 1024.0),
            details: "swapoff -a && swapon -a completed successfully.".to_string(),
            freed_mb: Some(swap_used as f64 / 1024.0),
        },
        Ok(o) => MaintenanceResult {
            success: false,
            action: "swap_flush".to_string(),
            message: "Swap flush failed.".to_string(),
            details: String::from_utf8_lossy(&o.stderr).to_string(),
            freed_mb: None,
        },
        Err(e) => MaintenanceResult {
            success: false,
            action: "swap_flush".to_string(),
            message: format!("Failed to execute swap flush: {}", e),
            details: e.to_string(),
            freed_mb: None,
        },
    }
}

fn do_trim() -> MaintenanceResult {
    let res = Command::new("sudo").args(&["-n", "fstrim", "-av"]).output();
    match res {
        Ok(o) if o.status.success() => {
            let out = String::from_utf8_lossy(&o.stdout).trim().to_string();
            MaintenanceResult {
                success: true,
                action: "trim".to_string(),
                message: "SSD TRIM completed successfully.".to_string(),
                details: out,
                freed_mb: None,
            }
        }
        Ok(o) => MaintenanceResult {
            success: false,
            action: "trim".to_string(),
            message: "SSD TRIM failed or not supported on this filesystem.".to_string(),
            details: String::from_utf8_lossy(&o.stderr).to_string(),
            freed_mb: None,
        },
        Err(e) => MaintenanceResult {
            success: false,
            action: "trim".to_string(),
            message: format!("Failed to execute fstrim: {}", e),
            details: e.to_string(),
            freed_mb: None,
        },
    }
}

pub async fn handle_maintenance_action(payload: web::Json<MaintenanceAction>) -> impl Responder {
    let action = payload.action.clone();

    // Execute in tokio blocking pool to prevent blocking actix worker threads
    let result = actix_web::web::block(move || match action.as_str() {
        "memory_flush" => do_memory_flush(),
        "cache_clean" => do_cache_clean(),
        "swap_flush" => do_swap_flush(),
        "trim" => do_trim(),
        "full_clean" => {
            let c = do_cache_clean();
            let m = do_memory_flush();
            let t = do_trim();
            let s = do_swap_flush();
            let total_freed = m.freed_mb.unwrap_or(0.0) + c.freed_mb.unwrap_or(0.0);
            MaintenanceResult {
                success: true,
                action: "full_clean".to_string(),
                message: format!("Full Cleanup Complete: {:.1} MB RAM freed, caches purged, and TRIM executed.", total_freed),
                details: format!(
                    "--- Cache Clean ---\n{}\n\n--- Memory Flush ---\n{}\n\n--- SSD Trim ---\n{}\n\n--- Swap Flush ---\n{}",
                    c.details, m.details, t.details, s.details
                ),
                freed_mb: Some(total_freed),
            }
        }
        _ => MaintenanceResult {
            success: false,
            action,
            message: "Invalid maintenance action specified.".to_string(),
            details: String::new(),
            freed_mb: None,
        },
    }).await;

    match result {
        Ok(res) => {
            if res.success {
                HttpResponse::Ok().json(res)
            } else {
                HttpResponse::BadRequest().json(res)
            }
        }
        Err(e) => HttpResponse::InternalServerError().json(MaintenanceResult {
            success: false,
            action: payload.action.clone(),
            message: format!("Maintenance task failed: {}", e),
            details: e.to_string(),
            freed_mb: None,
        }),
    }
}

pub async fn get_dns_info() -> impl Responder {
    let output = Command::new("sudo").args(&["-n", "resolvectl", "statistics"]).output()
        .or_else(|_| Command::new("sudo").args(&["-n", "systemd-resolve", "--statistics"]).output());

    match output {
        Ok(o) => {
            let stats = String::from_utf8_lossy(&o.stdout).to_string();
            HttpResponse::Ok().json(DnsInfo { stats })
        }
        Err(e) => HttpResponse::InternalServerError().json(format!("Failed to get DNS stats: {}", e)),
    }
}

pub async fn flush_dns() -> impl Responder {
    let output = Command::new("sudo").args(&["-n", "resolvectl", "flush-caches"]).output()
        .or_else(|_| Command::new("sudo").args(&["-n", "systemd-resolve", "--flush-caches"]).output());

    match output {
        Ok(o) => {
            if o.status.success() { HttpResponse::Ok().json("DNS cache flushed") }
            else { HttpResponse::InternalServerError().json(String::from_utf8_lossy(&o.stderr)) }
        }
        Err(e) => HttpResponse::InternalServerError().json(e.to_string()),
    }
}

#[derive(Serialize)]
pub struct SpeedtestResult {
    pub download_mbps: f32,
    pub upload_mbps: f32,
    pub ping_ms: f32,
}

pub async fn run_speedtest() -> impl Responder {
    
    
    let result = actix_web::web::block(move || {
        let output = Command::new("sh")
            .arg("-c")
            .arg("curl -sL https://raw.githubusercontent.com/sivel/speedtest-cli/master/speedtest.py | python3 - --json")
            .output()?;
        if output.status.success() {
            let stdout = String::from_utf8_lossy(&output.stdout);
            if let Ok(json) = serde_json::from_str::<serde_json::Value>(&stdout) {
                let download = json.get("download").and_then(|v| v.as_f64()).unwrap_or(0.0) / 1_000_000.0;
                let upload = json.get("upload").and_then(|v| v.as_f64()).unwrap_or(0.0) / 1_000_000.0;
                let ping = json.get("ping").and_then(|v| v.as_f64()).unwrap_or(0.0);
                return Ok(SpeedtestResult {
                    download_mbps: download as f32,
                    upload_mbps: upload as f32,
                    ping_ms: ping as f32,
                });
            }
        }
        Err(std::io::Error::new(std::io::ErrorKind::Other, "Speedtest failed"))
    }).await;

    match result {
        Ok(Ok(res)) => HttpResponse::Ok().json(res),
        Ok(Err(e)) => HttpResponse::InternalServerError().json(format!("Speedtest failed: {}", e)),
        Err(e) => HttpResponse::InternalServerError().json(format!("Execution failed: {}", e)),
    }
}
