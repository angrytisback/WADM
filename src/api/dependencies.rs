use actix_web::{web, HttpResponse, Responder};
use serde::{Deserialize, Serialize};
use std::process::Command;

#[derive(Serialize, Clone)]
pub struct Dependency {
    pub name: String,
    pub command: String,
    pub installed: bool,
    pub optional: bool,
    pub install_hint: Option<String>,
}

#[derive(Serialize)]
pub struct DependencyReport {
    pub dependencies: Vec<Dependency>,
    pub critical_missing: bool,
}

pub async fn check_dependencies() -> impl Responder {
    let mut deps = Vec::new();

    // Detect GPU vendors to only show relevant GPU tool dependencies
    let mut has_nvidia_gpu = false;
    let mut has_amd_gpu = false;
    let mut has_intel_gpu = false;

    if let Ok(entries) = std::fs::read_dir("/sys/bus/pci/devices/") {
        for entry in entries.flatten() {
            let path = entry.path();
            if let Ok(class) = std::fs::read_to_string(path.join("class")) {
                let class = class.trim();
                if class.starts_with("0x0300")
                    || class.starts_with("0x0302")
                    || class.starts_with("0x0380")
                {
                    if let Ok(vendor_id) = std::fs::read_to_string(path.join("vendor")) {
                        match vendor_id.trim() {
                            "0x10de" => has_nvidia_gpu = true,
                            "0x1002" => has_amd_gpu = true,
                            "0x8086" => has_intel_gpu = true,
                            _ => {}
                        }
                    }
                }
            }
        }
    }

    // 1. Package Manager (Critical)
    let pm_apt = check_command("apt");
    let pm_dnf = check_command("dnf");
    let pm_pacman = check_command("pacman");

    let has_pm = pm_apt || pm_dnf || pm_pacman;

    deps.push(Dependency {
        name: "Package Manager".to_string(),
        command: "apt/dnf/pacman".to_string(),
        installed: has_pm,
        optional: false,
        install_hint: if !has_pm {
            Some("Install apt, dnf, or pacman".to_string())
        } else {
            None
        },
    });

    // 1.5. PCI Utils (Critical for GPU)
    let has_pci = check_command("lspci");
    deps.push(Dependency {
        name: "PCI Utilities".to_string(),
        command: "lspci".to_string(),
        installed: has_pci,
        optional: false,
        install_hint: Some("Install pciutils package".to_string()),
    });

    // 2. S.M.A.R.T. Tools (Optional)
    let has_smart = check_command("smartctl");
    deps.push(Dependency {
        name: "S.M.A.R.T. Tools".to_string(),
        command: "smartctl".to_string(),
        installed: has_smart,
        optional: true,
        install_hint: if !has_smart {
            if pm_apt {
                Some("sudo apt install smartmontools".to_string())
            } else if pm_dnf {
                Some("sudo dnf install smartmontools".to_string())
            } else if pm_pacman {
                Some("sudo pacman -S smartmontools".to_string())
            } else {
                Some("Install smartmontools".to_string())
            }
        } else {
            None
        },
    });

    // 3. UFW Firewall (Optional)
    let has_ufw = check_command("ufw");
    deps.push(Dependency {
        name: "UFW Firewall".to_string(),
        command: "ufw".to_string(),
        installed: has_ufw,
        optional: true,
        install_hint: if !has_ufw {
            if pm_apt {
                Some("sudo apt install ufw".to_string())
            } else if pm_dnf {
                Some("sudo dnf install ufw".to_string())
            } else if pm_pacman {
                Some("sudo pacman -S ufw".to_string())
            } else {
                Some("Install ufw".to_string())
            }
        } else {
            None
        },
    });

    // 4. Docker (Optional)
    let has_docker = check_command("docker");
    deps.push(Dependency {
        name: "Docker".to_string(),
        command: "docker".to_string(),
        installed: has_docker,
        optional: true,
        install_hint: if !has_docker {
            Some("Install Docker engine".to_string())
        } else {
            None
        },
    });

    // 5. Sensors (Optional)
    let has_sensors = check_command("sensors");
    deps.push(Dependency {
        name: "Lm-Sensors".to_string(),
        command: "sensors".to_string(),
        installed: has_sensors,
        optional: true,
        install_hint: if !has_sensors {
            if pm_apt {
                Some("sudo apt install lm-sensors".to_string())
            } else {
                Some("Install lm-sensors".to_string())
            }
        } else {
            None
        },
    });

    // 5.5. Network Tools
    let has_net = check_command("ss") || check_command("netstat");
    deps.push(Dependency {
        name: "Network Tools".to_string(),
        command: "ss/netstat".to_string(),
        installed: has_net,
        optional: false,
        install_hint: Some("Install iproute2 or net-tools".to_string()),
    });
    // 6. NVIDIA GPU Tools (Optional)
    if has_nvidia_gpu {
        let has_nvidia = check_command("nvidia-smi");
        deps.push(Dependency {
            name: "NVIDIA SMI".to_string(),
            command: "nvidia-smi".to_string(),
            installed: has_nvidia,
            optional: true,
            install_hint: if !has_nvidia {
                if pm_apt {
                    Some("sudo apt install nvidia-utils-535".to_string())
                } else {
                    Some("Install NVIDIA drivers/utils".to_string())
                }
            } else {
                None
            },
        });
    }

    // 7. Intel GPU Tools (Optional)
    if has_intel_gpu {
        let has_intel = check_command("intel_gpu_top");
        deps.push(Dependency {
            name: "Intel GPU Tools".to_string(),
            command: "intel_gpu_top".to_string(),
            installed: has_intel,
            optional: true,
            install_hint: if !has_intel {
                if pm_apt {
                    Some("sudo apt install intel-gpu-tools".to_string())
                } else {
                    Some("Install intel-gpu-tools".to_string())
                }
            } else {
                None
            },
        });
    }

    // 8. AMD GPU Tools (Optional)
    if has_amd_gpu {
        let has_amd = check_command("rocm-smi");
        deps.push(Dependency {
            name: "ROCm SMI (AMD)".to_string(),
            command: "rocm-smi".to_string(),
            installed: has_amd,
            optional: true,
            install_hint: if !has_amd {
                Some("Install rocm-smi or use open-source amdgpu drivers".to_string())
            } else {
                None
            },
        });
    }

    let critical_missing = !has_pm;

    HttpResponse::Ok().json(DependencyReport {
        dependencies: deps,
        critical_missing,
    })
}

