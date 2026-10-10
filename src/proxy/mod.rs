pub mod db;
pub mod service;

use actix_web::{web, HttpRequest, HttpResponse};
use std::sync::Arc;

pub use db::{ProxyDatabase, ProxyRoute};
pub use service::{is_websocket_request, proxy_http, proxy_websocket};

use crate::audit::AuditLogger;
use crate::auth::{AuthenticatedUser, RequireAdmin};
use crate::drivers::error::AppError;

#[derive(rust_embed::RustEmbed)]
#[folder = "web/dist/"]
pub struct Asset;

pub fn serve_embedded_asset(path: &str) -> HttpResponse {
    let clean_path = path.trim_start_matches('/');
    let target = if clean_path.is_empty() {
        "index.html"
    } else {
        clean_path
    };

    if let Some(file) = Asset::get(target) {
        let mime = mime_guess::from_path(target).first_or_octet_stream();
        let mut resp = HttpResponse::Ok();
        resp.content_type(mime.as_ref());
        if target.starts_with("assets/") {
            resp.insert_header(("Cache-Control", "public, max-age=31536000, immutable"));
        } else if target == "index.html" {
            resp.insert_header(("Cache-Control", "no-cache, must-revalidate"));
        }
        resp.body(file.data.into_owned())
    } else if let Some(index) = Asset::get("index.html") {
        // SPA Fallback for client-side HTML5 history navigation
        HttpResponse::Ok()
            .content_type("text/html; charset=utf-8")
            .insert_header(("Cache-Control", "no-cache, must-revalidate"))
            .body(index.data.into_owned())
    } else {
        HttpResponse::NotFound().body("WADM Web assets not found in binary.")
    }
}

pub async fn proxy_subpath_handler(
    req: HttpRequest,
    body: web::Payload,
    proxy_db: web::Data<Arc<ProxyDatabase>>,
) -> Result<HttpResponse, actix_web::Error> {
    let path = req.uri().path();
    // Path format: /apps/{app_id} or /apps/{app_id}/...
    let segments: Vec<&str> = path.trim_start_matches('/').split('/').collect();
    if segments.len() < 2 || segments[0] != "apps" {
        return Ok(HttpResponse::NotFound().json(serde_json::json!({
            "error": "Not Found",
            "message": "Invalid application proxy path"
        })));
    }

    let app_id = segments[1];
    let route = match proxy_db.get_route_by_app_id(app_id) {
        Ok(Some(r)) => r,
        Ok(None) => {
            return Ok(HttpResponse::NotFound().json(serde_json::json!({
                "error": "Not Found",
                "message": format!("No active reverse proxy route found for application '{}'", app_id)
            })));
        }
        Err(e) => {
            log::error!(
                "Database error fetching proxy route for '{}': {}",
                app_id,
                e
            );
            return Ok(HttpResponse::InternalServerError().json(serde_json::json!({
                "error": "Internal Error",
                "message": "Failed to resolve proxy route"
            })));
        }
    };

    let prefix = format!("/apps/{}", app_id);

    if is_websocket_request(&req) && route.websocket_support {
        proxy_websocket(req, body, &route.target_url, Some(&prefix)).await
    } else {
        proxy_http(req, body, &route.target_url, Some(&prefix)).await
    }
}

pub async fn proxy_domain_or_static_handler(
    req: HttpRequest,
    body: web::Payload,
    proxy_db: web::Data<Arc<ProxyDatabase>>,
) -> Result<HttpResponse, actix_web::Error> {
    let host = req.connection_info().host().to_string();
    let domain = host.split(':').next().unwrap_or(&host).to_lowercase();

    // Check if this domain is assigned to an application reverse proxy route
    if let Ok(Some(route)) = proxy_db.get_route_by_domain(&domain) {
        if is_websocket_request(&req) && route.websocket_support {
            return proxy_websocket(req, body, &route.target_url, None).await;
        } else {
            return proxy_http(req, body, &route.target_url, None).await;
        }
    }

    // Otherwise, serve static files from embedded WADM bundle
    Ok(serve_embedded_asset(req.uri().path()))
}

// ============================================================================
// REST API Handlers for /api/proxy/routes
// ============================================================================

pub async fn list_proxy_routes(
    user: AuthenticatedUser,
    proxy_db: web::Data<Arc<ProxyDatabase>>,
) -> Result<HttpResponse, AppError> {
    user.require_operator()?;
    let routes = proxy_db
        .list_routes()
        .map_err(|e| AppError::ExecutionFailed(format!("Database query failed: {}", e)))?;
    Ok(HttpResponse::Ok().json(routes))
}

pub async fn create_proxy_route(
    admin: RequireAdmin,
    audit: web::Data<Arc<AuditLogger>>,
    proxy_db: web::Data<Arc<ProxyDatabase>>,
    body: web::Json<ProxyRoute>,
) -> Result<HttpResponse, AppError> {
    let route = body.into_inner();
    proxy_db
        .create_or_update_route(&route)
        .map_err(|e| AppError::ExecutionFailed(format!("Database insert failed: {}", e)))?;

    audit.log(
        &admin.0.username,
        admin.0.role.as_str(),
        "PROXY_ROUTE_CREATE",
        Some(&route.app_id),
        &admin.0.client_ip,
        "SUCCESS",
        Some(&format!(
            "Route configured: Target: {}, Domain: {:?}, Prefix: {:?}",
            route.target_url, route.domain, route.path_prefix
        )),
    );

    Ok(HttpResponse::Created().json(route))
}

pub async fn delete_proxy_route(
    admin: RequireAdmin,
    audit: web::Data<Arc<AuditLogger>>,
    proxy_db: web::Data<Arc<ProxyDatabase>>,
    path: web::Path<String>,
) -> Result<HttpResponse, AppError> {
    let app_id = path.into_inner();
    let deleted = proxy_db
        .delete_route_by_app_id(&app_id)
        .map_err(|e| AppError::ExecutionFailed(format!("Database delete failed: {}", e)))?;

    if deleted {
        audit.log(
            &admin.0.username,
            admin.0.role.as_str(),
            "PROXY_ROUTE_DELETE",
            Some(&app_id),
            &admin.0.client_ip,
            "SUCCESS",
            None,
        );
        Ok(HttpResponse::Ok().json(format!("Proxy route for '{}' deleted", app_id)))
    } else {
        Ok(HttpResponse::NotFound().json("Proxy route not found"))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_embedded_assets_exist() {
        assert!(Asset::get("index.html").is_some());
    }

    #[test]
    fn test_serve_embedded_asset_root_and_spa() {
        let resp_root = serve_embedded_asset("/");
        assert_eq!(resp_root.status(), actix_web::http::StatusCode::OK);

        let resp_spa = serve_embedded_asset("/dashboard");
        assert_eq!(resp_spa.status(), actix_web::http::StatusCode::OK);

        let resp_fallback = serve_embedded_asset("/non_existent_random_path_123");
        assert_eq!(resp_fallback.status(), actix_web::http::StatusCode::OK);
    }
}
