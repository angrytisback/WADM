use chrono::Utc;
use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::time::{Duration, Instant};
use tokio::sync::Mutex;

use super::manifest::{PluginManifest, PluginRuntimeInfo, PluginState};
use super::rpc::{call_uds, RpcRequest};
use crate::drivers::error::AppError;

pub struct PluginProcess {
    pub child: tokio::process::Child,
    pub pid: u32,
    pub socket_path: PathBuf,
    pub started_at: String,
}

#[derive(Clone)]
pub struct PluginManager {
    pub plugins_dir: PathBuf,
    pub sockets_dir: PathBuf,
    processes: Arc<Mutex<HashMap<String, PluginProcess>>>,
    last_errors: Arc<Mutex<HashMap<String, String>>>,
}

pub fn default_plugins_dir() -> PathBuf {
    if let Ok(dir) = std::env::var("WADM_PLUGINS_DIR") {
        let p = PathBuf::from(dir);
        let _ = std::fs::create_dir_all(&p);
        return p;
    }
    if let Ok(data_dir) = std::env::var("WADM_DATA_DIR") {
        let p = PathBuf::from(data_dir).join("plugins");
        let _ = std::fs::create_dir_all(&p);
        return p;
    }
    let system_dir = Path::new("/var/lib/wadm/plugins");
    if system_dir.exists() && std::fs::create_dir_all(system_dir).is_ok() {
        return system_dir.to_path_buf();
    }
    let local_dir = std::env::current_dir()
        .unwrap_or_default()
        .join("data/plugins");
    let _ = std::fs::create_dir_all(&local_dir);
    local_dir
}

pub fn default_sockets_dir() -> PathBuf {
    if let Ok(dir) = std::env::var("WADM_SOCKETS_DIR") {
        let p = PathBuf::from(dir);
        let _ = std::fs::create_dir_all(&p);
        return p;
    }
    let system_dir = Path::new("/run/wadm/plugins");
    if (system_dir.exists() || std::fs::create_dir_all(system_dir).is_ok())
        && std::fs::metadata(system_dir)
            .map(|m| !m.permissions().readonly())
            .unwrap_or(false)
    {
        return system_dir.to_path_buf();
    }
    let local_dir = std::env::current_dir()
        .unwrap_or_default()
        .join("data/run/plugins");
    if std::fs::create_dir_all(&local_dir).is_ok() {
        return local_dir;
    }
    let tmp_dir = std::env::temp_dir().join("wadm-plugins");
    let _ = std::fs::create_dir_all(&tmp_dir);
    tmp_dir
}

impl PluginManager {
    pub fn new(plugins_dir: PathBuf, sockets_dir: PathBuf) -> Self {
        let _ = std::fs::create_dir_all(&plugins_dir);
        let _ = std::fs::create_dir_all(&sockets_dir);
        Self {
            plugins_dir,
            sockets_dir,
            processes: Arc::new(Mutex::new(HashMap::new())),
            last_errors: Arc::new(Mutex::new(HashMap::new())),
        }
    }

    pub fn default_instance() -> Self {
        Self::new(default_plugins_dir(), default_sockets_dir())
    }

    pub fn discover_manifests(&self) -> Vec<PluginManifest> {
        let mut manifests = Vec::new();
        let entries = match std::fs::read_dir(&self.plugins_dir) {
            Ok(e) => e,
            Err(_) => return manifests,
        };

        for entry in entries.flatten() {
            let path = entry.path();
            if path.is_dir() {
                let manifest_path = path.join("manifest.json");
                if manifest_path.is_file() {
                    if let Ok(content) = std::fs::read_to_string(&manifest_path) {
                        if let Ok(manifest) = serde_json::from_str::<PluginManifest>(&content) {
                            manifests.push(manifest);
                        } else {
                            log::warn!(
                                "Failed to parse plugin manifest at '{}'",
                                manifest_path.display()
                            );
                        }
                    }
                }
            }
        }

        manifests.sort_by(|a, b| a.id.cmp(&b.id));
        manifests
    }

