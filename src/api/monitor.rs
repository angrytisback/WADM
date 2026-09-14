use actix_web::{web, HttpResponse, Responder};
use serde::Serialize;
use serde_json;
use std::process::Command;
use std::sync::Mutex;
use sysinfo::{Components, Disks, Networks, ProcessesToUpdate, System};

#[derive(Serialize)]
pub struct GpuStats {
    pub load: f32,
    pub vram_used: u64,
    pub vram_total: u64,
    pub temp: f32,
    pub name: String,
    pub vendor: String,
    pub pci_id: String,
    pub error: Option<String>,
}

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
    pub gpus: Vec<GpuStats>,
    pub cpu_temp: f32,
}

pub struct AppState {
    pub sys: Mutex<System>,
    pub networks: Mutex<Networks>,
}

fn get_pci_gpus() -> Vec<String> {
    let mut gpu_pci_ids = Vec::new();
    if let Ok(entries) = std::fs::read_dir("/sys/bus/pci/devices/") {
        for entry in entries.flatten() {
            let path = entry.path();
            let class_path = path.join("class");
            if let Ok(class) = std::fs::read_to_string(class_path) {
                let class = class.trim();
                // 0x0300 is VGA compatible, 0x0302 is 3D controller, 0x0380 is Display controller
                if class.starts_with("0x0300")
                    || class.starts_with("0x0302")
                    || class.starts_with("0x0380")
                {
                    gpu_pci_ids.push(entry.file_name().to_string_lossy().to_string());
                }
            }
        }
    }
    gpu_pci_ids
}

fn fill_nvidia_stats(stats: &mut GpuStats, pci_id: &str) {
    let output = Command::new("sudo")
        .args([
            "-n",
            "nvidia-smi",
            "-i",
            pci_id,
            "--query-gpu=name,utilization.gpu,memory.used,memory.total,temperature.gpu",
            "--format=csv,noheader,nounits",
        ])
        .output();

    if let Ok(o) = output {
        if o.status.success() {
            let stdout = String::from_utf8_lossy(&o.stdout);
            if let Some(line) = stdout.lines().next() {
                let parts: Vec<&str> = line.split(',').map(|s| s.trim()).collect();
                if parts.len() >= 5 {
                    stats.name = parts[0].to_string();
                    let load_str = parts[1].replace("%", "").trim().to_string();
                    stats.load = load_str.parse().unwrap_or(0.0);
                    stats.vram_used = parts[2].parse().unwrap_or(0) * 1024 * 1024;
                    stats.vram_total = parts[3].parse().unwrap_or(0) * 1024 * 1024;
                    stats.temp = parts[4].parse().unwrap_or(0.0);
                    stats.pci_id = pci_id.to_string();
                }
            }
        } else {
            stats.error = Some("nvidia-smi failed".to_string());
        }
    } else {
        // Fallback: try without -i if specific PCI ID fails
        if let Ok(o) = Command::new("sudo")
            .args([
                "-n",
                "nvidia-smi",
                "--query-gpu=name,utilization.gpu,memory.used,memory.total,temperature.gpu",
                "--format=csv,noheader,nounits",
            ])
            .output()
        {
            if o.status.success() {
                let stdout = String::from_utf8_lossy(&o.stdout);
                // We pick the first one if we can't find the specific one, or better, try to match by name
                if let Some(line) = stdout.lines().next() {
                    let parts: Vec<&str> = line.split(',').map(|s| s.trim()).collect();
                    if parts.len() >= 5 {
                        stats.load = parts[1].replace("%", "").trim().parse().unwrap_or(0.0);
                        stats.vram_used = parts[2].parse().unwrap_or(0) * 1024 * 1024;
                        stats.vram_total = parts[3].parse().unwrap_or(0) * 1024 * 1024;
                        stats.temp = parts[4].parse().unwrap_or(0.0);
                    }
                }
            }
        }
        if stats.load == 0.0 && stats.error.is_none() {
            stats.error = Some("nvidia-smi not found".to_string());
        }
    }
}

