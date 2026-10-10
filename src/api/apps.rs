use actix_web::{web, HttpResponse, Responder};
use log::info;
use serde::{Deserialize, Serialize};
use std::fs;
use std::net::TcpListener;
use std::process::Command;
use std::sync::Arc;

use crate::apps::manifest::get_app_templates;
use crate::audit::AuditLogger;
use crate::auth::AuthenticatedUser;
use crate::drivers::error::AppError;
use crate::drivers::firewall::FirewallRuleRequest;
use crate::drivers::DriverRegistry;
use crate::proxy::{ProxyDatabase, ProxyRoute};

pub fn is_port_available(port: u16) -> bool {
    TcpListener::bind(("127.0.0.1", port)).is_ok()
}

pub fn find_available_localhost_port() -> Result<u16, AppError> {
    // Search within private localhost block 18000..=18999
    for port in 18000..=18999 {
        if is_port_available(port) {
            return Ok(port);
        }
    }
    // Fallback: ephemeral OS-assigned loopback port
    let listener = TcpListener::bind(("127.0.0.1", 0)).map_err(|e| {
        AppError::ExecutionFailed(format!("Failed to bind local loopback port: {}", e))
    })?;
    let port = listener
        .local_addr()
        .map_err(|e| AppError::ExecutionFailed(e.to_string()))?
        .port();
    Ok(port)
}

fn generate_secure_password() -> String {
    use rand::Rng;
    const CHARSET: &[u8] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789";
    let mut rng = rand::thread_rng();
    (0..20)
        .map(|_| {
            let idx = rng.gen_range(0..CHARSET.len());
            CHARSET[idx] as char
        })
        .collect()
}

pub fn is_valid_app_id(id: &str) -> bool {
    !id.is_empty()
        && !id.starts_with('.')
        && !id.starts_with('-')
        && !id.contains('/')
        && !id.contains('\\')
        && id.len() <= 64
        && id
            .chars()
            .all(|c| c.is_ascii_alphanumeric() || c == '-' || c == '_')
}

pub fn get_apps_dir() -> std::path::PathBuf {
    if let Ok(data_dir) = std::env::var("WADM_DATA_DIR") {
        return std::path::PathBuf::from(data_dir).join("apps");
    }
    let system_dir = std::path::Path::new("/var/lib/wadm/apps");
    if system_dir.exists() || std::fs::create_dir_all(system_dir).is_ok() {
        system_dir.to_path_buf()
    } else {
        std::env::current_dir()
            .unwrap_or_default()
            .join("data")
            .join("apps")
    }
}

pub async fn list_apps() -> impl Responder {
    HttpResponse::Ok().json(get_app_templates())
}

#[derive(Debug, Deserialize)]
pub struct AppInstallRequest {
    pub id: String,
    #[serde(default = "default_access_mode")]
    pub access_mode: String, // "path" | "subdomain" | "none"
    #[serde(default)]
    pub domain: Option<String>,
    #[serde(default)]
    pub allow_exposed_ports: bool,
    #[serde(default)]
    pub approved_ports: Vec<u16>,
}

fn default_access_mode() -> String {
    "path".to_string()
}

#[derive(Serialize)]
pub struct AppCredentialsResponse {
    pub id: String,
    pub credentials: Option<String>,
    pub installed: bool,
    pub access_url: Option<String>,
    pub access_mode: Option<String>,
    pub internal_port: Option<u16>,
}

pub async fn get_app_credentials(
    user: AuthenticatedUser,
    audit: web::Data<Arc<AuditLogger>>,
    proxy_db: web::Data<Arc<ProxyDatabase>>,
    path: web::Path<String>,
) -> impl Responder {
    let id = path.into_inner();

    if user.require_admin().is_err() {
        audit.log_denied(
            &user,
            "APP_CREDENTIALS",
            Some(&id),
            Some("Requires Admin role"),
        );
        return HttpResponse::Forbidden().json(serde_json::json!({
            "error": "Insufficient permissions"
        }));
    }

    if !is_valid_app_id(&id) {
        return HttpResponse::BadRequest().json("Invalid app identifier");
    }

    let apps_dir = get_apps_dir();
    let creds_file = apps_dir.join(&id).join("credentials.txt");
    let compose_file = apps_dir.join(&id).join("docker-compose.yml");
    let installed = compose_file.exists();

    // Query reverse proxy route for access info
    let (access_url, access_mode, internal_port) = match proxy_db.get_route_by_app_id(&id) {
        Ok(Some(route)) => {
            let port = route
                .target_url
                .rsplit(':')
                .next()
                .and_then(|p| p.parse::<u16>().ok());
            if let Some(ref d) = route.domain {
                (
                    Some(format!("https://{}", d)),
                    Some("subdomain".to_string()),
                    port,
                )
            } else if let Some(ref p) = route.path_prefix {
                (Some(p.clone()), Some("path".to_string()), port)
            } else {
                (None, None, port)
            }
        }
        _ => (None, None, None),
    };

    audit.log(
        &user.username,
        user.role.as_str(),
        "APP_CREDENTIALS",
        Some(&id),
        &user.client_ip,
        "SUCCESS",
        None,
    );

    let credentials = if creds_file.exists() {
        fs::read_to_string(&creds_file).ok()
    } else {
        None
    };

    HttpResponse::Ok().json(AppCredentialsResponse {
        id,
        credentials,
        installed,
        access_url,
        access_mode,
        internal_port,
    })
}

