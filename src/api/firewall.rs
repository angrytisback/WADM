use actix_web::{web, HttpResponse, Responder};
use serde::{Deserialize, Serialize};
use std::process::Command;

#[derive(Serialize, Deserialize)]
pub struct FirewallStatus {
    pub active: bool,
    pub rules: Vec<String>,
    pub installed: bool,
}

#[derive(Deserialize)]
pub struct FirewallAction {
    pub action: String,
}

#[derive(Deserialize)]
pub struct FirewallRuleData {
    pub rule: String,
}

pub async fn get_status() -> impl Responder {
    let check = Command::new("which").arg("ufw").output();
    let installed = match check {
        Ok(o) => o.status.success(),
        Err(_) => false,
    };

    if !installed {
        return HttpResponse::Ok().json(FirewallStatus {
            active: false,
            rules: vec![],
            installed: false,
        });
    }

    let output = Command::new("sudo")
        .args(["-n", "ufw", "status", "numbered"])
        .output();

    match output {
        Ok(o) => {
            if !o.status.success() {
                let stderr = String::from_utf8_lossy(&o.stderr).trim().to_string();
                let err_msg = if stderr.is_empty() {
                    let stdout = String::from_utf8_lossy(&o.stdout).trim().to_string();
                    if stdout.is_empty() {
                        format!("ufw command failed with exit code {:?}", o.status.code())
                    } else {
                        stdout
                    }
                } else {
                    stderr
                };
                return HttpResponse::InternalServerError()
                    .json(format!("Firewall check error: {}", err_msg));
            }

            let stdout = String::from_utf8_lossy(&o.stdout);
            let active = stdout.contains("Status: active");

            let rules: Vec<String> = stdout
                .lines()
                .filter(|line| line.contains("["))
                .map(|line| line.to_string())
                .collect();

            HttpResponse::Ok().json(FirewallStatus {
                active,
                rules,
                installed: true,
            })
        }
        Err(e) => HttpResponse::InternalServerError()
            .json(format!("Failed to execute ufw command: {}", e)),
    }
}

pub async fn install_ufw() -> impl Responder {
    use crate::api::pkgmgr::{detect_manager, ManagerType};

    let manager = detect_manager();
    let (cmd, args) = match manager {
        ManagerType::Apt => ("apt-get", vec!["install", "-y", "ufw"]),
        ManagerType::Dnf => ("dnf", vec!["install", "-y", "ufw"]),
        ManagerType::Pacman => ("pacman", vec!["-S", "--noconfirm", "ufw"]),
        ManagerType::Unknown => {
            return HttpResponse::InternalServerError().json("Unsupported package manager")
        }
    };

    let output = Command::new("sudo").arg("-n").arg(cmd).args(args).output();

    match output {
        Ok(o) => {
            if o.status.success() {
                HttpResponse::Ok().json("UFW installed successfully")
            } else {
                let stderr = String::from_utf8_lossy(&o.stderr);
                HttpResponse::InternalServerError()
                    .json(format!("Failed to install UFW: {}", stderr))
            }
        }
        Err(e) => HttpResponse::InternalServerError()
            .json(format!("Failed to execute install command: {}", e)),
    }
}

fn is_safe_ufw_rule(rule: &str) -> bool {
    let r = rule.trim();
    if r.is_empty() || r.len() > 120 {
        return false;
    }
    // Only allow alphanumeric, spaces, dots, slashes, colons and hyphens
    if !r.chars().all(|c| {
        c.is_ascii_alphanumeric() || c == ' ' || c == '.' || c == '/' || c == ':' || c == '-'
    }) {
        return false;
    }
    let parts: Vec<&str> = r.split_whitespace().collect();
    if parts.is_empty() {
        return false;
    }
    let first = parts[0].to_lowercase();
    let valid_actions = [
        "allow", "deny", "reject", "limit", "delete", "insert", "route",
    ];
    let is_number = first.chars().all(|c| c.is_ascii_digit());
    valid_actions.contains(&first.as_str()) || is_number
}

pub async fn set_status(body: web::Json<FirewallAction>) -> impl Responder {
    let arg = if body.action == "enable" {
        "enable"
    } else {
        "disable"
    };

    let status = if arg == "enable" {
        Command::new("sudo")
            .args(["-n", "ufw", "--force", "enable"])
            .status()
    } else {
        Command::new("sudo").args(["-n", "ufw", "disable"]).status()
    };

    match status {
        Ok(s) => {
            if s.success() {
                HttpResponse::Ok().json(format!("Firewall {}d", arg))
            } else {
                HttpResponse::InternalServerError().json("Command failed")
            }
        }
        Err(e) => {
            log::error!("Failed to toggle firewall status: {}", e);
            HttpResponse::InternalServerError().json("Error executing firewall command")
        }
    }
}

pub async fn add_rule(body: web::Json<FirewallRuleData>) -> impl Responder {
    let rule = body.rule.trim();
    if !is_safe_ufw_rule(rule) {
        return HttpResponse::BadRequest().json("Invalid firewall rule format");
    }

    let rule_parts: Vec<&str> = rule.split_whitespace().collect();
    let mut args = vec!["-n", "ufw"];
    args.extend(rule_parts);

    let status = Command::new("sudo").args(&args).status();

    match status {
        Ok(s) => {
            if s.success() {
                HttpResponse::Ok().json("Rule added")
            } else {
                HttpResponse::InternalServerError().json("Failed to add rule")
            }
        }
        Err(e) => {
            log::error!("Failed to add firewall rule: {}", e);
            HttpResponse::InternalServerError().json("Error executing firewall command")
        }
    }
}

pub async fn delete_rule(body: web::Json<FirewallRuleData>) -> impl Responder {
    let rule = body.rule.trim();
    if !is_safe_ufw_rule(rule) {
        return HttpResponse::BadRequest().json("Invalid firewall rule format");
    }

    let mut args = vec!["-n", "ufw", "--force", "delete"];
    let rule_parts: Vec<&str> = rule.split_whitespace().collect();
    let effective_parts = if rule_parts.first().map(|s| *s == "delete").unwrap_or(false) {
        &rule_parts[1..]
    } else {
        &rule_parts[..]
    };
    args.extend(effective_parts);

    let status = Command::new("sudo").args(&args).status();

    match status {
        Ok(s) => {
            if s.success() {
                HttpResponse::Ok().json("Rule deleted")
            } else {
                HttpResponse::InternalServerError().json("Failed to delete rule")
            }
        }
        Err(e) => {
            log::error!("Failed to delete firewall rule: {}", e);
            HttpResponse::InternalServerError().json("Error executing firewall command")
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