fn fill_amd_stats(stats: &mut GpuStats, card_name: &str) {
    let device_path = format!("/sys/class/drm/{}/device", card_name);

    let load_path = format!("{}/gpu_busy_percent", device_path);
    let vram_used_path = format!("{}/mem_info_vram_used", device_path);
    let vram_total_path = format!("{}/mem_info_vram_total", device_path);

    if let Ok(load) = std::fs::read_to_string(&load_path) {
        stats.load = load.trim().parse().unwrap_or(0.0);
    }
    if let Ok(vram_used) = std::fs::read_to_string(&vram_used_path) {
        stats.vram_used = vram_used.trim().parse().unwrap_or(0);
    }
    if let Ok(vram_total) = std::fs::read_to_string(&vram_total_path) {
        stats.vram_total = vram_total.trim().parse().unwrap_or(0);
    }

    if let Ok(hwmon_dir) = std::fs::read_dir(format!("{}/hwmon", device_path)) {
        for entry in hwmon_dir.flatten() {
            let temp_path = entry.path().join("temp1_input");
            if temp_path.exists() {
                if let Ok(temp_str) = std::fs::read_to_string(temp_path) {
                    stats.temp = temp_str.trim().parse::<f32>().unwrap_or(0.0) / 1000.0;
                    break;
                }
            }
        }
    }
}

fn fill_intel_stats(stats: &mut GpuStats, card_name: &str) {
    log::trace!("Gathering Intel GPU stats for {}", card_name);
    let has_intel_top = Command::new("which")
        .arg("intel_gpu_top")
        .output()
        .map(|o| o.status.success())
        .unwrap_or(false);
    if has_intel_top {
        let output = Command::new("sudo")
            .args(["-n", "intel_gpu_top", "-J", "-s", "200", "-n", "1"])
            .output();

        if let Ok(o) = output {
            let stdout = String::from_utf8_lossy(&o.stdout);
            if let Some(json_start) = stdout.find('{') {
                if let Ok(v) = serde_json::from_str::<serde_json::Value>(&stdout[json_start..]) {
                    if let Some(engines) = v.get("engines").and_then(|e| e.as_object()) {
                        let mut max_busy = 0.0;
                        for (name, engine) in engines {
                            if name.contains("Render")
                                || name.contains("3D")
                                || name.contains("Video")
                            {
                                let busy = if let Some(b) = engine.get("busy") {
                                    if let Some(val) = b.as_f64() {
                                        Some(val)
                                    } else { b.get("value").and_then(|v| v.as_f64()) }
                                } else {
                                    None
                                };

                                if let Some(val) = busy {
                                    if val > max_busy {
                                        max_busy = val;
                                    }
                                }
                            }
                        }
                        stats.load = max_busy as f32;
                    }
                }
            }
        }
    }

    // Fallback for Intel: try frequency-based load estimation from sysfs
    if stats.load == 0.0 {
        let act_path = format!(
            "/sys/class/drm/{}/device/drm/{}/gt_act_freq_mhz",
            card_name, card_name
        );
        let max_path = format!(
            "/sys/class/drm/{}/device/drm/{}/gt_max_freq_mhz",
            card_name, card_name
        );

        if let (Ok(act_s), Ok(max_s)) = (
            std::fs::read_to_string(&act_path),
            std::fs::read_to_string(&max_path),
        ) {
            let act: f32 = act_s.trim().parse().unwrap_or(0.0);
            let max: f32 = max_s.trim().parse().unwrap_or(1.0);
            if max > 0.0 {
                stats.load = (act / max) * 100.0;
            }
        } else {
            let act_path = format!("/sys/class/drm/{}/device/gt_act_freq_mhz", card_name);
            let max_path = format!("/sys/class/drm/{}/device/gt_max_freq_mhz", card_name);
            if let (Ok(act_s), Ok(max_s)) = (
                std::fs::read_to_string(&act_path),
                std::fs::read_to_string(&max_path),
            ) {
                let act: f32 = act_s.trim().parse().unwrap_or(0.0);
                let max: f32 = max_s.trim().parse().unwrap_or(1.0);
                stats.load = (act / max) * 100.0;
            }
        }
    }

    // Intel Temperature retrieval (scanning hwmon)
    let device_path = format!("/sys/class/drm/{}/device", card_name);

    // 1. Try standard hwmon in device path
    if scan_hwmon_for_temp(format!("{}/hwmon", device_path), stats) {
        return;
    }

    // 2. Try nested hwmon
    if scan_hwmon_for_temp(format!("{}/device/hwmon", device_path), stats) {
        return;
    }

    // 3. Fallback: Scan all system hwmon for anything that looks like Intel GPU
    if let Ok(hwmon_dir) = std::fs::read_dir("/sys/class/hwmon") {
        for entry in hwmon_dir.flatten() {
            let path = entry.path();
            if let Ok(name) = std::fs::read_to_string(path.join("name")) {
                let name = name.trim();
                if (name == "i915" || name == "xe" || name.contains("intel_gpu"))
                    && scan_hwmon_dir(&path, stats) {
                        return;
                    }
            }
        }
    }

    // 4. Last resort: if it's an Intel card and we still have no temp,
    // it's likely an integrated GPU sharing CPU temperature
    if stats.vendor == "Intel" && stats.temp == 0.0 {
        let components = Components::new_with_refreshed_list();
        for component in components.list() {
            let label = component.label().to_lowercase();
            if label.contains("package") || label.contains("core 0") || label.contains("cpu") {
                if let Some(t) = component.temperature() {
                    stats.temp = t;
                    log::trace!("Using CPU temperature fallback for Intel iGPU: {}°C", t);
                    return;
                }
            }
        }
    }
}

