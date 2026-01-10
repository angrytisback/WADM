use actix_web::{web, HttpResponse, Responder};
use serde::{Deserialize, Serialize};
use std::process::Command;

#[derive(Serialize)]
struct Service {
    name: String,
    status: String, 
    description: String,
}

#[derive(Deserialize)]
pub struct ServiceControl {
    action: String, 
}

pub async fn list_services() -> impl Responder {
    use std::collections::HashSet;

    let mut services = Vec::new();
    let mut seen_names = HashSet::new();

    
    
    if let Ok(output) = Command::new("systemctl")
        .args(&[
            "list-units",
            "--type=service",
            "--all",
            "--no-pager",
            "--no-legend",
            "--plain",
            "--full",
        ])
        .output()
    {
        let stdout = String::from_utf8_lossy(&output.stdout);
        for line in stdout.lines().filter(|l| !l.is_empty()) {
            let parts: Vec<&str> = line.split_whitespace().collect();
            if parts.len() >= 4 {
                let name = parts[0].to_string();
                if name.starts_with("dbus-") || name.starts_with("user@") {
                    continue;
                }

                seen_names.insert(name.clone());

                let raw_status = parts[3].to_string();
                let nice_status = match raw_status.as_str() {
                    "running" => "Active (Running)",
                    "exited" => "Active (Exited)",
                    "dead" => "Stopped",
                    "failed" => "Failed",
                    _ => &raw_status,
                };

                services.push(Service {
                    name,
                    status: nice_status.to_string(),
                    description: parts[4..].join(" "),
                });
            }
        }
    }

    
    
    if let Ok(output) = Command::new("systemctl")
        .args(&[
            "list-unit-files",
            "--type=service",
            "--no-pager",
            "--no-legend",
            "--plain",
        ])
        .output()
    {
        let stdout = String::from_utf8_lossy(&output.stdout);
        for line in stdout.lines().filter(|l| !l.is_empty()) {
            let parts: Vec<&str> = line.split_whitespace().collect();
            if parts.len() >= 2 {
                let name = parts[0].to_string();

                
                if seen_names.contains(&name)
                    || name.starts_with("dbus-")
                    || name.starts_with("user@")
                {
                    continue;
                }

                
                
                services.push(Service {
                    name,
                    status: "Stopped".to_string(),
                    description: "Available (Not Loaded)".to_string(),
                });
            }
        }
    }

    
    services.sort_by(|a, b| a.name.cmp(&b.name));

    HttpResponse::Ok().json(services)
}

pub async fn control_service(
    path: web::Path<String>,
    body: web::Json<ServiceControl>,
) -> impl Responder {
    let service_name = path.into_inner();
    let action = &body.action;

    if !["start", "stop", "restart", "enable"].contains(&action.as_str()) {
        return HttpResponse::BadRequest().json("Invalid action");
    }

    
    
    let output = Command::new("systemctl")
        .arg(action)
        .arg(&service_name)
        .output();

    match output {
        Ok(out) if out.status.success() => {
            HttpResponse::Ok().json(format!("Service {} {}ed", service_name, action))
        }
        Ok(out) => HttpResponse::InternalServerError()
            .body(String::from_utf8_lossy(&out.stderr).to_string()),
        Err(_) => HttpResponse::InternalServerError().json("Failed to execute command"),
    }
}

pub async fn get_service_logs(path: web::Path<String>) -> impl Responder {
    let service_name = path.into_inner();

    
    let output = Command::new("journalctl")
        .arg("-u")
        .arg(&service_name)
        .arg("-n")
        .arg("100")
        .arg("--no-pager")
        .output();

    match output {
        Ok(output) => {
            let stdout = String::from_utf8_lossy(&output.stdout);
            HttpResponse::Ok().json(stdout.to_string())
        }
        Err(_) => HttpResponse::InternalServerError().json("Failed to fetch logs"),
    }
}
