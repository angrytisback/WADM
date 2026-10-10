use actix_ws::{Message, Session};
use chrono::Utc;
use futures_util::StreamExt;
use std::collections::HashMap;
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};
use tokio::sync::{mpsc, oneshot, RwLock};
use uuid::Uuid;

use super::db::{hash_node_token, ClusterDatabase, Node};
use super::protocol::{ClusterMessage, NodeSpecs};

pub type PendingRequestsMap =
    Arc<Mutex<HashMap<String, oneshot::Sender<Result<serde_json::Value, String>>>>>;

#[allow(dead_code)]
pub struct NodeConnection {
    pub node_id: String,
    pub tx: mpsc::Sender<ClusterMessage>,
    pub pending_requests: PendingRequestsMap,
    pub connected_at: Instant,
}

pub struct ClusterManager {
    pub db: Arc<ClusterDatabase>,
    pub active_nodes: Arc<RwLock<HashMap<String, NodeConnection>>>,
}

impl ClusterManager {
    pub fn new(db: Arc<ClusterDatabase>) -> Arc<Self> {
        Arc::new(Self {
            db,
            active_nodes: Arc::new(RwLock::new(HashMap::new())),
        })
    }

    pub async fn register_connection(
        &self,
        node_id: String,
        tx: mpsc::Sender<ClusterMessage>,
        pending_requests: PendingRequestsMap,
    ) {
        let mut nodes = self.active_nodes.write().await;
        nodes.insert(
            node_id.clone(),
            NodeConnection {
                node_id: node_id.clone(),
                tx,
                pending_requests,
                connected_at: Instant::now(),
            },
        );
        log::info!(
            "Cluster node {} successfully registered in active tunnel pool",
            node_id
        );
    }

    pub async fn unregister_connection(&self, node_id: &str) {
        let mut nodes = self.active_nodes.write().await;
        if nodes.remove(node_id).is_some() {
            log::info!("Cluster node {} removed from active tunnel pool", node_id);
        }
    }

    pub async fn is_node_connected(&self, node_id: &str) -> bool {
        let nodes = self.active_nodes.read().await;
        nodes.contains_key(node_id)
    }

    #[allow(dead_code)]
    pub async fn connected_node_ids(&self) -> Vec<String> {
        let nodes = self.active_nodes.read().await;
        nodes.keys().cloned().collect()
    }

    pub async fn forward_to_node(
        &self,
        node_id: &str,
        method: &str,
        params: serde_json::Value,
        timeout_duration: Duration,
    ) -> Result<serde_json::Value, String> {
        let (tx, pending_map) = {
            let nodes = self.active_nodes.read().await;
            if let Some(conn) = nodes.get(node_id) {
                (conn.tx.clone(), conn.pending_requests.clone())
            } else {
                return Err(format!(
                    "Node '{}' is offline or not connected to cluster",
                    node_id
                ));
            }
        };

        let req_id = Uuid::new_v4().to_string();
        let (resp_tx, resp_rx) = oneshot::channel();

        {
            let mut pending = pending_map.lock().unwrap();
            pending.insert(req_id.clone(), resp_tx);
        }

        let msg = ClusterMessage::RpcRequest {
            id: req_id.clone(),
            method: method.to_string(),
            params,
        };

        if let Err(e) = tx.send(msg).await {
            let mut pending = pending_map.lock().unwrap();
            pending.remove(&req_id);
            return Err(format!("Failed to forward request to node tunnel: {}", e));
        }

        match tokio::time::timeout(timeout_duration, resp_rx).await {
            Ok(Ok(result)) => result,
            Ok(Err(_)) => {
                let mut pending = pending_map.lock().unwrap();
                pending.remove(&req_id);
                Err("Node dropped tunnel connection before responding".to_string())
            }
            Err(_) => {
                let mut pending = pending_map.lock().unwrap();
                pending.remove(&req_id);
                Err(format!(
                    "Timed out waiting for response from node '{}' after {:?}",
                    node_id, timeout_duration
                ))
            }
        }
    }

