use actix_web::{web, HttpMessage, HttpRequest, HttpResponse, Responder};
use chrono::{Duration, Utc};
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use uuid::Uuid;

use crate::cluster::db::ClusterDatabase;
use crate::cluster::hub::ClusterManager;

#[derive(Serialize)]
pub struct JoinTokenResponse {
    pub token: String,
    pub command: String,
    pub expires_at: String,
}

#[derive(Deserialize)]
pub struct TunnelQuery {
    pub token: Option<String>,
    pub wadm_token: Option<String>,
}

#[derive(Deserialize)]
pub struct ProxyCommandBody {
    pub cmd: String,
}

#[derive(Deserialize)]
pub struct ProxyServiceBody {
    pub action: String,
}

#[derive(Deserialize)]
pub struct ProxyJobBody {
    pub cmd: String,
    pub description: Option<String>,
}

pub async fn list_nodes(
    cluster_db: web::Data<Arc<ClusterDatabase>>,
    cluster_manager: web::Data<Arc<ClusterManager>>,
) -> impl Responder {
    match cluster_db.list_nodes() {
        Ok(mut nodes) => {
            // Reconcile status with active in-memory connections
            for node in &mut nodes {
                if cluster_manager.is_node_connected(&node.id).await {
                    node.status = "online".to_string();
                } else if node.status == "online" {
                    // Check if stale
                    node.status = "offline".to_string();
                }
            }
            HttpResponse::Ok().json(nodes)
        }
        Err(e) => {
            log::error!("Failed to query cluster nodes: {}", e);
            HttpResponse::InternalServerError().json(serde_json::json!({
                "error": "Failed to query nodes"
            }))
        }
    }
}

pub async fn get_node(
    cluster_db: web::Data<Arc<ClusterDatabase>>,
    cluster_manager: web::Data<Arc<ClusterManager>>,
    path: web::Path<String>,
) -> impl Responder {
    let node_id = path.into_inner();
    match cluster_db.get_node(&node_id) {
        Ok(Some(mut node)) => {
            if cluster_manager.is_node_connected(&node.id).await {
                node.status = "online".to_string();
            } else if node.status == "online" {
                node.status = "offline".to_string();
            }
            HttpResponse::Ok().json(node)
        }
        Ok(None) => HttpResponse::NotFound().json(serde_json::json!({
            "error": "Node not found"
        })),
        Err(e) => HttpResponse::InternalServerError().json(serde_json::json!({
            "error": format!("Database error: {}", e)
        })),
    }
}

pub async fn generate_join_token(
    req: HttpRequest,
    cluster_db: web::Data<Arc<ClusterDatabase>>,
    audit: web::Data<Arc<crate::audit::AuditLogger>>,
) -> impl Responder {
    let client_ip = crate::auth::extract_client_ip(&req);
    let username = req
        .extensions()
        .get::<crate::api::auth::Claims>()
        .map(|c| c.sub.clone())
        .unwrap_or_else(|| "admin".to_string());

    let token = format!("wadm_join_{}", Uuid::new_v4().to_string().replace('-', ""));
    let expires_at = Utc::now() + Duration::hours(1);

    if let Err(e) = cluster_db.create_join_token(&token, expires_at) {
        log::error!("Failed to create cluster join token: {}", e);
        return HttpResponse::InternalServerError().json(serde_json::json!({
            "error": "Failed to create join token"
        }));
    }

    audit.log(
        &username,
        "admin",
        "CLUSTER_TOKEN_GENERATE",
        None,
        &client_ip,
        "SUCCESS",
        Some("Generated 1-hour cluster agent join token"),
    );

    let host = req.connection_info().host().to_string();
    let scheme = if req.connection_info().scheme() == "https" {
        "wss"
    } else {
        "ws"
    };
    let hub_url = format!("{}://{}/api/cluster/tunnel", scheme, host);
    let command = format!("wadm --agent --hub-url {} --token {}", hub_url, token);

    HttpResponse::Ok().json(JoinTokenResponse {
        token,
        command,
        expires_at: expires_at.to_rfc3339(),
    })
}

