use actix_web::{web, HttpResponse, Responder};
use log::info;
use serde::{Deserialize, Serialize};
use std::process::Command;

#[derive(Serialize)]
struct Package {
    name: String,
    version: String,
    status: String,
}

#[derive(Deserialize)]
pub struct PackageAction {
    name: String,
}

#[derive(Serialize)]
struct ErrorResponse {
    error: String,
}

#[derive(Debug)]
pub enum ManagerType {
    Apt,
    Dnf,
    Pacman,
    Unknown,
}

pub fn detect_manager() -> ManagerType {
    if Command::new("which")
        .arg("apt-get")
        .output()
        .map(|o| o.status.success())
        .unwrap_or(false)
    {
        return ManagerType::Apt;
    }
    if Command::new("which")
        .arg("dnf")
        .output()
        .map(|o| o.status.success())
        .unwrap_or(false)
    {
        return ManagerType::Dnf;
    }
    if Command::new("which")
        .arg("pacman")
        .output()
        .map(|o| o.status.success())
        .unwrap_or(false)
    {
        return ManagerType::Pacman;
    }
    ManagerType::Unknown
}

fn list_packages_pacman() -> Result<Vec<Package>, String> {
    let output = Command::new("sudo")
        .args(["-n", "pacman", "-Qu"])
        .output()
        .map_err(|e| e.to_string())?;

    if !output.status.success() {
        if output.status.code() == Some(1) {
            return Ok(Vec::new());
        }
        return Err(String::from_utf8_lossy(&output.stderr).to_string());
    }

    let stdout = String::from_utf8_lossy(&output.stdout);
    let packages = stdout
        .lines()
        .filter(|l| !l.is_empty())
        .filter_map(|line| {
            let parts: Vec<&str> = line.split_whitespace().collect();
            if !parts.is_empty() {
                let name = parts[0].to_string();
                let version = if parts.len() >= 4 {
                    parts[3].to_string()
                } else {
                    "latest".to_string()
                };
                Some(Package {
                    name,
                    version,
                    status: "upgradable".to_string(),
                })
            } else {
                None
            }
        })
        .collect();
    Ok(packages)
}

fn list_packages_apt() -> Result<Vec<Package>, String> {
    let output = Command::new("sudo")
        .args(["-n", "apt", "list", "--upgradable"])
        .output()
        .map_err(|e| e.to_string())?;

    if !output.status.success() {
        return Err(String::from_utf8_lossy(&output.stderr).to_string());
    }

    let stdout = String::from_utf8_lossy(&output.stdout);
    let packages = stdout
        .lines()
        .skip(1)
        .filter(|l| !l.is_empty())
        .filter_map(|line| {
            let parts: Vec<&str> = line.split_whitespace().collect();
            if parts.len() >= 2 {
                let name = parts[0].split('/').next().unwrap_or(parts[0]).to_string();
                let version = parts[1].to_string();
                Some(Package {
                    name,
                    version,
                    status: "upgradable".to_string(),
                })
            } else {
                None
            }
        })
        .collect();
    Ok(packages)
}

fn list_packages_dnf() -> Result<Vec<Package>, String> {
    let output = Command::new("sudo")
        .args(["-n", "dnf", "check-update"])
        .output()
        .map_err(|e| e.to_string())?;

    let code = output.status.code();
    if code == Some(0) {
        return Ok(Vec::new());
    } else if code != Some(100) {
        return Err(String::from_utf8_lossy(&output.stderr).to_string());
    }

    let stdout = String::from_utf8_lossy(&output.stdout);
    let packages = stdout
        .lines()
        .filter(|l| {
            !l.is_empty()
                && !l.starts_with("Last metadata expiration check")
                && !l.starts_with("Obsoleting Packages")
        })
        .filter_map(|line| {
            let parts: Vec<&str> = line.split_whitespace().collect();
            if parts.len() >= 2 {
                let name = parts[0].to_string();
                let version = parts[1].to_string();
                Some(Package {
                    name,
                    version,
                    status: "upgradable".to_string(),
                })
            } else {
                None
            }
        })
        .collect();
    Ok(packages)
}