pub async fn uninstall_app(
    user: AuthenticatedUser,
    audit: web::Data<Arc<AuditLogger>>,
    proxy_db: web::Data<Arc<ProxyDatabase>>,
    path: web::Path<String>,
) -> impl Responder {
    let id = path.into_inner();

    if user.require_admin().is_err() {
        audit.log_denied(
            &user,
            "APP_UNINSTALL",
            Some(&id),
            Some("Requires Admin role"),
        );
        return HttpResponse::Forbidden().json(serde_json::json!({
            "error": "Insufficient permissions"
        }));
    }

    if !is_valid_app_id(&id) {
        return HttpResponse::BadRequest().json("Invalid app identifier");
    }

    // 1. Remove proxy route from reverse proxy database
    let _ = proxy_db.delete_route_by_app_id(&id);

    // 2. Tear down Docker containers and clean files
    let apps_dir = get_apps_dir();
    let app_dir = apps_dir.join(&id);

    if !app_dir.exists() {
        return HttpResponse::NotFound().json("App not found or not installed");
    }

    let dir_clone = app_dir.clone();
    let success = actix_web::web::block(move || {
        let mut cmd1 = Command::new("sudo");
        cmd1.args(["-n", "docker-compose", "down", "-v"])
            .current_dir(&dir_clone);
        if cmd1.output().map(|o| o.status.success()).unwrap_or(false) {
            return true;
        }

        let mut cmd2 = Command::new("sudo");
        cmd2.args(["-n", "docker", "compose", "down", "-v"])
            .current_dir(&dir_clone);
        cmd2.output().map(|o| o.status.success()).unwrap_or(false)
    })
    .await
    .unwrap_or(false);

    let _ = fs::remove_dir_all(&app_dir);

    audit.log(
        &user.username,
        user.role.as_str(),
        "APP_UNINSTALL",
        Some(&id),
        &user.client_ip,
        "SUCCESS",
        if success {
            Some("Containers stopped and filesystem purged")
        } else {
            Some("Removed from filesystem")
        },
    );

    HttpResponse::Ok().json(format!(
        "App {} uninstalled and reverse proxy route purged",
        id
    ))
}