    pub async fn handle_ws_session(
        self: &Arc<Self>,
        mut session: Session,
        mut stream: actix_ws::MessageStream,
        client_ip: String,
        initial_token: Option<String>,
    ) {
        let mut authenticated_node: Option<Node> = None;
        let pending_requests: PendingRequestsMap = Arc::new(Mutex::new(HashMap::new()));
        let (out_tx, mut out_rx) = mpsc::channel::<ClusterMessage>(64);

        // Try authenticating with initial_token if provided via header or query
        if let Some(ref token) = initial_token {
            let token_hash = hash_node_token(token);
            if let Ok(Some(node)) = self.db.get_node_by_auth_token_hash(&token_hash) {
                let _ = self.db.update_status(&node.id, "online");
                let _ = self.db.update_heartbeat(&node.id, None, None);
                authenticated_node = Some(node);
            } else if let Ok(true) = self.db.validate_and_consume_join_token(token) {
                // Token is a valid join token
                let new_node_token = Uuid::new_v4().to_string();
                let new_token_hash = hash_node_token(&new_node_token);
                let default_specs = NodeSpecs::default().to_json_string();
                let node_name = format!("worker-{}", &Uuid::new_v4().to_string()[..8]);

                if let Ok(node) = self.db.register_or_update_node(
                    None,
                    &node_name,
                    "agent-host",
                    &client_ip,
                    &new_token_hash,
                    "0.96.0",
                    &default_specs,
                ) {
                    let _ = out_tx
                        .send(ClusterMessage::HandshakeAck {
                            node_id: node.id.clone(),
                            auth_token: new_node_token,
                            message: "Registered into WADM cluster successfully".to_string(),
                        })
                        .await;
                    authenticated_node = Some(node);
                }
            }
        }

        // If authenticated upfront, register immediately
        if let Some(ref node) = authenticated_node {
            self.register_connection(node.id.clone(), out_tx.clone(), pending_requests.clone())
                .await;
        }

        // Spawn task to forward outgoing ClusterMessages to actix_ws session
        let mut session_writer = session.clone();
        let writer_task = tokio::spawn(async move {
            while let Some(msg) = out_rx.recv().await {
                if let Ok(json) = serde_json::to_string(&msg) {
                    if session_writer.text(json).await.is_err() {
                        break;
                    }
                }
            }
        });

        // Reading incoming messages from agent
        while let Some(Ok(msg)) = stream.next().await {
            match msg {
                Message::Text(text) => {
                    if let Ok(cluster_msg) = serde_json::from_str::<ClusterMessage>(&text) {
                        match cluster_msg {
                            ClusterMessage::Handshake {
                                node_id,
                                name,
                                hostname,
                                version,
                                specs,
                                token,
                            } => {
                                let token_hash = hash_node_token(&token);
                                let mut resolved_node = None;

                                if let Ok(Some(existing)) =
                                    self.db.get_node_by_auth_token_hash(&token_hash)
                                {
                                    let _ = self.db.register_or_update_node(
                                        Some(&existing.id),
                                        &name,
                                        &hostname,
                                        &client_ip,
                                        &token_hash,
                                        &version,
                                        &specs.to_json_string(),
                                    );
                                    resolved_node = Some(existing);
                                } else if let Ok(true) =
                                    self.db.validate_and_consume_join_token(&token)
                                {
                                    let new_node_token = Uuid::new_v4().to_string();
                                    let new_token_hash = hash_node_token(&new_node_token);

                                    if let Ok(new_node) = self.db.register_or_update_node(
                                        node_id.as_deref(),
                                        &name,
                                        &hostname,
                                        &client_ip,
                                        &new_token_hash,
                                        &version,
                                        &specs.to_json_string(),
                                    ) {
                                        let _ = out_tx
                                            .send(ClusterMessage::HandshakeAck {
                                                node_id: new_node.id.clone(),
                                                auth_token: new_node_token,
                                                message: "Registered into cluster".to_string(),
                                            })
                                            .await;
                                        resolved_node = Some(new_node);
                                    }
                                }

                                if let Some(n) = resolved_node {
                                    self.register_connection(
                                        n.id.clone(),
                                        out_tx.clone(),
                                        pending_requests.clone(),
                                    )
                                    .await;
                                    authenticated_node = Some(n);
                                } else {
                                    let _ = out_tx
                                        .send(ClusterMessage::Disconnect {
                                            reason: "Invalid or expired join token".to_string(),
                                        })
                                        .await;
                                    let _ = session.close(None).await;
                                    break;
                                }
                            }
                            ClusterMessage::Ping { specs, version } => {
                                if let Some(ref node) = authenticated_node {
                                    let specs_str = specs.map(|s| s.to_json_string());
                                    let _ = self.db.update_heartbeat(
                                        &node.id,
                                        specs_str.as_deref(),
                                        version.as_deref(),
                                    );
                                    let _ = out_tx
                                        .send(ClusterMessage::Pong {
                                            timestamp: Utc::now().timestamp(),
                                        })
                                        .await;
                                }
                            }
                            ClusterMessage::RpcResponse { id, result, error } => {
                                let mut pending = pending_requests.lock().unwrap();
                                if let Some(sender) = pending.remove(&id) {
                                    if let Some(err) = error {
                                        let _ = sender.send(Err(err));
                                    } else {
                                        let _ = sender
                                            .send(Ok(result.unwrap_or(serde_json::Value::Null)));
                                    }
                                }
                            }
                            ClusterMessage::Disconnect { reason } => {
                                log::info!("Agent requested disconnect: {}", reason);
                                break;
                            }
                            _ => {}
                        }
                    }
                }
                Message::Ping(bytes) => {
                    let _ = session.pong(&bytes).await;
                }
                Message::Close(_) => {
                    break;
                }
                _ => {}
            }
        }

        writer_task.abort();

        if let Some(node) = authenticated_node {
            self.unregister_connection(&node.id).await;
            let _ = self.db.update_status(&node.id, "offline");
        }
    }
}

