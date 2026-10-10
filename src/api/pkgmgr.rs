use actix_web::{web, HttpResponse, Responder};
use log::info;
use once_cell::sync::Lazy;
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use std::sync::Mutex;
use std::time::{Duration, Instant};
use tokio::process::Command;

pub use crate::drivers::package::is_valid_package_name;
#[allow(unused_imports)]
pub use crate::drivers::package::PackageUpdate as Package;
use crate::drivers::DriverRegistry;

static UPGRADABLE_CACHE: Lazy<Mutex<(u32, Instant)>> =
    Lazy::new(|| Mutex::new((0, Instant::now() - Duration::from_secs(3600))));

pub fn invalidate_upgradable_cache() {
    if let Ok(mut cache) = UPGRADABLE_CACHE.lock() {
        cache.1 = Instant::now() - Duration::from_secs(3600);
    }
}

#[derive(Deserialize)]
pub struct PackageAction {
    pub name: String,
}

#[derive(Serialize)]
struct ErrorResponse {
    error: String,
}

#[allow(dead_code)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ManagerType {
    Apt,
    Dnf,
    Pacman,
    Unknown,
}

#[allow(dead_code)]
pub fn detect_manager() -> ManagerType {
    let registry = DriverRegistry::detect_and_init();
    match registry.package_manager.name() {
        "apt" => ManagerType::Apt,
        "dnf" => ManagerType::Dnf,
        "pacman" => ManagerType::Pacman,
        _ => ManagerType::Unknown,
    }
}

pub fn count_upgradable_packages() -> u32 {
    const TTL: Duration = Duration::from_secs(300); // 5-minute cache
    if let Ok(cache) = UPGRADABLE_CACHE.lock() {
        if cache.1.elapsed() < TTL {
            return cache.0;
        }
    }

    let registry = DriverRegistry::detect_and_init();
    let count = match tokio::runtime::Handle::try_current() {
        Ok(handle) => tokio::task::block_in_place(|| {
            handle.block_on(async {
                registry
                    .package_manager
                    .check_updates()
                    .await
                    .map(|pkgs| pkgs.len() as u32)
                    .unwrap_or(0)
            })
        }),
        Err(_) => {
            // When not in a tokio runtime context (e.g. testing)
            0
        }
    };

    if let Ok(mut cache) = UPGRADABLE_CACHE.lock() {
        *cache = (count, Instant::now());
    }

    count
}

pub async fn list_packages(registry: web::Data<DriverRegistry>) -> impl Responder {
    log::debug!(
        "Request: list_packages via {}",
        registry.package_manager.name()
    );
    match registry.package_manager.check_updates().await {
        Ok(pkgs) => {
            log::debug!("Found {} upgradable packages", pkgs.len());
            HttpResponse::Ok().json(pkgs)
        }
        Err(e) => {
            log::error!("List packages failed: {}", e);
            HttpResponse::InternalServerError().json(ErrorResponse {
                error: e.to_string(),
            })
        }
    }
}

pub async fn list_installed_packages(registry: web::Data<DriverRegistry>) -> impl Responder {
    log::debug!(
        "Request: list_installed_packages via {}",
        registry.package_manager.name()
    );
    match registry.package_manager.list_installed().await {
        Ok(pkgs) => {
            log::debug!("Found {} installed packages", pkgs.len());
            HttpResponse::Ok().json(pkgs)
        }
        Err(e) => {
            log::error!("List installed packages failed: {}", e);
            HttpResponse::InternalServerError().json(ErrorResponse {
                error: e.to_string(),
            })
        }
    }
}

