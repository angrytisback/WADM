use futures_util::{SinkExt, StreamExt};
use std::process::Command;
use std::sync::Arc;
use std::time::Duration;
use sysinfo::{Networks, System};
use tokio::sync::mpsc;
use tokio_tungstenite::connect_async;
use tokio_tungstenite::tungstenite::Message;

use super::protocol::{ClusterMessage, NodeSpecs};
use crate::api::monitor::{collect_metrics_snapshot, AppState};

pub async fn run_agent(hub_url: String, mut token: String, node_name: Option<String>) {
    log::info!("Starting WADM Agent in headless mode...");
    log::info!("Target Hub URL: {}", hub_url);

    let mut sys = System::new_all();
    sys.refresh_all();
    let networks = Networks::new_with_refreshed_list();
    let (metrics_sender, _) =
        tokio::sync::broadcast::channel::<crate::api::monitor::SystemStats>(32);

    let app_state = Arc::new(AppState {
        sys: std::sync::Mutex::new(sys),
        networks: std::sync::Mutex::new(networks),
        metrics_sender,
    });

    let db_path = crate::api::jobs::get_wadm_db_path();
    let job_db = Arc::new(
        crate::api::jobs::JobDatabase::new(db_path)
            .expect("Failed to initialize Agent Job database"),
    );
    let job_manager = crate::api::jobs::JobManager::new(job_db);

    let hostname = System::host_name().unwrap_or_else(|| "unknown-host".to_string());
    let name = node_name.unwrap_or_else(|| hostname.clone());
    let version = env!("CARGO_PKG_VERSION").to_string();

    let mut node_id: Option<String> = None;
    let mut backoff = 1u64;

    loop {
        let specs = collect_local_specs(&app_state);

        let ws_url = if !hub_url.contains("token=") && !token.is_empty() {
            let sep = if hub_url.contains('?') { '&' } else { '?' };
            format!("{}{}{}token={}", hub_url, sep, "wadm_", token)
        } else {
            hub_url.clone()
        };

        log::info!("Connecting outbound tunnel to Hub: {} ...", ws_url);

        match connect_async(&ws_url).await {
            Ok((ws_stream, _response)) => {
                log::info!("WebSocket tunnel connection successfully established with Hub!");
                backoff = 1;

                let (mut ws_write, mut ws_read) = ws_stream.split();
                let (out_tx, mut out_rx) = mpsc::channel::<ClusterMessage>(64);

                // Send initial Handshake
                let handshake = ClusterMessage::Handshake {
                    node_id: node_id.clone(),
                    name: name.clone(),
                    hostname: hostname.clone(),
                    version: version.clone(),
                    specs: specs.clone(),
                    token: token.clone(),
                };

                let _ = out_tx.send(handshake).await;

                // Spawn task to pump out_rx into ws_write
                let out_pump = tokio::spawn(async move {
                    while let Some(msg) = out_rx.recv().await {
                        if let Ok(json) = serde_json::to_string(&msg) {
                            if ws_write.send(Message::Text(json.into())).await.is_err() {
                                break;
                            }
                        }
                    }
                });

                // Spawn 5s ping sender task
                let ping_tx = out_tx.clone();
                let app_state_ping = app_state.clone();
                let ver_clone = version.clone();
                let ping_task = tokio::spawn(async move {
                    let mut interval = tokio::time::interval(Duration::from_secs(5));
                    loop {
                        interval.tick().await;
                        let cur_specs = collect_local_specs(&app_state_ping);
                        let ping_msg = ClusterMessage::Ping {
                            specs: Some(cur_specs),
                            version: Some(ver_clone.clone()),
                        };
                        if ping_tx.send(ping_msg).await.is_err() {
                            break;
                        }
                    }
                });

                // Main receive loop
                while let Some(msg_res) = ws_read.next().await {
                    match msg_res {
                        Ok(Message::Text(text)) => {
                            if let Ok(cluster_msg) = serde_json::from_str::<ClusterMessage>(&text) {
                                match cluster_msg {
                                    ClusterMessage::HandshakeAck {
                                        node_id: assigned_id,
                                        auth_token,
                                        message,
                                    } => {
                                        log::info!(
                                            "Hub acknowledged handshake: {} (Node ID: {})",
                                            message,
                                            assigned_id
                                        );
                                        node_id = Some(assigned_id);
                                        if !auth_token.is_empty() {
                                            token = auth_token;
                                        }
                                    }
                                    ClusterMessage::Pong { timestamp } => {
                                        log::trace!(
                                            "Received Pong from Hub, timestamp: {}",
                                            timestamp
                                        );
                                    }
                                    ClusterMessage::RpcRequest { id, method, params } => {
                                        let out_tx_reply = out_tx.clone();
                                        let app_state_rpc = app_state.clone();
                                        let job_mgr_rpc = job_manager.clone();

                                        tokio::spawn(async move {
                                            let (res, err) = handle_rpc(
                                                &method,
                                                params,
                                                &app_state_rpc,
                                                &job_mgr_rpc,
                                            )
                                            .await;

                                            let reply = ClusterMessage::RpcResponse {
                                                id,
                                                result: res,
                                                error: err,
                                            };
                                            let _ = out_tx_reply.send(reply).await;
                                        });
                                    }
                                    ClusterMessage::Disconnect { reason } => {
                                        log::warn!("Hub sent disconnect message: {}", reason);
                                        break;
                                    }
                                    _ => {}
                                }
                            }
                        }
                        Ok(Message::Ping(bytes)) => {
                            let _ = out_tx.send(ClusterMessage::Pong { timestamp: 0 }).await;
                            log::trace!("Received WS Ping ({} bytes)", bytes.len());
                        }
                        Ok(Message::Close(frame)) => {
                            log::warn!("WebSocket closed by Hub: {:?}", frame);
                            break;
                        }
                        Err(e) => {
                            log::error!("Error reading from WebSocket: {}", e);
                            break;
                        }
                        _ => {}
                    }
                }

                ping_task.abort();
                out_pump.abort();
                log::warn!("Tunnel connection lost. Reconnecting...");
            }
            Err(e) => {
                log::warn!(
                    "Failed to connect to Hub: {}. Retrying in {} seconds...",
                    e,
                    backoff
                );
            }
        }

        tokio::time::sleep(Duration::from_secs(backoff)).await;
        backoff = std::cmp::min(backoff * 2, 30);
    }
}