pub async fn install_app(
    user: AuthenticatedUser,
    audit: web::Data<Arc<AuditLogger>>,
    registry: web::Data<DriverRegistry>,
    proxy_db: web::Data<Arc<ProxyDatabase>>,
    body: web::Json<AppInstallRequest>,
    job_manager: web::Data<Arc<crate::api::jobs::JobManager>>,
) -> impl Responder {
    if user.require_operator().is_err() {
        audit.log_denied(
            &user,
            "APP_INSTALL",
            Some(&body.id),
            Some("Requires Operator or Admin role"),
        );
        return HttpResponse::Forbidden().json(serde_json::json!({
            "error": "Insufficient permissions"
        }));
    }

    if !is_valid_app_id(&body.id) {
        return HttpResponse::BadRequest().json("Invalid app identifier");
    }

    let templates = get_app_templates();
    let template = match templates.into_iter().find(|t| t.id == body.id) {
        Some(t) => t,
        None => return HttpResponse::NotFound().json("App template not found"),
    };

    // ------------------------------------------------------------------------
    // STRICT SECURITY POLICY: Non-HTTP / External Ports Consent Verification
    // ------------------------------------------------------------------------
    if !template.ports.exposed_network_ports.is_empty() {
        let required_ports: Vec<u16> = template
            .ports
            .exposed_network_ports
            .iter()
            .map(|p| p.port)
            .collect();
        let unapproved = !body.approved_ports.is_empty()
            && required_ports
                .iter()
                .any(|p| !body.approved_ports.contains(p));

        if !body.allow_exposed_ports || unapproved {
            let port_summary: Vec<String> = template
                .ports
                .exposed_network_ports
                .iter()
                .map(|p| format!("{}/{} ({})", p.port, p.protocol, p.reason))
                .collect();

            audit.log(
                &user.username,
                user.role.as_str(),
                "APP_INSTALL_BLOCKED",
                Some(&template.id),
                &user.client_ip,
                "REJECTED",
                Some("Missing explicit user consent for external ports"),
            );

            return HttpResponse::BadRequest().json(serde_json::json!({
                "error": "External Port Authorization Required",
                "message": format!(
                    "Application '{}' requires opening external network port(s): {}. Explicit authorization is mandatory before proceeding.",
                    template.name,
                    port_summary.join(", ")
                ),
                "required_ports": template.ports.exposed_network_ports
            }));
        }

        // User explicitly consented to opening external firewall ports
        for p in &template.ports.exposed_network_ports {
            let rule = format!("allow {}/{}", p.port, p.protocol.to_lowercase());
            let rule_req = FirewallRuleRequest { rule };
            if let Err(e) = registry.firewall.add_rule(&rule_req).await {
                log::warn!("Failed to auto-add firewall rule for {}: {}", p.port, e);
            }
            audit.log(
                &user.username,
                user.role.as_str(),
                "APP_PORT_AUTHORIZED",
                Some(&template.id),
                &user.client_ip,
                "SUCCESS",
                Some(&format!(
                    "User authorized opening external port {}/{} for '{}' ({})",
                    p.port, p.protocol, template.name, p.reason
                )),
            );
        }
    }

    // ------------------------------------------------------------------------
    // ZERO PORT EXPOSURE: Localhost Loopback Port & Reverse Proxy Setup
    // ------------------------------------------------------------------------
    let mut assigned_internal_port = None;
    let mut access_url = None;

    if template.ports.internal_web_port.is_some() {
        let internal_port = match find_available_localhost_port() {
            Ok(p) => p,
            Err(e) => {
                log::error!("Failed to allocate available loopback port: {}", e);
                return HttpResponse::InternalServerError()
                    .json("Failed to allocate local port on 127.0.0.1");
            }
        };
        assigned_internal_port = Some(internal_port);

        let (domain, path_prefix, url_display) = if body.access_mode == "subdomain" {
            if let Some(ref raw_domain) = body.domain {
                let d = raw_domain.trim().to_lowercase();
                (Some(d.clone()), None, format!("https://{}", d))
            } else {
                let p = format!("/apps/{}", template.id);
                (None, Some(p.clone()), p)
            }
        } else {
            let p = format!("/apps/{}", template.id);
            (None, Some(p.clone()), p)
        };
        access_url = Some(url_display);

        let route = ProxyRoute {
            id: uuid::Uuid::new_v4().to_string(),
            app_id: template.id.clone(),
            domain,
            path_prefix,
            target_url: format!("http://127.0.0.1:{}", internal_port),
            websocket_support: true,
            created_at: chrono::Utc::now().to_rfc3339(),
        };

        if let Err(e) = proxy_db.create_or_update_route(&route) {
            log::error!("Failed to register reverse proxy route: {}", e);
            return HttpResponse::InternalServerError()
                .json("Failed to configure reverse proxy route");
        }
    }

    let wadm_dir = get_apps_dir();
    if !wadm_dir.exists() {
        let _ = fs::create_dir_all(&wadm_dir);
    }

    let app_dir = wadm_dir.join(&template.id);
    if !app_dir.exists() {
        let _ = fs::create_dir_all(&app_dir);
    }

    let secure_pwd = generate_secure_password();
    let mut customized_compose = template
        .compose_template
        .replace("{{SECURE_PASSWORD}}", &secure_pwd);

    if let Some(internal_port) = assigned_internal_port {
        customized_compose =
            customized_compose.replace("{{INTERNAL_PORT}}", &internal_port.to_string());
    }

    let compose_file = app_dir.join("docker-compose.yml");
    if let Err(e) = fs::write(&compose_file, &customized_compose) {
        return HttpResponse::InternalServerError()
            .json(format!("Failed to write compose file: {}", e));
    }

    // Save generated credentials
    let creds_file = app_dir.join("credentials.txt");
    let creds_content = format!(
        "Generated Password: {}\nGenerated At: {}\nAccess URL: {}\n",
        secure_pwd,
        chrono::Utc::now(),
        access_url.clone().unwrap_or_else(|| "N/A".to_string())
    );
    let _ = fs::write(&creds_file, creds_content);
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        let _ = fs::set_permissions(&creds_file, fs::Permissions::from_mode(0o600));
    }

    info!(
        "Enqueuing app deployment job for {} via JobManager...",
        template.name
    );

    let job_res = job_manager
        .enqueue_job(
            "app_install",
            crate::api::jobs::JobTaskPayload::AppInstall {
                app_id: template.id.clone(),
                app_name: template.name.clone(),
                app_dir: app_dir.clone(),
                _secure_pwd: secure_pwd.clone(),
            },
        )
        .await;

    match job_res {
        Ok(job_id) => {
            audit.log(
                &user.username,
                user.role.as_str(),
                "APP_INSTALL",
                Some(&template.id),
                &user.client_ip,
                "ACCEPTED",
                Some(&format!("Enqueued job {}", job_id)),
            );

            HttpResponse::Accepted().json(serde_json::json!({
                "job_id": job_id,
                "status": "accepted",
                "message": format!("Deployment of {} initiated via JobManager", template.name),
                "access_url": access_url
            }))
        }
        Err(e) => {
            log::error!("Failed to enqueue app install: {}", e);
            audit.log(
                &user.username,
                user.role.as_str(),
                "APP_INSTALL",
                Some(&template.id),
                &user.client_ip,
                "FAILED",
                Some(&e),
            );
            HttpResponse::InternalServerError().json("Failed to queue app installation")
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_get_templates_contains_all_apps() {
        let templates = get_app_templates();
        let ids: Vec<String> = templates.into_iter().map(|t| t.id).collect();
        assert!(ids.contains(&"nextcloud".to_string()));
        assert!(ids.contains(&"pihole".to_string()));
        assert!(ids.contains(&"wireguard".to_string()));
        assert!(ids.contains(&"vaultwarden".to_string()));
        assert!(ids.contains(&"jellyfin".to_string()));
        assert!(ids.contains(&"wordpress".to_string()));
        assert!(ids.contains(&"portainer".to_string()));
    }

    #[test]
    fn test_is_port_available_on_ephemeral_port() {
        let listener = TcpListener::bind("127.0.0.1:0").expect("Failed to bind ephemeral port");
        let port = listener.local_addr().unwrap().port();
        assert!(!is_port_available(port));
        drop(listener);
        assert!(is_port_available(port));
    }

    #[test]
    fn test_find_available_localhost_port() {
        let port = find_available_localhost_port().expect("Failed to get available port");
        assert!(port > 0);
        assert!(is_port_available(port));
    }

    #[test]
    fn test_is_valid_app_id() {
        assert!(is_valid_app_id("nextcloud"));
        assert!(is_valid_app_id("wireguard"));
        assert!(is_valid_app_id("vaultwarden"));
        assert!(!is_valid_app_id(""));
        assert!(!is_valid_app_id("../app"));
        assert!(!is_valid_app_id("/app"));
        assert!(!is_valid_app_id(".hidden"));
        assert!(!is_valid_app_id("-flag"));
        assert!(!is_valid_app_id("app/sub"));
        assert!(!is_valid_app_id("app\\sub"));
    }

    #[test]
    fn test_zero_external_ports_policy_for_web_apps() {
        let templates = get_app_templates();
        for t in &templates {
            if t.id == "nextcloud"
                || t.id == "vaultwarden"
                || t.id == "jellyfin"
                || t.id == "wordpress"
                || t.id == "portainer"
            {
                // Zero external ports
                assert!(
                    t.ports.exposed_network_ports.is_empty(),
                    "Web app {} must have 0 exposed ports",
                    t.id
                );
                assert!(
                    t.ports.internal_web_port.is_some(),
                    "Web app {} must have internal web port",
                    t.id
                );
            }
        }
    }

    #[test]
    fn test_wireguard_requires_external_udp_port() {
        let templates = get_app_templates();
        let wg = templates
            .into_iter()
            .find(|t| t.id == "wireguard")
            .expect("WireGuard template not found");
        assert_eq!(wg.ports.exposed_network_ports.len(), 1);
        assert_eq!(wg.ports.exposed_network_ports[0].port, 51820);
        assert_eq!(wg.ports.exposed_network_ports[0].protocol, "UDP");
        assert!(wg.ports.internal_web_port.is_none());
    }
}