    pub async fn list_plugins(&self) -> Vec<PluginRuntimeInfo> {
        let manifests = self.discover_manifests();
        let mut processes = self.processes.lock().await;
        let mut last_errors = self.last_errors.lock().await;

        // Clean up any processes that exited unexpectedly
        let mut dead_pids = Vec::new();
        for (id, proc) in processes.iter_mut() {
            match proc.child.try_wait() {
                Ok(Some(status)) => {
                    log::warn!(
                        "Plugin '{}' (PID {}) exited unexpectedly with status {}",
                        id,
                        proc.pid,
                        status
                    );
                    let _ = std::fs::remove_file(&proc.socket_path);
                    last_errors.insert(
                        id.clone(),
                        format!("Plugin process exited unexpectedly ({})", status),
                    );
                    dead_pids.push(id.clone());
                }
                Ok(None) => {}
                Err(e) => {
                    log::warn!("Error inspecting plugin '{}' child process: {}", id, e);
                }
            }
        }
        for dead_id in dead_pids {
            processes.remove(&dead_id);
        }

        let mut results = Vec::new();
        for manifest in manifests {
            let id = &manifest.id;
            let (state, pid, socket_path, started_at) = if let Some(proc) = processes.get(id) {
                (
                    PluginState::Running,
                    Some(proc.pid),
                    Some(proc.socket_path.to_string_lossy().to_string()),
                    Some(proc.started_at.clone()),
                )
            } else if last_errors.contains_key(id) {
                (PluginState::Crashed, None, None, None)
            } else {
                (PluginState::Stopped, None, None, None)
            };

            let error = last_errors.get(id).cloned();

            results.push(PluginRuntimeInfo {
                manifest,
                state,
                pid,
                socket_path,
                started_at,
                error,
            });
        }

        results
    }

    pub async fn get_plugin(&self, id: &str) -> Result<PluginRuntimeInfo, AppError> {
        let manifest_path = self.plugins_dir.join(id).join("manifest.json");
        if !manifest_path.exists() {
            return Err(AppError::NotFound(format!("Plugin '{}' not found", id)));
        }

        let content = std::fs::read_to_string(&manifest_path)
            .map_err(|e| AppError::ExecutionFailed(format!("Failed to read manifest: {}", e)))?;
        let manifest: PluginManifest = serde_json::from_str(&content)
            .map_err(|e| AppError::InvalidInput(format!("Invalid manifest format: {}", e)))?;

        let mut processes = self.processes.lock().await;
        let last_errors = self.last_errors.lock().await;

        if let Some(proc) = processes.get_mut(id) {
            if let Ok(Some(status)) = proc.child.try_wait() {
                let _ = std::fs::remove_file(&proc.socket_path);
                processes.remove(id);
                return Ok(PluginRuntimeInfo {
                    manifest,
                    state: PluginState::Crashed,
                    pid: None,
                    socket_path: None,
                    started_at: None,
                    error: Some(format!("Process exited with status {}", status)),
                });
            }
            Ok(PluginRuntimeInfo {
                manifest,
                state: PluginState::Running,
                pid: Some(proc.pid),
                socket_path: Some(proc.socket_path.to_string_lossy().to_string()),
                started_at: Some(proc.started_at.clone()),
                error: None,
            })
        } else {
            let error = last_errors.get(id).cloned();
            let state = if error.is_some() {
                PluginState::Crashed
            } else {
                PluginState::Stopped
            };
            Ok(PluginRuntimeInfo {
                manifest,
                state,
                pid: None,
                socket_path: None,
                started_at: None,
                error,
            })
        }
    }