fn is_valid_package_name(name: &str) -> bool {
    !name.is_empty()
        && !name.starts_with('-')
        && name
            .chars()
            .all(|c| c.is_ascii_alphanumeric() || c == '_' || c == '-' || c == '.')
}

fn upgrade_package_impl(manager: &ManagerType, name: &str) -> Result<String, String> {
    if !is_valid_package_name(name) {
        return Err("Invalid package name".to_string());
    }
    let (cmd, args) = match manager {
        ManagerType::Pacman => ("pacman", vec!["-S", "--noconfirm", name]),
        ManagerType::Apt => ("apt-get", vec!["install", "-y", "--only-upgrade", name]),
        ManagerType::Dnf => ("dnf", vec!["upgrade", "-y", name]),
        ManagerType::Unknown => return Err("Unknown package manager".to_string()),
    };

    info!("Attempting to update package: {} using {:?}", name, manager);
    let output = Command::new("sudo")
        .args(["-n", cmd])
        .args(args)
        .output()
        .map_err(|e| e.to_string())?;

    if output.status.success() {
        info!("Successfully updated package: {}", name);
        Ok(format!("Package {} updated successfully", name))
    } else {
        Err(String::from_utf8_lossy(&output.stderr).to_string())
    }
}

fn install_package_impl(manager: &ManagerType, name: &str) -> Result<String, String> {
    if !is_valid_package_name(name) {
        return Err("Invalid package name".to_string());
    }
    let (cmd, args) = match manager {
        ManagerType::Pacman => ("pacman", vec!["-S", "--noconfirm", name]),
        ManagerType::Apt => ("apt-get", vec!["install", "-y", name]),
        ManagerType::Dnf => ("dnf", vec!["install", "-y", name]),
        ManagerType::Unknown => return Err("Unknown package manager".to_string()),
    };

    info!(
        "Attempting to install package: {} using {:?}",
        name, manager
    );
    let output = Command::new("sudo")
        .args(["-n", cmd])
        .args(args)
        .output()
        .map_err(|e| e.to_string())?;

    if output.status.success() {
        info!("Successfully installed package: {}", name);
        Ok(format!("Package {} installed successfully", name))
    } else {
        Err(String::from_utf8_lossy(&output.stderr).to_string())
    }
}

pub fn count_upgradable_packages() -> u32 {
    let manager = detect_manager();
    let result = match manager {
        ManagerType::Pacman => list_packages_pacman(),
        ManagerType::Apt => list_packages_apt(),
        ManagerType::Dnf => list_packages_dnf(),
        ManagerType::Unknown => Ok(Vec::new()),
    };

    match result {
        Ok(pkgs) => pkgs.len() as u32,
        Err(_) => 0,
    }
}

pub async fn list_packages() -> impl Responder {
    log::debug!("Request: list_packages");
    let manager = detect_manager();
    log::debug!("Using manager: {:?}", manager);
    let result = match manager {
        ManagerType::Pacman => list_packages_pacman(),
        ManagerType::Apt => list_packages_apt(),
        ManagerType::Dnf => list_packages_dnf(),
        ManagerType::Unknown => {
            Err("No supported package manager found (pacman, apt, dnf)".to_string())
        }
    };

    match result {
        Ok(pkgs) => {
            log::debug!("Found {} upgradable packages", pkgs.len());
            HttpResponse::Ok().json(pkgs)
        }
        Err(e) => {
            log::error!("List packages failed: {}", e);
            HttpResponse::InternalServerError().json(ErrorResponse { error: e })
        }
    }
}

pub async fn upgrade_package(body: web::Json<PackageAction>) -> impl Responder {
    log::info!("Request: upgrade_package {}", body.name);
    let manager = detect_manager();
    match upgrade_package_impl(&manager, &body.name) {
        Ok(msg) => {
            log::info!("Upgrade success: {}", msg);
            HttpResponse::Ok().json(msg)
        }
        Err(e) => {
            log::error!("Upgrade package failed: {}", e);
            HttpResponse::InternalServerError().json(ErrorResponse { error: e })
        }
    }
}

