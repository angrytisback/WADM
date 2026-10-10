use actix_web::{web, HttpResponse, Responder};
use serde::Deserialize;
use tokio::process::Command;

pub use crate::drivers::firewall::FirewallRuleRequest as FirewallRuleData;
#[allow(unused_imports)]
pub use crate::drivers::firewall::{is_safe_ufw_rule, FirewallStatus};
use crate::drivers::DriverRegistry;

#[derive(Deserialize)]
pub struct FirewallAction {
    pub action: String,
}

pub async fn get_status(registry: web::Data<DriverRegistry>) -> impl Responder {
    match registry.firewall.get_status().await {
        Ok(status) => HttpResponse::Ok().json(status),
        Err(e) => {
            log::error!("Failed to get firewall status: {}", e);
            HttpResponse::InternalServerError().json(format!("Firewall check error: {}", e))
        }
    }
}

pub async fn install_ufw(
    user: crate::auth::AuthenticatedUser,
    audit: web::Data<std::sync::Arc<crate::audit::AuditLogger>>,
    registry: web::Data<DriverRegistry>,
) -> impl Responder {
    if user.require_admin().is_err() {
        audit.log_denied(
            &user,
            "FIREWALL_INSTALL",
            Some("ufw"),
            Some("Requires Admin role"),
        );
        return HttpResponse::Forbidden().json(serde_json::json!({
            "error": "Insufficient permissions"
        }));
    }

    let (cmd, args) = registry.package_manager.build_install_command("ufw");
    let output = Command::new(&cmd).args(&args).output().await;

    match output {
        Ok(o) => {
            if o.status.success() {
                audit.log(
                    &user.username,
                    user.role.as_str(),
                    "FIREWALL_INSTALL",
                    Some("ufw"),
                    &user.client_ip,
                    "SUCCESS",
                    None,
                );
                HttpResponse::Ok().json("UFW installed successfully")
            } else {
                let stderr = String::from_utf8_lossy(&o.stderr);
                audit.log(
                    &user.username,
                    user.role.as_str(),
                    "FIREWALL_INSTALL",
                    Some("ufw"),
                    &user.client_ip,
                    "FAILED",
                    Some(&stderr),
                );
                HttpResponse::InternalServerError()
                    .json(format!("Failed to install UFW: {}", stderr))
            }
        }
        Err(e) => {
            audit.log(
                &user.username,
                user.role.as_str(),
                "FIREWALL_INSTALL",
                Some("ufw"),
                &user.client_ip,
                "FAILED",
                Some(&e.to_string()),
            );
            HttpResponse::InternalServerError()
                .json(format!("Failed to execute install command: {}", e))
        }
    }
}

pub async fn set_status(
    user: crate::auth::AuthenticatedUser,
    audit: web::Data<std::sync::Arc<crate::audit::AuditLogger>>,
    body: web::Json<FirewallAction>,
    registry: web::Data<DriverRegistry>,
) -> impl Responder {
    let enable = body.action == "enable";
    let action_str = if enable { "enable" } else { "disable" };

    if user.require_admin().is_err() {
        audit.log_denied(
            &user,
            "FIREWALL_STATUS_CHANGE",
            Some(action_str),
            Some("Requires Admin role"),
        );
        return HttpResponse::Forbidden().json(serde_json::json!({
            "error": "Insufficient permissions"
        }));
    }

    match registry.firewall.set_status(enable).await {
        Ok(_) => {
            audit.log(
                &user.username,
                user.role.as_str(),
                "FIREWALL_STATUS_CHANGE",
                Some(action_str),
                &user.client_ip,
                "SUCCESS",
                None,
            );
            HttpResponse::Ok().json(format!("Firewall {}d", action_str))
        }
        Err(e) => {
            log::error!("Failed to toggle firewall status: {}", e);
            audit.log(
                &user.username,
                user.role.as_str(),
                "FIREWALL_STATUS_CHANGE",
                Some(action_str),
                &user.client_ip,
                "FAILED",
                Some(&e.to_string()),
            );
            HttpResponse::InternalServerError().json("Error executing firewall command")
        }
    }
}