fn scan_hwmon_for_temp(path: String, stats: &mut GpuStats) -> bool {
    if let Ok(hwmon_dir) = std::fs::read_dir(path) {
        for entry in hwmon_dir.flatten() {
            if scan_hwmon_dir(&entry.path(), stats) {
                return true;
            }
        }
    }
    false
}

fn scan_hwmon_dir(path: &std::path::Path, stats: &mut GpuStats) -> bool {
    if let Ok(files) = std::fs::read_dir(path) {
        let mut best_temp = 0.0;
        for file in files.flatten() {
            let file_name = file.file_name().to_string_lossy().to_string();
            if file_name.starts_with("temp") && file_name.ends_with("_input") {
                if let Ok(temp_str) = std::fs::read_to_string(file.path()) {
                    let t = temp_str.trim().parse::<f32>().unwrap_or(0.0) / 1000.0;
                    if t > 0.0 && t < 150.0 {
                        // Check label for "GPU" or "Composite" to be sure, but any temp is better than none
                        let label_path = path.join(file_name.replace("_input", "_label"));
                        let label = std::fs::read_to_string(label_path)
                            .unwrap_or_default()
                            .to_lowercase();

                        if label.contains("gpu") || label.contains("composite") {
                            stats.temp = t;
                            return true;
                        }
                        if t > best_temp {
                            best_temp = t;
                        }
                    }
                }
            }
        }
        if best_temp > 0.0 {
            stats.temp = best_temp;
            return true;
        }
    }
    false
}

pub fn get_gpu_stats() -> Vec<GpuStats> {
    let mut gpus = Vec::new();
    let mut detected_pci_ids = Vec::new();
    // 1. Scan via DRM (preferred for active GPUs)
    log::trace!("Scanning /sys/class/drm for GPUs...");
    if let Ok(entries) = std::fs::read_dir("/sys/class/drm") {
        let mut card_entries: Vec<_> = entries
            .flatten()
            .filter(|e| {
                e.file_name().to_string_lossy().starts_with("card")
                    && !e.file_name().to_string_lossy().contains('-')
            })
            .collect();
        card_entries.sort_by_key(|e| e.file_name());

        for entry in card_entries {
            let card_name = entry.file_name().to_string_lossy().to_string();
            let card_path = entry.path();
            log::trace!("Found DRM device: {}", card_name);

            if let Ok(vendor_id) = std::fs::read_to_string(card_path.join("device/vendor")) {
                let vendor_id = vendor_id.trim();
                let vendor = match vendor_id {
                    "0x10de" => "NVIDIA",
                    "0x1002" => "AMD",
                    "0x8086" => "Intel",
                    _ => "Generic",
                };
                log::trace!("Device {} vendor: {} ({})", card_name, vendor, vendor_id);

                let mut stats = GpuStats {
                    name: format!("{} ({})", vendor, card_name),
                    load: 0.0,
                    vram_used: 0,
                    vram_total: 0,
                    temp: 0.0,
                    vendor: vendor.to_string(),
                    pci_id: String::new(),
                    error: None,
                };

                if let Ok(pci_link) = std::fs::read_link(card_path.join("device")) {
                    if let Some(pci_id) = pci_link.file_name() {
                        let pci_id_str = pci_id.to_string_lossy().to_string();
                        detected_pci_ids.push(pci_id_str.clone());
                        stats.pci_id = pci_id_str.clone();
                        log::trace!("Device {} PCI ID: {}", card_name, pci_id_str);

                        match vendor {
                            "NVIDIA" => fill_nvidia_stats(&mut stats, &pci_id_str),
                            "AMD" => fill_amd_stats(&mut stats, &card_name),
                            "Intel" => fill_intel_stats(&mut stats, &card_name),
                            _ => {}
                        }
                    }
                }
                gpus.push(stats);
            }
        }
    }

    // 2. Scan via PCI (find GPUs without DRM nodes or missing drivers)
    log::trace!("Scanning PCI devices for additional GPUs...");
    let pci_gpus = get_pci_gpus();
    for pci_id in pci_gpus {
        if !detected_pci_ids.contains(&pci_id) {
            log::trace!("Found PCI GPU not in DRM: {}", pci_id);
            let vendor_id =
                std::fs::read_to_string(format!("/sys/bus/pci/devices/{}/vendor", pci_id))
                    .unwrap_or_default()
                    .trim()
                    .to_string();
            let vendor = match vendor_id.as_str() {
                "0x10de" => "NVIDIA",
                "0x1002" => "AMD",
                "0x8086" => "Intel",
                _ => "Generic",
            };

            gpus.push(GpuStats {
                name: format!("{} (PCI {})", vendor, pci_id),
                load: 0.0,
                vram_used: 0,
                vram_total: 0,
                temp: 0.0,
                vendor: vendor.to_string(),
                pci_id: pci_id.clone(),
                error: Some("Driver not active".to_string()),
            });
        }
    }

    // Apply indexing (GPU 0, GPU 1...)
    log::trace!("Finalizing GPU list, count: {}", gpus.len());
    for (i, gpu) in gpus.iter_mut().enumerate() {
        gpu.name = format!("GPU {} - {}", i, gpu.name);
        log::trace!(
            "GPU {}: {} (PCI: {}, Temp: {}°C)",
            i,
            gpu.name,
            gpu.pci_id,
            gpu.temp
        );
    }

    gpus
}