fn collect_local_specs(app_state: &Arc<AppState>) -> NodeSpecs {
    let mut sys = app_state.sys.lock().unwrap_or_else(|e| e.into_inner());
    sys.refresh_cpu_usage();
    sys.refresh_memory();

    NodeSpecs {
        cpu_cores: sys.cpus().len(),
        total_memory: sys.total_memory(),
        os_name: System::name().unwrap_or_else(|| "Linux".to_string()),
        os_version: System::os_version().unwrap_or_else(|| "Unknown".to_string()),
        kernel_version: System::kernel_version().unwrap_or_else(|| "Unknown".to_string()),
        architecture: std::env::consts::ARCH.to_string(),
    }
}

async fn handle_rpc(
    method: &str,
    params: serde_json::Value,
    app_state: &Arc<AppState>,
    job_manager: &Arc<crate::api::jobs::JobManager>,
) -> (Option<serde_json::Value>, Option<String>) {
    match method {
        "get_stats" => {
            let stats = collect_metrics_snapshot(&app_state.sys, &app_state.networks);
            (Some(serde_json::to_value(stats).unwrap_or_default()), None)
        }
        "get_system_info" => {
            let (cpu_count, total_memory, used_memory, total_swap, used_swap) = {
                let mut sys = app_state.sys.lock().unwrap_or_else(|e| e.into_inner());
                sys.refresh_memory();
                (
                    sys.cpus().len(),
                    sys.total_memory(),
                    sys.used_memory(),
                    sys.total_swap(),
                    sys.used_swap(),
                )
            };

            let os_name = System::name().unwrap_or_else(|| "Linux".to_string());
            let os_version = System::os_version().unwrap_or_else(|| "Unknown".to_string());
            let kernel_version = System::kernel_version().unwrap_or_else(|| "Unknown".to_string());
            let host_name = System::host_name().unwrap_or_else(|| "localhost".to_string());
            let uptime = System::uptime();
            let cpu_arch = std::env::consts::ARCH.to_string();

            let (username, has_sudo, is_root) = {
                let u = std::env::var("USER").unwrap_or_else(|_| "wadm".to_string());
                let root = u == "root";
                (u, root, root)
            };

            let info = serde_json::json!({
                "os_name": os_name,
                "os_version": os_version,
                "kernel_version": kernel_version,
                "host_name": host_name,
                "uptime": uptime,
                "cpu_arch": cpu_arch,
                "cpu_count": cpu_count,
                "total_memory": total_memory,
                "used_memory": used_memory,
                "total_swap": total_swap,
                "used_swap": used_swap,
                "username": username,
                "has_sudo": has_sudo,
                "is_root": is_root,
                "smart": null,
                "cpu_temp": null,
                "gpu_temp": null,
                "gpus": []
            });

            (Some(info), None)
        }
        "list_services" => {
            if let Ok(output) = Command::new("sudo")
                .args([
                    "-n",
                    "systemctl",
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
                let mut services = Vec::new();

                for line in stdout.lines().filter(|l| !l.is_empty()) {
                    let parts: Vec<&str> = line.split_whitespace().collect();
                    if parts.len() >= 4 {
                        let name = parts[0].to_string();
                        if name.starts_with("dbus-") || name.starts_with("user@") {
                            continue;
                        }
                        let raw_status = parts[3].to_string();
                        let nice_status = match raw_status.as_str() {
                            "running" => "Active (Running)",
                            "exited" => "Active (Exited)",
                            "dead" => "Stopped",
                            "failed" => "Failed",
                            _ => &raw_status,
                        };
                        services.push(serde_json::json!({
                            "name": name,
                            "status": nice_status,
                            "description": parts[4..].join(" ")
                        }));
                    }
                }
                (Some(serde_json::Value::Array(services)), None)
            } else {
                (Some(serde_json::json!([])), None)
            }
        }
        "service_action" => {
            let service = params
                .get("service")
                .or_else(|| params.get("name"))
                .and_then(|v| v.as_str())
                .unwrap_or_default();
            let action = params
                .get("action")
                .and_then(|v| v.as_str())
                .unwrap_or_default();

            if service.is_empty() || action.is_empty() {
                return (None, Some("Missing service name or action".to_string()));
            }

            match Command::new("sudo")
                .args(["-n", "systemctl", action, service])
                .output()
            {
                Ok(output) => {
                    if output.status.success() {
                        (
                            Some(serde_json::json!({
                                "success": true,
                                "message": format!("Service {} {} action successful", service, action)
                            })),
                            None,
                        )
                    } else {
                        let err = String::from_utf8_lossy(&output.stderr).to_string();
                        (None, Some(err))
                    }
                }
                Err(e) => (None, Some(format!("Failed to execute systemctl: {}", e))),
            }
        }
        "execute_job" => {
            let cmd = params
                .get("cmd")
                .and_then(|v| v.as_str())
                .unwrap_or("uptime");
            let desc = params
                .get("description")
                .and_then(|v| v.as_str())
                .unwrap_or("Remote cluster command");

            let payload = crate::api::jobs::JobTaskPayload::CommandExecution {
                cmd: cmd.to_string(),
                args: vec![],
                output_file: None,
                description: desc.to_string(),
            };

            match job_manager.enqueue_job("command", payload).await {
                Ok(job_id) => (
                    Some(serde_json::json!({
                        "job_id": job_id,
                        "status": "pending",
                        "message": "Job enqueued on node"
                    })),
                    None,
                ),
                Err(e) => (None, Some(e)),
            }
        }
        "list_jobs" => match job_manager.list_jobs(50) {
            Ok(jobs) => (Some(serde_json::to_value(jobs).unwrap_or_default()), None),
            Err(e) => (None, Some(format!("Failed to query jobs: {}", e))),
        },
        "get_job" => {
            let id = params
                .get("id")
                .and_then(|v| v.as_str())
                .unwrap_or_default();
            match job_manager.get_job(id) {
                Ok(Some(job)) => (Some(serde_json::to_value(job).unwrap_or_default()), None),
                Ok(None) => (None, Some("Job not found".to_string())),
                Err(e) => (None, Some(format!("Database error: {}", e))),
            }
        }
        "execute_command" => {
            let cmd = params
                .get("cmd")
                .and_then(|v| v.as_str())
                .unwrap_or_default();
            if cmd.is_empty() {
                return (None, Some("Empty command".to_string()));
            }

            match tokio::process::Command::new("bash")
                .arg("-c")
                .arg(cmd)
                .output()
                .await
            {
                Ok(output) => {
                    let stdout = String::from_utf8_lossy(&output.stdout).to_string();
                    let stderr = String::from_utf8_lossy(&output.stderr).to_string();
                    (
                        Some(serde_json::json!({
                            "stdout": stdout,
                            "stderr": stderr,
                            "exit_code": output.status.code().unwrap_or(-1),
                            "success": output.status.success()
                        })),
                        None,
                    )
                }
                Err(e) => (None, Some(format!("Execution failed: {}", e))),
            }
        }
        _ => (
            None,
            Some(format!("Method '{}' is not supported by agent", method)),
        ),
    }
}