pub async fn install_package(body: web::Json<PackageAction>) -> impl Responder {
    log::info!("Request: install_package {}", body.name);
    let manager = detect_manager();
    match install_package_impl(&manager, &body.name) {
        Ok(msg) => {
            log::info!("Install success: {}", msg);
            HttpResponse::Ok().json(msg)
        }
        Err(e) => {
            log::error!("Install package failed: {}", e);
            HttpResponse::InternalServerError().json(ErrorResponse { error: e })
        }
    }
}

fn list_installed_packages_pacman() -> Result<Vec<Package>, String> {
    let output = Command::new("sudo")
        .args(["-n", "pacman", "-Q"])
        .output()
        .map_err(|e| e.to_string())?;

    if !output.status.success() {
        return Err(String::from_utf8_lossy(&output.stderr).to_string());
    }

    let stdout = String::from_utf8_lossy(&output.stdout);
    let packages = stdout
        .lines()
        .filter_map(|line| {
            let parts: Vec<&str> = line.split_whitespace().collect();
            if parts.len() >= 2 {
                Some(Package {
                    name: parts[0].to_string(),
                    version: parts[1].to_string(),
                    status: "installed".to_string(),
                })
            } else {
                None
            }
        })
        .collect();
    Ok(packages)
}

fn list_installed_packages_apt() -> Result<Vec<Package>, String> {
    let output = Command::new("dpkg-query")
        .arg("-W")
        .arg("-f=${binary:Package} ${Version}\n")
        .output()
        .map_err(|e| e.to_string())?;

    if !output.status.success() {
        return Err(String::from_utf8_lossy(&output.stderr).to_string());
    }

    let stdout = String::from_utf8_lossy(&output.stdout);
    let packages = stdout
        .lines()
        .filter_map(|line| {
            let parts: Vec<&str> = line.split_whitespace().collect();
            if parts.len() >= 2 {
                Some(Package {
                    name: parts[0].to_string(),
                    version: parts[1].to_string(),
                    status: "installed".to_string(),
                })
            } else {
                None
            }
        })
        .collect();
    Ok(packages)
}

fn list_installed_packages_dnf() -> Result<Vec<Package>, String> {
    let output = Command::new("sudo")
        .args(["-n", "dnf", "list", "installed", "-q"])
        .output()
        .map_err(|e| e.to_string())?;

    if !output.status.success() {
        return Err(String::from_utf8_lossy(&output.stderr).to_string());
    }

    let stdout = String::from_utf8_lossy(&output.stdout);
    let packages = stdout
        .lines()
        .filter(|l| !l.starts_with("Installed Packages"))
        .filter_map(|line| {
            let parts: Vec<&str> = line.split_whitespace().collect();
            if parts.len() >= 2 {
                let name = parts[0].split('.').next().unwrap_or(parts[0]).to_string();
                Some(Package {
                    name,
                    version: parts[1].to_string(),
                    status: "installed".to_string(),
                })
            } else {
                None
            }
        })
        .collect();
    Ok(packages)
}

fn update_all_packages_impl(manager: &ManagerType) -> Result<String, String> {
    let (cmd, args) = match manager {
        ManagerType::Pacman => ("pacman", vec!["-Syu", "--noconfirm"]),
        ManagerType::Apt => ("apt-get", vec!["upgrade", "-y"]),
        ManagerType::Dnf => ("dnf", vec!["upgrade", "-y"]),
        ManagerType::Unknown => return Err("Unknown package manager".to_string()),
    };

    if let ManagerType::Apt = manager {
        let _ = Command::new("sudo")
            .args(["-n", "apt-get", "update"])
            .output();
    }

    let output = Command::new("sudo")
        .args(["-n", cmd])
        .args(args)
        .output()
        .map_err(|e| e.to_string())?;

    if output.status.success() {
        Ok("System updated successfully".to_string())
    } else {
        Err(String::from_utf8_lossy(&output.stderr).to_string())
    }
}