    pub async fn start_plugin(&self, id: &str) -> Result<PluginRuntimeInfo, AppError> {
        let mut processes = self.processes.lock().await;
        let mut last_errors = self.last_errors.lock().await;

        if let Some(proc) = processes.get_mut(id) {
            if let Ok(None) = proc.child.try_wait() {
                return Err(AppError::InvalidInput(format!(
                    "Plugin '{}' is already running (PID {})",
                    id, proc.pid
                )));
            }
            // Already exited, clean up
            let _ = std::fs::remove_file(&proc.socket_path);
            processes.remove(id);
        }

        let plugin_dir = self.plugins_dir.join(id);
        let manifest_path = plugin_dir.join("manifest.json");
        if !manifest_path.exists() {
            return Err(AppError::NotFound(format!("Plugin '{}' not found", id)));
        }

        let content = std::fs::read_to_string(&manifest_path)
            .map_err(|e| AppError::ExecutionFailed(format!("Failed to read manifest: {}", e)))?;
        let manifest: PluginManifest = serde_json::from_str(&content)
            .map_err(|e| AppError::InvalidInput(format!("Invalid manifest format: {}", e)))?;

        let exec_path = if Path::new(&manifest.executable).is_absolute() {
            PathBuf::from(&manifest.executable)
        } else {
            plugin_dir.join(&manifest.executable)
        };

        if !exec_path.exists() {
            return Err(AppError::NotFound(format!(
                "Plugin executable '{}' not found",
                exec_path.display()
            )));
        }

        let _ = std::fs::create_dir_all(&self.sockets_dir);
        let socket_path = self.sockets_dir.join(format!("{}.sock", id));
        let _ = std::fs::remove_file(&socket_path);

        log::info!(
            "Starting plugin '{}' via executable '{}' with socket '{}'",
            id,
            exec_path.display(),
            socket_path.display()
        );

        let mut cmd = tokio::process::Command::new(&exec_path);
        cmd.arg("--socket").arg(&socket_path);
        cmd.current_dir(&plugin_dir);
        cmd.stdout(std::process::Stdio::null());
        cmd.stderr(std::process::Stdio::piped());

        let mut child = cmd.spawn().map_err(|e| {
            AppError::ExecutionFailed(format!(
                "Failed to spawn plugin executable '{}': {}",
                exec_path.display(),
                e
            ))
        })?;

        let pid = child.id().unwrap_or(0);

        // Wait for socket to become available (up to 3 seconds)
        let mut ready = false;
        for _ in 0..60 {
            if let Ok(Some(status)) = child.try_wait() {
                let mut err_msg =
                    format!("Plugin process exited prematurely with status {}", status);
                if let Some(mut stderr) = child.stderr.take() {
                    let mut buf = Vec::new();
                    use tokio::io::AsyncReadExt;
                    let _ = stderr.read_to_end(&mut buf).await;
                    if !buf.is_empty() {
                        err_msg.push_str(&format!(": {}", String::from_utf8_lossy(&buf).trim()));
                    }
                }
                let _ = std::fs::remove_file(&socket_path);
                last_errors.insert(id.to_string(), err_msg.clone());
                return Err(AppError::ExecutionFailed(err_msg));
            }

            if socket_path.exists() && tokio::net::UnixStream::connect(&socket_path).await.is_ok() {
                ready = true;
                break;
            }
            tokio::time::sleep(Duration::from_millis(50)).await;
        }

        if !ready {
            let _ = child.kill().await;
            let _ = std::fs::remove_file(&socket_path);
            let err_msg = format!(
                "Plugin '{}' failed to create ready Unix socket at '{}' within 3 seconds",
                id,
                socket_path.display()
            );
            last_errors.insert(id.to_string(), err_msg.clone());
            return Err(AppError::ExecutionFailed(err_msg));
        }

        // Perform plugin.init handshake
        let init_req = serde_json::json!({
            "jsonrpc": "2.0",
            "id": 1,
            "method": "plugin.init",
            "params": {
                "plugin_id": id,
                "version": manifest.version,
                "api_version": "1.0",
                "capabilities": manifest.capabilities
            }
        });

        match call_uds(&socket_path, &init_req, Duration::from_secs(4)).await {
            Ok(resp) => {
                log::info!("Plugin '{}' initialized successfully: {:?}", id, resp);
            }
            Err(e) => {
                log::error!("Plugin '{}' handshake failed: {}", id, e);
                let _ = child.kill().await;
                let _ = std::fs::remove_file(&socket_path);
                let err_msg = format!("Handshake with plugin '{}' failed: {}", id, e);
                last_errors.insert(id.to_string(), err_msg.clone());
                return Err(AppError::ExecutionFailed(err_msg));
            }
        }

        let started_at = Utc::now().to_rfc3339();
        processes.insert(
            id.to_string(),
            PluginProcess {
                child,
                pid,
                socket_path: socket_path.clone(),
                started_at: started_at.clone(),
            },
        );
        last_errors.remove(id);

        Ok(PluginRuntimeInfo {
            manifest,
            state: PluginState::Running,
            pid: Some(pid),
            socket_path: Some(socket_path.to_string_lossy().to_string()),
            started_at: Some(started_at),
            error: None,
        })
    }