pub fn start_heartbeat_auditor(manager: Arc<ClusterManager>, db: Arc<ClusterDatabase>) {
    tokio::spawn(async move {
        log::info!("Starting cluster heartbeat auditor daemon (5s interval)...");
        let mut interval = tokio::time::interval(Duration::from_secs(5));
        loop {
            interval.tick().await;
            match db.mark_stale_nodes_offline(15) {
                Ok(stale_nodes) => {
                    for node_id in stale_nodes {
                        log::warn!(
                            "Node '{}' heartbeat timed out (>15s silent). Marked offline.",
                            node_id
                        );
                        manager.unregister_connection(&node_id).await;
                    }
                }
                Err(e) => {
                    log::error!("Error checking cluster stale nodes: {}", e);
                }
            }
        }
    });
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_cluster_manager_registration_and_offline_forward() {
        let db = Arc::new(ClusterDatabase::new_in_memory().unwrap());
        let manager = ClusterManager::new(db);

        assert!(!manager.is_node_connected("test-node").await);

        // Forward to offline node should fail immediately
        let err = manager
            .forward_to_node(
                "test-node",
                "get_stats",
                serde_json::json!({}),
                Duration::from_millis(500),
            )
            .await;

        assert!(err.is_err());
        assert!(err.unwrap_err().contains("offline"));
    }

    #[tokio::test]
    async fn test_cluster_manager_forward_rpc_roundtrip() {
        let db = Arc::new(ClusterDatabase::new_in_memory().unwrap());
        let manager = ClusterManager::new(db);
        let node_id = "node-alpha".to_string();

        let (tx, mut rx) = mpsc::channel(16);
        let pending = Arc::new(Mutex::new(HashMap::new()));
        manager
            .register_connection(node_id.clone(), tx, pending.clone())
            .await;

        assert!(manager.is_node_connected(&node_id).await);

        // Spawn mock agent on rx
        let pending_clone = pending.clone();
        tokio::spawn(async move {
            if let Some(ClusterMessage::RpcRequest { id, method, .. }) = rx.recv().await {
                assert_eq!(method, "get_stats");
                // Reply
                let mut p = pending_clone.lock().unwrap();
                if let Some(sender) = p.remove(&id) {
                    let _ = sender.send(Ok(serde_json::json!({ "cpu_usage": 42.0 })));
                }
            }
        });

        let resp = manager
            .forward_to_node(
                &node_id,
                "get_stats",
                serde_json::json!({}),
                Duration::from_secs(2),
            )
            .await
            .expect("RPC forwarding failed");

        assert_eq!(resp.get("cpu_usage").unwrap().as_f64().unwrap(), 42.0);

        manager.unregister_connection(&node_id).await;
        assert!(!manager.is_node_connected(&node_id).await);
    }
}
