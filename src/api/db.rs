use actix_web::{HttpResponse, Responder};
use serde::Serialize;
use std::process::Command;

#[derive(Serialize)]
struct Database {
    name: String,
    engine: String, 
    size: String,
}

pub async fn list_dbs() -> impl Responder {
    let mut dbs = Vec::new();

    
    
    
    if let Ok(output) = Command::new("mysql")
        .args(&["-e", "SHOW DATABASES"])
        .output()
    {
        if output.status.success() {
            let stdout = String::from_utf8_lossy(&output.stdout);
            for line in stdout.lines().skip(1) {
                
                if !line.trim().is_empty() {
                    dbs.push(Database {
                        name: line.trim().to_string(),
                        engine: "mysql".to_string(),
                        size: "-".to_string(), 
                    });
                }
            }
        }
    }

    
    
    if let Ok(output) = Command::new("sudo")
        .args(&["-u", "postgres", "psql", "-l", "-t", "-A", "-F", "|"])
        .output()
    {
        if output.status.success() {
            let stdout = String::from_utf8_lossy(&output.stdout);
            for line in stdout.lines() {
                let parts: Vec<&str> = line.split('|').collect();
                if parts.len() >= 1 {
                    dbs.push(Database {
                        name: parts[0].to_string(),
                        engine: "postgres".to_string(),
                        size: "-".to_string(),
                    });
                }
            }
        }
    }

    HttpResponse::Ok().json(dbs)
}