pub async fn delete_node(
    req: HttpRequest,
    path: web::Path<String>,
    cluster_db: web::Data<Arc<ClusterDatabase>>,
    cluster_manager: web::Data<Arc<ClusterManager>>,
    audit: web::Data<Arc<crate::audit::AuditLogger>>,
) -> impl Responder {
    let node_id = path.into_inner();
    let client_ip = crate::auth::extract_client_ip(&req);
    let username = req
        .extensions()
        .get::<crate::api::auth::Claims>()
        .map(|c| c.sub.clone())
        .unwrap_or_else(|| "admin".to_string());

    // Disconnect active tunnel if running
    cluster_manager.unregister_connection(&node_id).await;

    match cluster_db.delete_node(&node_id) {
        Ok(true) => {
            audit.log(
                &username,
                "admin",
                "CLUSTER_NODE_DELETE",
                Some(&node_id),
                &client_ip,
                "SUCCESS",
                Some("Node removed from cluster"),
            );
            HttpResponse::Ok().json(serde_json::json!({ "success": true }))
        }
        Ok(false) => {
            HttpResponse::NotFound().json(serde_json::json!({ "error": "Node not found" }))
        }
        Err(e) => HttpResponse::InternalServerError().json(serde_json::json!({
            "error": format!("Database error: {}", e)
        })),
    }
}

pub async fn tunnel_handler(
    req: HttpRequest,
    stream: web::Payload,
    query: web::Query<TunnelQuery>,
    cluster_manager: web::Data<Arc<ClusterManager>>,
) -> Result<HttpResponse, actix_web::Error> {
    let client_ip = crate::auth::extract_client_ip(&req);

    // Extract token from query or Authorization header or Sec-WebSocket-Protocol
    let mut initial_token = query.token.clone().or_else(|| query.wadm_token.clone());

    if initial_token.is_none() {
        if let Some(auth_hdr) = req.headers().get("Authorization") {
            if let Ok(val) = auth_hdr.to_str() {
                let parts: Vec<&str> = val.split_whitespace().collect();
                if parts.len() == 2 && parts[0].eq_ignore_ascii_case("bearer") {
                    initial_token = Some(parts[1].to_string());
                }
            }
        }
    }

    if initial_token.is_none() {
        if let Some(proto) = req.headers().get("Sec-WebSocket-Protocol") {
            if let Ok(p) = proto.to_str() {
                let clean = p.trim().to_string();
                if !clean.is_empty() {
                    initial_token = Some(clean);
                }
            }
        }
    }

    let (res, session, msg_stream) = actix_ws::handle(&req, stream)?;

    let manager_clone = cluster_manager.get_ref().clone();
    actix_web::rt::spawn(async move {
        manager_clone
            .handle_ws_session(session, msg_stream, client_ip, initial_token)
            .await;
    });

    Ok(res)
}

// Targeted Proxy Route Handlers

pub async fn proxy_node_stats(
    path: web::Path<String>,
    cluster_manager: web::Data<Arc<ClusterManager>>,
) -> impl Responder {
    let node_id = path.into_inner();
    match cluster_manager
        .forward_to_node(
            &node_id,
            "get_stats",
            serde_json::json!({}),
            std::time::Duration::from_secs(10),
        )
        .await
    {
        Ok(stats) => HttpResponse::Ok().json(stats),
        Err(e) => HttpResponse::ServiceUnavailable().json(serde_json::json!({
            "error": e
        })),
    }
}

pub async fn proxy_node_system(
    path: web::Path<String>,
    cluster_manager: web::Data<Arc<ClusterManager>>,
) -> impl Responder {
    let node_id = path.into_inner();
    match cluster_manager
        .forward_to_node(
            &node_id,
            "get_system_info",
            serde_json::json!({}),
            std::time::Duration::from_secs(10),
        )
        .await
    {
        Ok(info) => HttpResponse::Ok().json(info),
        Err(e) => HttpResponse::ServiceUnavailable().json(serde_json::json!({
            "error": e
        })),
    }
}

pub async fn proxy_node_services(
    path: web::Path<String>,
    cluster_manager: web::Data<Arc<ClusterManager>>,
) -> impl Responder {
    let node_id = path.into_inner();
    match cluster_manager
        .forward_to_node(
            &node_id,
            "list_services",
            serde_json::json!({}),
            std::time::Duration::from_secs(15),
        )
        .await
    {
        Ok(services) => HttpResponse::Ok().json(services),
        Err(e) => HttpResponse::ServiceUnavailable().json(serde_json::json!({
            "error": e
        })),
    }
}