    pub async fn stop_plugin(&self, id: &str) -> Result<PluginRuntimeInfo, AppError> {
        let manifest_path = self.plugins_dir.join(id).join("manifest.json");
        if !manifest_path.exists() {
            return Err(AppError::NotFound(format!("Plugin '{}' not found", id)));
        }

        let content = std::fs::read_to_string(&manifest_path)
            .map_err(|e| AppError::ExecutionFailed(format!("Failed to read manifest: {}", e)))?;
        let manifest: PluginManifest = serde_json::from_str(&content)
            .map_err(|e| AppError::InvalidInput(format!("Invalid manifest format: {}", e)))?;

        let mut processes = self.processes.lock().await;
        let mut last_errors = self.last_errors.lock().await;

        let proc = match processes.remove(id) {
            Some(p) => p,
            None => {
                return Ok(PluginRuntimeInfo {
                    manifest,
                    state: PluginState::Stopped,
                    pid: None,
                    socket_path: None,
                    started_at: None,
                    error: None,
                });
            }
        };

        // Attempt graceful JSON-RPC plugin.shutdown
        let shutdown_req = serde_json::json!({
            "jsonrpc": "2.0",
            "id": 999,
            "method": "plugin.shutdown",
            "params": {}
        });
        let _ = call_uds(&proc.socket_path, &shutdown_req, Duration::from_millis(600)).await;

        // Try SIGTERM
        let _ = tokio::process::Command::new("kill")
            .args(["-15", &proc.pid.to_string()])
            .output()
            .await;

        let mut child = proc.child;
        if tokio::time::timeout(Duration::from_secs(2), child.wait())
            .await
            .is_err()
        {
            let _ = child.kill().await;
        }

        let _ = std::fs::remove_file(&proc.socket_path);
        last_errors.remove(id);

        Ok(PluginRuntimeInfo {
            manifest,
            state: PluginState::Stopped,
            pid: None,
            socket_path: None,
            started_at: None,
            error: None,
        })
    }

    pub async fn forward_rpc(
        &self,
        id: &str,
        request: &serde_json::Value,
    ) -> Result<serde_json::Value, AppError> {
        let socket_path = {
            let mut processes = self.processes.lock().await;
            if let Some(proc) = processes.get_mut(id) {
                if let Ok(Some(status)) = proc.child.try_wait() {
                    let _ = std::fs::remove_file(&proc.socket_path);
                    let mut last_errors = self.last_errors.lock().await;
                    last_errors.insert(
                        id.to_string(),
                        format!("Process exited unexpectedly ({})", status),
                    );
                    processes.remove(id);
                    return Err(AppError::ExecutionFailed(format!(
                        "Plugin '{}' crashed with status {}",
                        id, status
                    )));
                }
                proc.socket_path.clone()
            } else {
                return Err(AppError::ExecutionFailed(format!(
                    "Plugin '{}' is not running",
                    id
                )));
            }
        };

        call_uds(&socket_path, request, Duration::from_secs(15)).await
    }

    pub async fn ping_plugin(&self, id: &str) -> Result<serde_json::Value, AppError> {
        let req = RpcRequest::new(Some(serde_json::json!(1)), "ping", None);
        let req_val = serde_json::to_value(req).map_err(|e| {
            AppError::InvalidInput(format!("Failed to serialize ping request: {}", e))
        })?;

        let start = Instant::now();
        let resp = self.forward_rpc(id, &req_val).await?;
        let latency_ms = start.elapsed().as_secs_f64() * 1000.0;

        Ok(serde_json::json!({
            "response": resp,
            "latency_ms": (latency_ms * 100.0).round() / 100.0
        }))
    }

