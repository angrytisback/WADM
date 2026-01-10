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
        .arg("ufw")
        .arg("status")
        .arg("numbered")
        .output();

    match output {
        Ok(o) => {
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
        Err(_) => HttpResponse::InternalServerError().json("Failed to execute ufw command"),
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

    let output = Command::new("sudo").arg(cmd).args(args).output();

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

pub async fn set_status(body: web::Json<FirewallAction>) -> impl Responder {
    let arg = if body.action == "enable" {
        "enable"
    } else {
        "disable"
    };

    
    let status = if arg == "enable" {
        Command::new("sh")
            .arg("-c")
            .arg("yes | sudo ufw enable")
            .status()
    } else {
        Command::new("sudo").arg("ufw").arg("disable").status()
    };

    match status {
        Ok(s) => {
            if s.success() {
                HttpResponse::Ok().json(format!("Firewall {}d", arg))
            } else {
                HttpResponse::InternalServerError().json("Command failed")
            }
        }
        Err(e) => HttpResponse::InternalServerError().json(format!("Error: {}", e)),
    }
}

pub async fn add_rule(body: web::Json<FirewallRuleData>) -> impl Responder {
    
    
    

    
    let args: Vec<&str> = body.rule.split_whitespace().collect();

    if args.is_empty() {
        return HttpResponse::BadRequest().json("Rule cannot be empty");
    }

    let status = Command::new("sudo").arg("ufw").args(args).status();

    match status {
        Ok(s) => {
            if s.success() {
                HttpResponse::Ok().json("Rule added")
            } else {
                HttpResponse::InternalServerError().json("Failed to add rule")
            }
        }
        Err(e) => HttpResponse::InternalServerError().json(format!("Error: {}", e)),
    }
}

pub async fn delete_rule(body: web::Json<FirewallRuleData>) -> impl Responder {
    
    
    

    let args: Vec<&str> = body.rule.split_whitespace().collect();
    if args.is_empty() {
        return HttpResponse::BadRequest().json("Rule cannot be empty");
    }

    let mut command = Command::new("sudo");
    command.arg("ufw").arg("delete");
    command.args(args);

    
    
    let status = Command::new("sh")
        .arg("-c")
        .arg(format!("yes | sudo ufw delete {}", body.rule))
        .status();

    match status {
        Ok(s) => {
            if s.success() {
                HttpResponse::Ok().json("Rule deleted")
            } else {
                HttpResponse::InternalServerError().json("Failed to delete rule")
            }
        }
        Err(e) => HttpResponse::InternalServerError().json(format!("Error: {}", e)),
    }
}