pub async fn proxy_node_service_action(
    path: web::Path<(String, String)>,
    body: web::Json<ProxyServiceBody>,
    cluster_manager: web::Data<Arc<ClusterManager>>,
) -> impl Responder {
    let (node_id, service_name) = path.into_inner();
    match cluster_manager
        .forward_to_node(
            &node_id,
            "service_action",
            serde_json::json!({
                "service": service_name,
                "action": body.action
            }),
            std::time::Duration::from_secs(20),
        )
        .await
    {
        Ok(res) => HttpResponse::Ok().json(res),
        Err(e) => HttpResponse::ServiceUnavailable().json(serde_json::json!({
            "error": e
        })),
    }
}

pub async fn proxy_node_jobs(
    path: web::Path<String>,
    cluster_manager: web::Data<Arc<ClusterManager>>,
) -> impl Responder {
    let node_id = path.into_inner();
    match cluster_manager
        .forward_to_node(
            &node_id,
            "list_jobs",
            serde_json::json!({}),
            std::time::Duration::from_secs(10),
        )
        .await
    {
        Ok(jobs) => HttpResponse::Ok().json(jobs),
        Err(e) => HttpResponse::ServiceUnavailable().json(serde_json::json!({
            "error": e
        })),
    }
}

pub async fn proxy_node_execute_job(
    path: web::Path<String>,
    body: web::Json<ProxyJobBody>,
    cluster_manager: web::Data<Arc<ClusterManager>>,
) -> impl Responder {
    let node_id = path.into_inner();
    match cluster_manager
        .forward_to_node(
            &node_id,
            "execute_job",
            serde_json::json!({
                "cmd": body.cmd,
                "description": body.description
            }),
            std::time::Duration::from_secs(15),
        )
        .await
    {
        Ok(res) => HttpResponse::Ok().json(res),
        Err(e) => HttpResponse::ServiceUnavailable().json(serde_json::json!({
            "error": e
        })),
    }
}

pub async fn proxy_node_command(
    path: web::Path<String>,
    body: web::Json<ProxyCommandBody>,
    cluster_manager: web::Data<Arc<ClusterManager>>,
) -> impl Responder {
    let node_id = path.into_inner();
    match cluster_manager
        .forward_to_node(
            &node_id,
            "execute_command",
            serde_json::json!({ "cmd": body.cmd }),
            std::time::Duration::from_secs(30),
        )
        .await
    {
        Ok(res) => HttpResponse::Ok().json(res),
        Err(e) => HttpResponse::ServiceUnavailable().json(serde_json::json!({
            "error": e
        })),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use actix_web::test;

    #[actix_web::test]
    async fn test_list_nodes_empty() {
        let db = Arc::new(ClusterDatabase::new_in_memory().unwrap());
        let mgr = ClusterManager::new(db.clone());

        let resp = list_nodes(web::Data::new(db), web::Data::new(mgr))
            .await
            .respond_to(&test::TestRequest::default().to_http_request());

        assert_eq!(resp.status(), actix_web::http::StatusCode::OK);
    }

    #[actix_web::test]
    async fn test_get_nonexistent_node_404() {
        let db = Arc::new(ClusterDatabase::new_in_memory().unwrap());
        let mgr = ClusterManager::new(db.clone());

        let resp = get_node(
            web::Data::new(db),
            web::Data::new(mgr),
            web::Path::from("non-existent-uuid".to_string()),
        )
        .await
        .respond_to(&test::TestRequest::default().to_http_request());

        assert_eq!(resp.status(), actix_web::http::StatusCode::NOT_FOUND);
    }

    #[actix_web::test]
    async fn test_proxy_node_stats_offline_503() {
        let db = Arc::new(ClusterDatabase::new_in_memory().unwrap());
        let mgr = ClusterManager::new(db.clone());

        let resp = proxy_node_stats(
            web::Path::from("offline-node".to_string()),
            web::Data::new(mgr),
        )
        .await
        .respond_to(&test::TestRequest::default().to_http_request());

        assert_eq!(
            resp.status(),
            actix_web::http::StatusCode::SERVICE_UNAVAILABLE
        );
    }
}
