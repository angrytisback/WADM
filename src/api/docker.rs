#![allow(deprecated)]
use actix_web::{web, HttpResponse, Responder};
use bollard::Docker;
use futures_util::stream::TryStreamExt;
use serde::{Deserialize, Serialize};

use std::process::Command;

#[derive(Serialize)]
struct ContainerInfo {
    id: String,
    name: String,
    image: String,
    status: String,
    state: String,
    
}

#[derive(Serialize)]
pub struct ContainerStats {
    pub id: String,
    pub cpu_usage: f64,    
    pub memory_usage: u64, 
}

#[derive(Serialize)]
pub struct DockerStatus {
    pub installed: bool,
    pub running: bool,
    pub version: String,
}

#[derive(Deserialize)]
pub struct ContainerAction {
    pub action: String, 
}

pub async fn list_containers() -> impl Responder {
    let docker = match Docker::connect_with_local_defaults() {
        Ok(d) => d,
        Err(_) => {
            return HttpResponse::InternalServerError().json("Failed to connect to Docker socket")
        }
    };

    let options = Some(bollard::container::ListContainersOptions::<String> {
        all: true,
        ..Default::default()
    });

    match docker.list_containers(options).await {
        Ok(containers) => {
            let info: Vec<ContainerInfo> = containers
                .into_iter()
                .map(|c| ContainerInfo {
                    id: c.id.unwrap_or_default(),
                    name: c
                        .names
                        .unwrap_or_default()
                        .get(0)
                        .cloned()
                        .unwrap_or_default()
                        .trim_start_matches('/')
                        .to_string(),
                    image: c.image.unwrap_or_default(),
                    status: c.status.unwrap_or_default(),
                    state: match c.state {
                        Some(s) => format!("{:?}", s).to_lowercase(),
                        None => "unknown".to_string(),
                    },
                })
                .collect();
            HttpResponse::Ok().json(info)
        }
        Err(e) => HttpResponse::InternalServerError().json(format!("Docker error: {}", e)),
    }
}



pub async fn get_container_stats(id: web::Path<String>) -> impl Responder {
    let docker = match Docker::connect_with_local_defaults() {
        Ok(d) => d,
        Err(_) => {
            return HttpResponse::InternalServerError().json("Failed to connect to Docker socket")
        }
    };

    let options = Some(bollard::container::StatsOptions {
        stream: false,
        one_shot: true,
    });

    match docker.stats(&id.into_inner(), options).try_next().await {
        Ok(Some(stats)) => {
            
            
            
            

            let cpu_stats = stats.cpu_stats;
            let precpu_stats = stats.precpu_stats;

            let cpu_usage_total = cpu_stats
                .as_ref()
                .and_then(|c| c.cpu_usage.as_ref())
                .and_then(|u| u.total_usage)
                .unwrap_or(0);

            let precpu_usage_total = precpu_stats
                .as_ref()
                .and_then(|c| c.cpu_usage.as_ref())
                .and_then(|u| u.total_usage)
                .unwrap_or(0);

            let system_cpu_usage = cpu_stats
                .as_ref()
                .and_then(|c| c.system_cpu_usage)
                .unwrap_or(0);

            let presystem_cpu_usage = precpu_stats
                .as_ref()
                .and_then(|c| c.system_cpu_usage)
                .unwrap_or(0);

            let online_cpus = cpu_stats.as_ref().and_then(|c| c.online_cpus).unwrap_or(1);

            let cpu_delta = cpu_usage_total as f64 - precpu_usage_total as f64;
            let system_delta = system_cpu_usage as f64 - presystem_cpu_usage as f64;

            let mut cpu_percent = 0.0;
            if system_delta > 0.0 && cpu_delta > 0.0 {
                cpu_percent = (cpu_delta / system_delta) * (online_cpus as f64) * 100.0;
            }

            
            let memory_usage = stats
                .memory_stats
                .as_ref()
                .and_then(|m| m.usage)
                .unwrap_or(0);

            HttpResponse::Ok().json(ContainerStats {
                id: "".to_string(),
                cpu_usage: cpu_percent,
                memory_usage,
            })
        }
        Ok(None) => HttpResponse::NotFound().json("No stats found"),
        Err(e) => HttpResponse::InternalServerError().json(format!("Failed to fetch stats: {}", e)),
    }
}

pub async fn control_container(
    id: web::Path<String>,
    body: web::Json<ContainerAction>,
) -> impl Responder {
    let container_id = id.into_inner();
    let action = &body.action;

    let docker = match Docker::connect_with_local_defaults() {
        Ok(d) => d,
        Err(_) => {
            return HttpResponse::InternalServerError().json("Failed to connect to Docker socket")
        }
    };

    let result = match action.as_str() {
        "start" => {
            docker
                .start_container(
                    &container_id,
                    None::<bollard::container::StartContainerOptions<String>>,
                )
                .await
        }
        "stop" => {
            docker
                .stop_container(
                    &container_id,
                    None::<bollard::container::StopContainerOptions>,
                )
                .await
        }
        "restart" => {
            docker
                .restart_container(
                    &container_id,
                    None::<bollard::container::RestartContainerOptions>,
                )
                .await
        }
        _ => return HttpResponse::BadRequest().json("Invalid action"),
    };

    match result {
        Ok(_) => HttpResponse::Ok().json(format!("Container {} {}ed", container_id, action)),
        Err(e) => HttpResponse::InternalServerError().json(format!("Docker action failed: {}", e)),
    }
}

pub async fn get_status() -> impl Responder {
    let installed = Command::new("which")
        .arg("docker")
        .output()
        .map(|o| o.status.success())
        .unwrap_or(false);

    let mut running = false;
    if installed {
        
        if let Ok(output) = Command::new("systemctl")
            .arg("is-active")
            .arg("docker")
            .output()
        {
            let s = String::from_utf8_lossy(&output.stdout);
            if s.trim() == "active" {
                running = true;
            }
        }
    }

    let mut version = String::new();
    if installed {
        if let Ok(output) = Command::new("docker").arg("--version").output() {
            version = String::from_utf8_lossy(&output.stdout).trim().to_string();
        }
    }

    HttpResponse::Ok().json(DockerStatus {
        installed,
        running,
        version,
    })
}

pub async fn start_service() -> impl Responder {
    let status = Command::new("sudo") 
        .arg("systemctl")
        .arg("start")
        .arg("docker")
        .status();

    match status {
        Ok(s) => {
            if s.success() {
                HttpResponse::Ok().json("Docker service started")
            } else {
                HttpResponse::InternalServerError().json("Failed to start docker service")
            }
        }
        Err(e) => HttpResponse::InternalServerError().json(format!("Error: {}", e)),
    }
}