fn get_default_interface() -> String {
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
    let output = Command::new("sudo")
        .args([
            "-n",
            "systemctl",
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
    let output = Command::new("sudo")
        .args(["-n", "docker", "ps", "-q"])
        .output();
    match output {
        Ok(out) => {
            let stdout = String::from_utf8_lossy(&out.stdout);
            stdout.lines().count() as u32
        }
        Err(_) => 0,
    }
}

pub async fn get_system_stats(data: web::Data<AppState>) -> impl Responder {
    let (cpu_usage, ram_total, ram_used, swap_total, swap_used) = {
        let mut sys = data.sys.lock().unwrap();
        sys.refresh_all();
        (
            sys.global_cpu_usage(),
            sys.total_memory(),
            sys.used_memory(),
            sys.total_swap(),
            sys.used_swap(),
        )
    };

    let (disk_total, disk_used) = {
        let disks = Disks::new_with_refreshed_list();
        let mut t = 0;
        let mut u = 0;
        for disk in disks.list() {
            t += disk.total_space();
            u += disk.total_space() - disk.available_space();
        }
        (t, u)
    };

    let (network_rx, network_tx) = {
        let mut networks = data.networks.lock().unwrap();
        networks.refresh(true);
        let mut rx = 0;
        let mut tx = 0;
        for (_interface_name, data) in &*networks {
            rx += data.received();
            tx += data.transmitted();
        }
        (rx, tx)
    };

    let cpu_temp = {
        let mut temp = 0.0;
        let components = Components::new_with_refreshed_list();
        for component in components.list() {
            let label = component.label().to_lowercase();
            if label.contains("cpu") || label.contains("core") || label.contains("package") {
                if let Some(t) = component.temperature() {
                    if t > temp {
                        temp = t;
                    }
                }
            }
        }
        temp
    };

    let (
        active_services,
        failed_services,
        active_containers,
        upgradable_packages,
        network_interface,
        network_max_speed,
        gpus,
    ) = actix_web::web::block(move || {
        let active = count_services("running");
        let failed = count_services("failed");
        let containers = count_containers();
        let pkgs = crate::api::pkgmgr::count_upgradable_packages();
        let iface = get_default_interface();
        let speed = get_interface_speed(&iface);
        let gpus = get_gpu_stats();
        (active, failed, containers, pkgs, iface, speed, gpus)
    })
    .await
    .unwrap_or((0, 0, 0, 0, String::new(), 0, Vec::new()));

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
        gpus,
        cpu_temp,
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
    let cpu_count = sys.cpus().len() as f32;
    let mut processes: Vec<ProcessInfo> = sys
        .processes()
        .iter()
        .map(|(pid, process)| ProcessInfo {
            pid: pid.as_u32(),
            name: process.name().to_string_lossy().into_owned(),
            cpu_usage: process.cpu_usage() / cpu_count,
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
    let output = std::process::Command::new("sudo")
        .args(["-n", "kill", &format!("-{}", signal), &pid.to_string()])
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