pub async fn upgrade_package(
    user: crate::auth::AuthenticatedUser,
    audit: web::Data<std::sync::Arc<crate::audit::AuditLogger>>,
    body: web::Json<PackageAction>,
    registry: web::Data<DriverRegistry>,
) -> impl Responder {
    log::info!("Request: upgrade_package {}", body.name);
    if user.require_admin().is_err() {
        audit.log_denied(
            &user,
            "PACKAGE_UPGRADE",
            Some(&body.name),
            Some("Requires Admin role"),
        );
        return HttpResponse::Forbidden().json(ErrorResponse {
            error: "Insufficient permissions".to_string(),
        });
    }

    if !is_valid_package_name(&body.name) {
        return HttpResponse::BadRequest().json(ErrorResponse {
            error: "Invalid package name".to_string(),
        });
    }

    let (cmd, args) = match registry.package_manager.name() {
        "pacman" => (
            "sudo",
            vec!["-n", "pacman", "-S", "--noconfirm", &body.name],
        ),
        "apt" => (
            "sudo",
            vec![
                "-n",
                "apt-get",
                "install",
                "-y",
                "--only-upgrade",
                &body.name,
            ],
        ),
        "dnf" => ("sudo", vec!["-n", "dnf", "upgrade", "-y", &body.name]),
        _ => {
            return HttpResponse::BadRequest().json(ErrorResponse {
                error: "Unknown package manager".to_string(),
            })
        }
    };

    let output = Command::new(cmd).args(&args).output().await;
    match output {
        Ok(o) if o.status.success() => {
            info!("Successfully updated package: {}", body.name);
            invalidate_upgradable_cache();
            audit.log(
                &user.username,
                user.role.as_str(),
                "PACKAGE_UPGRADE",
                Some(&body.name),
                &user.client_ip,
                "SUCCESS",
                None,
            );
            HttpResponse::Ok().json(format!("Package {} updated successfully", body.name))
        }
        Ok(o) => {
            let stderr = String::from_utf8_lossy(&o.stderr).to_string();
            log::error!("Upgrade package failed: {}", stderr);
            audit.log(
                &user.username,
                user.role.as_str(),
                "PACKAGE_UPGRADE",
                Some(&body.name),
                &user.client_ip,
                "FAILED",
                Some(&stderr),
            );
            HttpResponse::InternalServerError().json(ErrorResponse { error: stderr })
        }
        Err(e) => {
            log::error!("Upgrade package process failed: {}", e);
            audit.log(
                &user.username,
                user.role.as_str(),
                "PACKAGE_UPGRADE",
                Some(&body.name),
                &user.client_ip,
                "FAILED",
                Some(&e.to_string()),
            );
            HttpResponse::InternalServerError().json(ErrorResponse {
                error: e.to_string(),
            })
        }
    }
}

pub async fn install_package(
    user: crate::auth::AuthenticatedUser,
    audit: web::Data<std::sync::Arc<crate::audit::AuditLogger>>,
    body: web::Json<PackageAction>,
    registry: web::Data<DriverRegistry>,
) -> impl Responder {
    log::info!("Request: install_package {}", body.name);
    if user.require_admin().is_err() {
        audit.log_denied(
            &user,
            "PACKAGE_INSTALL",
            Some(&body.name),
            Some("Requires Admin role"),
        );
        return HttpResponse::Forbidden().json(ErrorResponse {
            error: "Insufficient permissions".to_string(),
        });
    }

    if !is_valid_package_name(&body.name) {
        return HttpResponse::BadRequest().json(ErrorResponse {
            error: "Invalid package name".to_string(),
        });
    }

    let (cmd, args) = registry.package_manager.build_install_command(&body.name);
    let output = Command::new(&cmd).args(&args).output().await;
    match output {
        Ok(o) if o.status.success() => {
            info!("Successfully installed package: {}", body.name);
            invalidate_upgradable_cache();
            audit.log(
                &user.username,
                user.role.as_str(),
                "PACKAGE_INSTALL",
                Some(&body.name),
                &user.client_ip,
                "SUCCESS",
                None,
            );
            HttpResponse::Ok().json(format!("Package {} installed successfully", body.name))
        }
        Ok(o) => {
            let stderr = String::from_utf8_lossy(&o.stderr).to_string();
            log::error!("Install package failed: {}", stderr);
            audit.log(
                &user.username,
                user.role.as_str(),
                "PACKAGE_INSTALL",
                Some(&body.name),
                &user.client_ip,
                "FAILED",
                Some(&stderr),
            );
            HttpResponse::InternalServerError().json(ErrorResponse { error: stderr })
        }
        Err(e) => {
            log::error!("Install package process failed: {}", e);
            audit.log(
                &user.username,
                user.role.as_str(),
                "PACKAGE_INSTALL",
                Some(&body.name),
                &user.client_ip,
                "FAILED",
                Some(&e.to_string()),
            );
            HttpResponse::InternalServerError().json(ErrorResponse {
                error: e.to_string(),
            })
        }
    }
}