pub async fn add_rule(
    user: crate::auth::AuthenticatedUser,
    audit: web::Data<std::sync::Arc<crate::audit::AuditLogger>>,
    body: web::Json<FirewallRuleData>,
    registry: web::Data<DriverRegistry>,
) -> impl Responder {
    let rule_data = body.into_inner();
    let rule_desc = &rule_data.rule;

    if user.require_admin().is_err() {
        audit.log_denied(
            &user,
            "FIREWALL_RULE_ADD",
            Some(rule_desc),
            Some("Requires Admin role"),
        );
        return HttpResponse::Forbidden().json(serde_json::json!({
            "error": "Insufficient permissions"
        }));
    }

    match registry.firewall.add_rule(&rule_data).await {
        Ok(_) => {
            audit.log(
                &user.username,
                user.role.as_str(),
                "FIREWALL_RULE_ADD",
                Some(rule_desc),
                &user.client_ip,
                "SUCCESS",
                None,
            );
            HttpResponse::Ok().json("Rule added")
        }
        Err(crate::drivers::AppError::InvalidInput(msg)) => {
            audit.log(
                &user.username,
                user.role.as_str(),
                "FIREWALL_RULE_ADD",
                Some(rule_desc),
                &user.client_ip,
                "FAILED",
                Some(&msg),
            );
            HttpResponse::BadRequest().json(msg)
        }
        Err(e) => {
            log::error!("Failed to add firewall rule: {}", e);
            audit.log(
                &user.username,
                user.role.as_str(),
                "FIREWALL_RULE_ADD",
                Some(rule_desc),
                &user.client_ip,
                "FAILED",
                Some(&e.to_string()),
            );
            HttpResponse::InternalServerError().json("Failed to add rule")
        }
    }
}

pub async fn delete_rule(
    user: crate::auth::AuthenticatedUser,
    audit: web::Data<std::sync::Arc<crate::audit::AuditLogger>>,
    body: web::Json<FirewallRuleData>,
    registry: web::Data<DriverRegistry>,
) -> impl Responder {
    let rule_desc = &body.rule;

    if user.require_admin().is_err() {
        audit.log_denied(
            &user,
            "FIREWALL_RULE_DELETE",
            Some(rule_desc),
            Some("Requires Admin role"),
        );
        return HttpResponse::Forbidden().json(serde_json::json!({
            "error": "Insufficient permissions"
        }));
    }

    match registry.firewall.delete_rule(rule_desc).await {
        Ok(_) => {
            audit.log(
                &user.username,
                user.role.as_str(),
                "FIREWALL_RULE_DELETE",
                Some(rule_desc),
                &user.client_ip,
                "SUCCESS",
                None,
            );
            HttpResponse::Ok().json("Rule deleted")
        }
        Err(crate::drivers::AppError::InvalidInput(msg)) => {
            audit.log(
                &user.username,
                user.role.as_str(),
                "FIREWALL_RULE_DELETE",
                Some(rule_desc),
                &user.client_ip,
                "FAILED",
                Some(&msg),
            );
            HttpResponse::BadRequest().json(msg)
        }
        Err(e) => {
            log::error!("Failed to delete firewall rule: {}", e);
            audit.log(
                &user.username,
                user.role.as_str(),
                "FIREWALL_RULE_DELETE",
                Some(rule_desc),
                &user.client_ip,
                "FAILED",
                Some(&e.to_string()),
            );
            HttpResponse::InternalServerError().json("Failed to delete rule")
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_valid_ufw_rules() {
        assert!(is_safe_ufw_rule("allow 22/tcp"));
        assert!(is_safe_ufw_rule("deny from 192.168.1.50"));
        assert!(is_safe_ufw_rule("limit 80/tcp"));
        assert!(is_safe_ufw_rule("delete 1"));
        assert!(is_safe_ufw_rule("1"));
    }

    #[test]
    fn test_invalid_ufw_rules_injection() {
        assert!(!is_safe_ufw_rule(""));
        assert!(!is_safe_ufw_rule("allow 22; rm -rf /"));
        assert!(!is_safe_ufw_rule("allow 22 && reboot"));
        assert!(!is_safe_ufw_rule("status --force"));
        assert!(!is_safe_ufw_rule("reset"));
        assert!(!is_safe_ufw_rule("allow `id`"));
    }
}