fn remove_package_dry_run_impl(manager: &ManagerType, name: &str) -> Result<String, String> {
    if !is_valid_package_name(name) {
        return Err("Invalid package name".to_string());
    }
    match manager {
        ManagerType::Pacman => {
            let output = Command::new("sudo")
                .args(["-n", "pacman", "-Rns", name, "-p"])
                .output()
                .map_err(|e| e.to_string())?;
            Ok(String::from_utf8_lossy(&output.stdout).to_string())
        }
        ManagerType::Apt => {
            let output = Command::new("sudo")
                .args(["-n", "apt-get", "remove", "-s", name])
                .output()
                .map_err(|e| e.to_string())?;
            Ok(String::from_utf8_lossy(&output.stdout).to_string())
        }
        ManagerType::Dnf => {
            let output = Command::new("sudo")
                .args(["-n", "dnf", "remove", name, "--assumeno"])
                .output()
                .map_err(|e| e.to_string())?;

            Ok(String::from_utf8_lossy(&output.stdout).to_string())
        }
        ManagerType::Unknown => Err("Unknown package manager".to_string()),
    }
}

fn remove_package_impl(manager: &ManagerType, name: &str) -> Result<String, String> {
    if !is_valid_package_name(name) {
        return Err("Invalid package name".to_string());
    }
    let (cmd, args) = match manager {
        ManagerType::Pacman => ("pacman", vec!["-Rns", "--noconfirm", name]),
        ManagerType::Apt => ("apt-get", vec!["remove", "-y", name]),
        ManagerType::Dnf => ("dnf", vec!["remove", "-y", name]),
        ManagerType::Unknown => return Err("Unknown package manager".to_string()),
    };

    info!("Attempting to remove package: {} using {:?}", name, manager);
    let output = Command::new("sudo")
        .args(["-n", cmd])
        .args(args)
        .output()
        .map_err(|e| e.to_string())?;

    if output.status.success() {
        info!("Successfully removed package: {}", name);
        Ok(format!("Package {} removed successfully", name))
    } else {
        Err(String::from_utf8_lossy(&output.stderr).to_string())
    }
}

pub async fn list_installed_packages() -> impl Responder {
    log::debug!("Request: list_installed_packages");
    let manager = detect_manager();
    let result = match manager {
        ManagerType::Pacman => list_installed_packages_pacman(),
        ManagerType::Apt => list_installed_packages_apt(),
        ManagerType::Dnf => list_installed_packages_dnf(),
        ManagerType::Unknown => Ok(Vec::new()),
    };

    match result {
        Ok(pkgs) => {
            log::debug!("Found {} installed packages", pkgs.len());
            HttpResponse::Ok().json(pkgs)
        }
        Err(e) => {
            log::error!("List installed packages failed: {}", e);
            HttpResponse::InternalServerError().json(ErrorResponse { error: e })
        }
    }
}

pub async fn update_all_packages() -> impl Responder {
    log::info!("Request: update_all_packages");
    let manager = detect_manager();
    match update_all_packages_impl(&manager) {
        Ok(msg) => {
            log::info!("Global update success: {}", msg);
            HttpResponse::Ok().json(msg)
        }
        Err(e) => {
            log::error!("Global update failed: {}", e);
            HttpResponse::InternalServerError().json(ErrorResponse { error: e })
        }
    }
}

pub async fn remove_package_dry_run(body: web::Json<PackageAction>) -> impl Responder {
    log::debug!("Request: remove_package_dry_run {}", body.name);
    let manager = detect_manager();
    match remove_package_dry_run_impl(&manager, &body.name) {
        Ok(output) => {
            log::debug!("Remove dry run success for {}", body.name);
            HttpResponse::Ok().json(output)
        }
        Err(e) => {
            log::error!("Remove dry run failed: {}", e);
            HttpResponse::InternalServerError().json(ErrorResponse { error: e })
        }
    }
}

pub async fn remove_package(body: web::Json<PackageAction>) -> impl Responder {
    log::info!("Request: remove_package {}", body.name);
    let manager = detect_manager();
    match remove_package_impl(&manager, &body.name) {
        Ok(msg) => {
            log::info!("Remove success: {}", msg);
            HttpResponse::Ok().json(msg)
        }
        Err(e) => {
            log::error!("Remove package failed: {}", e);
            HttpResponse::InternalServerError().json(ErrorResponse { error: e })
        }
    }
}