pub async fn remove_package(
    user: crate::auth::AuthenticatedUser,
    audit: web::Data<std::sync::Arc<crate::audit::AuditLogger>>,
    body: web::Json<PackageAction>,
    registry: web::Data<DriverRegistry>,
) -> impl Responder {
    log::info!("Request: remove_package {}", body.name);
    if user.require_admin().is_err() {
        audit.log_denied(
            &user,
            "PACKAGE_REMOVE",
            Some(&body.name),
            Some("Requires Admin role"),
        );
        return HttpResponse::Forbidden().json(ErrorResponse {
            error: "Insufficient permissions".to_string(),
        });
    }

    if !is_valid_package_name(&body.name) {
        return HttpResponse::BadRequest().json(ErrorResponse {
            error: "Invalid package name".to_string(),
        });
    }

    let (cmd, args) = registry
        .package_manager
        .build_remove_command(&body.name, false);
    let output = Command::new(&cmd).args(&args).output().await;
    match output {
        Ok(o) if o.status.success() => {
            info!("Successfully removed package: {}", body.name);
            invalidate_upgradable_cache();
            audit.log(
                &user.username,
                user.role.as_str(),
                "PACKAGE_REMOVE",
                Some(&body.name),
                &user.client_ip,
                "SUCCESS",
                None,
            );
            HttpResponse::Ok().json(format!("Package {} removed successfully", body.name))
        }
        Ok(o) => {
            let stderr = String::from_utf8_lossy(&o.stderr).to_string();
            log::error!("Remove package failed: {}", stderr);
            audit.log(
                &user.username,
                user.role.as_str(),
                "PACKAGE_REMOVE",
                Some(&body.name),
                &user.client_ip,
                "FAILED",
                Some(&stderr),
            );
            HttpResponse::InternalServerError().json(ErrorResponse { error: stderr })
        }
        Err(e) => {
            log::error!("Remove package process failed: {}", e);
            audit.log(
                &user.username,
                user.role.as_str(),
                "PACKAGE_REMOVE",
                Some(&body.name),
                &user.client_ip,
                "FAILED",
                Some(&e.to_string()),
            );
            HttpResponse::InternalServerError().json(ErrorResponse {
                error: e.to_string(),
            })
        }
    }
}

pub async fn remove_package_dry_run(
    user: crate::auth::AuthenticatedUser,
    audit: web::Data<std::sync::Arc<crate::audit::AuditLogger>>,
    body: web::Json<PackageAction>,
    registry: web::Data<DriverRegistry>,
) -> impl Responder {
    log::debug!("Request: remove_package_dry_run {}", body.name);
    if user.require_admin().is_err() {
        audit.log_denied(
            &user,
            "PACKAGE_REMOVE_DRY_RUN",
            Some(&body.name),
            Some("Requires Admin role"),
        );
        return HttpResponse::Forbidden().json(ErrorResponse {
            error: "Insufficient permissions".to_string(),
        });
    }

    match registry.package_manager.remove_dry_run(&body.name).await {
        Ok(output) => {
            log::debug!("Remove dry run success for {}", body.name);
            HttpResponse::Ok().json(output)
        }
        Err(e) => {
            log::error!("Remove dry run failed: {}", e);
            HttpResponse::InternalServerError().json(ErrorResponse {
                error: e.to_string(),
            })
        }
    }
}

pub async fn update_all_packages(
    user: crate::auth::AuthenticatedUser,
    audit: web::Data<std::sync::Arc<crate::audit::AuditLogger>>,
    job_manager: web::Data<Arc<crate::api::jobs::JobManager>>,
    registry: web::Data<DriverRegistry>,
) -> impl Responder {
    let pm_name = registry.package_manager.name();
    log::info!(
        "Request: update_all_packages via {} (enqueue as background job)",
        pm_name
    );
    if user.require_admin().is_err() {
        audit.log_denied(
            &user,
            "PACKAGE_UPDATE_ALL",
            Some(pm_name),
            Some("Requires Admin role"),
        );
        return HttpResponse::Forbidden().json(ErrorResponse {
            error: "Insufficient permissions".to_string(),
        });
    }

    let (cmd, args) = registry.package_manager.build_upgrade_command();

    audit.log(
        &user.username,
        user.role.as_str(),
        "PACKAGE_UPDATE_ALL",
        Some(pm_name),
        &user.client_ip,
        "SUCCESS",
        Some("Enqueued update-all background job"),
    );

    match job_manager
        .enqueue_job(
            "package_upgrade",
            crate::api::jobs::JobTaskPayload::CommandExecution {
                cmd,
                args,
                output_file: None,
                description: format!("System packages upgrade via {}", pm_name),
            },
        )
        .await
    {
        Ok(job_id) => HttpResponse::Accepted().json(crate::api::jobs::JobActionResponse {
            job_id,
            status: "pending".to_string(),
            message: "Package update queued successfully".to_string(),
        }),
        Err(e) => {
            log::error!("Failed to enqueue package update job: {}", e);
            HttpResponse::InternalServerError().json(ErrorResponse { error: e })
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_valid_package_names() {
        assert!(is_valid_package_name("nginx"));
        assert!(is_valid_package_name("curl-7.68.0"));
        assert!(is_valid_package_name("libssl_dev.1"));
        assert!(is_valid_package_name("python3"));
    }

    #[test]
    fn test_invalid_package_names_injection() {
        assert!(!is_valid_package_name(""));
        assert!(!is_valid_package_name("-oAPT::Update=1"));
        assert!(!is_valid_package_name("--force"));
        assert!(!is_valid_package_name("nginx; rm -rf /"));
        assert!(!is_valid_package_name("nginx | bash"));
        assert!(!is_valid_package_name("package`whoami`"));
        assert!(!is_valid_package_name("pkg name"));
    }
}