fn check_command(cmd: &str) -> bool {
    Command::new("which")
        .arg(cmd)
        .output()
        .map(|o| o.status.success())
        .unwrap_or(false)
}

#[derive(Deserialize)]
pub struct InstallReq {
    name: String,
}

pub async fn install_dependency(
    user: crate::auth::AuthenticatedUser,
    audit: web::Data<std::sync::Arc<crate::audit::AuditLogger>>,
    body: web::Json<InstallReq>,
) -> impl Responder {
    if user.require_admin().is_err() {
        audit.log_denied(
            &user,
            "SYSTEM_DEPENDENCY_INSTALL",
            Some(&body.name),
            Some("Requires Admin role"),
        );
        return HttpResponse::Forbidden().json(serde_json::json!({
            "error": "Insufficient permissions"
        }));
    }

    let name = body.name.as_str();

    // Determine package manager first
    let (pm, install_args_prefix) = if check_command("apt") {
        ("apt", vec!["install", "-y"])
    } else if check_command("pacman") {
        ("pacman", vec!["-S", "--noconfirm"])
    } else if check_command("dnf") {
        ("dnf", vec!["install", "-y"])
    } else {
        return HttpResponse::InternalServerError().json("No supported package manager found");
    };

    // Security & Distro Awareness: Map dependency to distro-specific package name and command check
    let (package_name, cmd_check) = match (name, pm) {
        ("S.M.A.R.T. Tools", _) => ("smartmontools", "smartctl"),
        ("UFW Firewall", _) => ("ufw", "ufw"),
        ("Lm-Sensors", "pacman") => ("lm_sensors", "sensors"),
        ("Lm-Sensors", _) => ("lm-sensors", "sensors"),
        ("Docker", "pacman") => ("docker", "docker"),
        ("Docker", "dnf") => ("docker", "docker"),
        ("Docker", _) => ("docker.io", "docker"),
        ("Intel GPU Tools", _) => ("intel-gpu-tools", "intel_gpu_top"),
        _ => return HttpResponse::BadRequest().json("Invalid dependency name"),
    };

    if check_command(cmd_check) {
        return HttpResponse::Ok().json(format!("{} is already installed.", name));
    }

    let mut install_cmd = install_args_prefix;
    install_cmd.push(package_name);

    // Execute
    // Note: This requires the user running WADM to have sudo NOPASSWD or be root.
    // If not root, we try sudo.
    let output = Command::new("sudo")
        .arg("-n")
        .arg(pm)
        .args(&install_cmd)
        .output();

    match output {
        Ok(o) => {
            let stdout = String::from_utf8_lossy(&o.stdout);
            let stderr = String::from_utf8_lossy(&o.stderr);
            if o.status.success() {
                audit.log(
                    &user.username,
                    user.role.as_str(),
                    "SYSTEM_DEPENDENCY_INSTALL",
                    Some(name),
                    &user.client_ip,
                    "SUCCESS",
                    Some(&format!("Package: {}", package_name)),
                );
                HttpResponse::Ok().json(format!(
                    "Successfully installed {}.\n\nOutput:\n{}",
                    name, stdout
                ))
            } else {
                audit.log(
                    &user.username,
                    user.role.as_str(),
                    "SYSTEM_DEPENDENCY_INSTALL",
                    Some(name),
                    &user.client_ip,
                    "FAILED",
                    Some(&stderr),
                );
                HttpResponse::InternalServerError().json(format!(
                    "Failed to install {}.\n\nError:\n{}\nOutput:\n{}",
                    name, stderr, stdout
                ))
            }
        }
        Err(e) => {
            audit.log(
                &user.username,
                user.role.as_str(),
                "SYSTEM_DEPENDENCY_INSTALL",
                Some(name),
                &user.client_ip,
                "FAILED",
                Some(&e.to_string()),
            );
            HttpResponse::InternalServerError()
                .json(format!("Failed to execute installation command: {}", e))
        }
    }
}