    #[allow(dead_code)]
    pub async fn shutdown_all(&self) {
        let mut processes = self.processes.lock().await;
        for (id, mut proc) in processes.drain() {
            log::info!("Shutting down plugin '{}' (PID {})", id, proc.pid);
            let _ = tokio::process::Command::new("kill")
                .args(["-15", &proc.pid.to_string()])
                .output()
                .await;
            if tokio::time::timeout(Duration::from_millis(500), proc.child.wait())
                .await
                .is_err()
            {
                let _ = proc.child.kill().await;
            }
            let _ = std::fs::remove_file(&proc.socket_path);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::plugins::manifest::{PluginEntryType, PluginUiConfig};
    use std::fs;

    #[tokio::test]
    async fn test_plugin_discovery_and_lifecycle() {
        let temp_dir = std::env::temp_dir().join(format!("wadm_test_{}", uuid::Uuid::new_v4()));
        let plugins_dir = temp_dir.join("plugins");
        let sockets_dir = temp_dir.join("sockets");
        fs::create_dir_all(&plugins_dir).unwrap();
        fs::create_dir_all(&sockets_dir).unwrap();

        let plugin_id = "test-plugin";
        let test_plugin_dir = plugins_dir.join(plugin_id);
        fs::create_dir_all(&test_plugin_dir).unwrap();

        let manifest = PluginManifest {
            id: plugin_id.to_string(),
            name: "Test Plugin".to_string(),
            version: "0.1.0".to_string(),
            author: "Tester".to_string(),
            description: "Test desc".to_string(),
            executable: "run.sh".to_string(),
            ui: PluginUiConfig {
                tab_id: "test".to_string(),
                title: "Test".to_string(),
                icon: "box".to_string(),
                entry_type: PluginEntryType::DeclarativeSchema,
            },
            capabilities: vec!["test".to_string()],
        };

        fs::write(
            test_plugin_dir.join("manifest.json"),
            serde_json::to_string_pretty(&manifest).unwrap(),
        )
        .unwrap();

        // Write a simple runner script for testing that sets up a socket listener
        // We can create a simple python script or mock runner
        let python_runner = r#"#!/usr/bin/env python3
import sys, socket, json, os

socket_path = None
for i in range(len(sys.argv)):
    if sys.argv[i] == "--socket" and i + 1 < len(sys.argv):
        socket_path = sys.argv[i + 1]

if not socket_path:
    sys.exit(1)

if os.path.exists(socket_path):
    os.remove(socket_path)

server = socket.socket(socket.AF_UNIX, socket.SOCK_STREAM)
server.bind(socket_path)
server.listen(5)

running = True
while running:
    try:
        conn, _ = server.accept()
        file = conn.makefile("rwb", buffering=0)
        line = file.readline()
        if not line:
            conn.close()
            continue
        req = json.loads(line.decode("utf-8"))
        method = req.get("method")
        msg_id = req.get("id")

        if method == "plugin.init":
            resp = {"jsonrpc": "2.0", "id": msg_id, "result": {"status": "initialized"}}
        elif method == "ping":
            resp = {"jsonrpc": "2.0", "id": msg_id, "result": "pong"}
        elif method == "echo":
            resp = {"jsonrpc": "2.0", "id": msg_id, "result": req.get("params")}
        elif method == "plugin.shutdown":
            resp = {"jsonrpc": "2.0", "id": msg_id, "result": "shutting_down"}
            running = False
        else:
            resp = {"jsonrpc": "2.0", "id": msg_id, "error": {"code": -32601, "message": "Method not found"}}

        out = (json.dumps(resp) + "\n").encode("utf-8")
        file.write(out)
        conn.close()
    except Exception as e:
        break

server.close()
"#;

        let script_path = test_plugin_dir.join("run.sh");
        fs::write(&script_path, python_runner).unwrap();

        // Make executable
        use std::os::unix::fs::PermissionsExt;
        let mut perms = fs::metadata(&script_path).unwrap().permissions();
        perms.set_mode(0o755);
        fs::set_permissions(&script_path, perms).unwrap();

        let manager = PluginManager::new(plugins_dir, sockets_dir);

        // Discovery test
        let discovered = manager.discover_manifests();
        assert_eq!(discovered.len(), 1);
        assert_eq!(discovered[0].id, plugin_id);

        let list_before = manager.list_plugins().await;
        assert_eq!(list_before.len(), 1);
        assert_eq!(list_before[0].state, PluginState::Stopped);

        // Check if python3 is available to run lifecycle test
        if std::process::Command::new("python3")
            .arg("--version")
            .output()
            .is_ok()
        {
            // Start plugin
            let start_res = manager.start_plugin(plugin_id).await;
            assert!(start_res.is_ok(), "Failed to start: {:?}", start_res.err());
            let started_info = start_res.unwrap();
            assert_eq!(started_info.state, PluginState::Running);
            assert!(started_info.pid.is_some());

            // Ping plugin
            let ping_res = manager.ping_plugin(plugin_id).await;
            assert!(ping_res.is_ok(), "Failed to ping: {:?}", ping_res.err());
            let ping_val = ping_res.unwrap();
            assert_eq!(ping_val["response"]["result"], "pong");

            // Forward custom RPC
            let custom_rpc = serde_json::json!({
                "jsonrpc": "2.0",
                "id": 42,
                "method": "echo",
                "params": {"message": "hello from wadm"}
            });
            let echo_res = manager.forward_rpc(plugin_id, &custom_rpc).await;
            assert!(echo_res.is_ok());
            assert_eq!(echo_res.unwrap()["result"]["message"], "hello from wadm");

            // Stop plugin
            let stop_res = manager.stop_plugin(plugin_id).await;
            assert!(stop_res.is_ok());
            assert_eq!(stop_res.unwrap().state, PluginState::Stopped);
        }

        let _ = fs::remove_dir_all(&temp_dir);
    }
}
