use actix_web::{web, HttpResponse, Responder};
use std::sync::Arc;

use crate::api::jobs::{JobActionResponse, JobManager, JobTaskPayload};
use crate::audit::AuditLogger;
use crate::auth::RequireAdmin;
use crate::drivers::error::AppError;
use crate::plugins::{installer, PluginManager, PluginStore, PluginStoreItemView};

pub async fn list_plugins(manager: web::Data<Arc<PluginManager>>) -> impl Responder {
    let plugins = manager.list_plugins().await;
    HttpResponse::Ok().json(plugins)
}

pub async fn get_plugin(
    manager: web::Data<Arc<PluginManager>>,
    path: web::Path<String>,
) -> Result<HttpResponse, AppError> {
    let id = path.into_inner();
    let info = manager.get_plugin(&id).await?;
    Ok(HttpResponse::Ok().json(info))
}

pub async fn enable_plugin(
    user: crate::auth::AuthenticatedUser,
    audit: web::Data<std::sync::Arc<crate::audit::AuditLogger>>,
    manager: web::Data<Arc<PluginManager>>,
    path: web::Path<String>,
) -> Result<HttpResponse, AppError> {
    let id = path.into_inner();
    if user.require_operator().is_err() {
        audit.log_denied(
            &user,
            "PLUGIN_ENABLE",
            Some(&id),
            Some("Requires Operator or Admin role"),
        );
        return Err(AppError::Forbidden("Insufficient permissions".to_string()));
    }

    match manager.start_plugin(&id).await {
        Ok(info) => {
            audit.log(
                &user.username,
                user.role.as_str(),
                "PLUGIN_ENABLE",
                Some(&id),
                &user.client_ip,
                "SUCCESS",
                None,
            );
            Ok(HttpResponse::Ok().json(info))
        }
        Err(e) => {
            audit.log(
                &user.username,
                user.role.as_str(),
                "PLUGIN_ENABLE",
                Some(&id),
                &user.client_ip,
                "FAILED",
                Some(&e.to_string()),
            );
            Err(e)
        }
    }
}

pub async fn disable_plugin(
    user: crate::auth::AuthenticatedUser,
    audit: web::Data<std::sync::Arc<crate::audit::AuditLogger>>,
    manager: web::Data<Arc<PluginManager>>,
    path: web::Path<String>,
) -> Result<HttpResponse, AppError> {
    let id = path.into_inner();
    if user.require_operator().is_err() {
        audit.log_denied(
            &user,
            "PLUGIN_DISABLE",
            Some(&id),
            Some("Requires Operator or Admin role"),
        );
        return Err(AppError::Forbidden("Insufficient permissions".to_string()));
    }

    match manager.stop_plugin(&id).await {
        Ok(info) => {
            audit.log(
                &user.username,
                user.role.as_str(),
                "PLUGIN_DISABLE",
                Some(&id),
                &user.client_ip,
                "SUCCESS",
                None,
            );
            Ok(HttpResponse::Ok().json(info))
        }
        Err(e) => {
            audit.log(
                &user.username,
                user.role.as_str(),
                "PLUGIN_DISABLE",
                Some(&id),
                &user.client_ip,
                "FAILED",
                Some(&e.to_string()),
            );
            Err(e)
        }
    }
}

pub async fn forward_rpc(
    user: crate::auth::AuthenticatedUser,
    audit: web::Data<std::sync::Arc<crate::audit::AuditLogger>>,
    manager: web::Data<Arc<PluginManager>>,
    path: web::Path<String>,
    payload: web::Json<serde_json::Value>,
) -> Result<HttpResponse, AppError> {
    let id = path.into_inner();
    if user.require_operator().is_err() {
        audit.log_denied(
            &user,
            "PLUGIN_RPC",
            Some(&id),
            Some("Requires Operator or Admin role"),
        );
        return Err(AppError::Forbidden("Insufficient permissions".to_string()));
    }

    let method = payload
        .get("method")
        .and_then(|m| m.as_str())
        .unwrap_or("unknown")
        .to_string();

    match manager.forward_rpc(&id, &payload.into_inner()).await {
        Ok(response) => {
            audit.log(
                &user.username,
                user.role.as_str(),
                "PLUGIN_RPC",
                Some(&id),
                &user.client_ip,
                "SUCCESS",
                Some(&format!("Method: {}", method)),
            );
            Ok(HttpResponse::Ok().json(response))
        }
        Err(e) => {
            audit.log(
                &user.username,
                user.role.as_str(),
                "PLUGIN_RPC",
                Some(&id),
                &user.client_ip,
                "FAILED",
                Some(&e.to_string()),
            );
            Err(e)
        }
    }
}

pub async fn ping_plugin(
    user: crate::auth::AuthenticatedUser,
    manager: web::Data<Arc<PluginManager>>,
    path: web::Path<String>,
) -> Result<HttpResponse, AppError> {
    user.require_operator()?;
    let id = path.into_inner();
    let response = manager.ping_plugin(&id).await?;
    Ok(HttpResponse::Ok().json(response))
}

pub async fn list_store_plugins(
    manager: web::Data<Arc<PluginManager>>,
    store: web::Data<Arc<PluginStore>>,
) -> Result<HttpResponse, AppError> {
    let catalog = store.get_catalog(false).await?;
    let installed = manager.list_plugins().await;

    let views: Vec<PluginStoreItemView> = catalog
        .into_iter()
        .map(|item| {
            let installed_plugin = installed.iter().find(|p| p.manifest.id == item.id);
            let is_installed = installed_plugin.is_some();
            let installed_version = installed_plugin.map(|p| p.manifest.version.clone());
            let has_update = if let Some(ref inst_ver) = installed_version {
                inst_ver != &item.version
            } else {
                false
            };

            PluginStoreItemView {
                item,
                is_installed,
                installed_version,
                has_update,
            }
        })
        .collect();

    Ok(HttpResponse::Ok().json(views))
}

pub async fn refresh_store_catalog(
    admin: RequireAdmin,
    audit: web::Data<Arc<AuditLogger>>,
    store: web::Data<Arc<PluginStore>>,
) -> Result<HttpResponse, AppError> {
    let catalog = store.get_catalog(true).await?;
    audit.log(
        &admin.0.username,
        admin.0.role.as_str(),
        "PLUGIN_STORE_REFRESH",
        None,
        &admin.0.client_ip,
        "SUCCESS",
        Some(&format!(
            "Refreshed store catalog ({} plugins)",
            catalog.len()
        )),
    );
    Ok(HttpResponse::Ok().json(catalog))
}

pub async fn install_store_plugin(
    admin: RequireAdmin,
    audit: web::Data<Arc<AuditLogger>>,
    job_manager: web::Data<Arc<JobManager>>,
    manager: web::Data<Arc<PluginManager>>,
    store: web::Data<Arc<PluginStore>>,
    path: web::Path<String>,
) -> Result<HttpResponse, AppError> {
    let id = path.into_inner();
    let item = store
        .get_item(&id)
        .await?
        .ok_or_else(|| AppError::NotFound(format!("Plugin '{}' not found in store catalog", id)))?;

    let target_triple = installer::get_target_triple();
    let asset = item
        .downloads
        .get(target_triple)
        .or_else(|| item.downloads.get("x86_64-unknown-linux-gnu"))
        .ok_or_else(|| {
            AppError::NotFound(format!(
                "No compatible binary found for platform '{}'",
                target_triple
            ))
        })?;

    let payload = JobTaskPayload::PluginInstall {
        plugin_id: id.clone(),
        download_url: asset.url.clone(),
        expected_sha256: asset.sha256.clone(),
        manager: manager.get_ref().clone(),
    };

    let job_id = job_manager
        .enqueue_job(&format!("install_plugin_{}", id), payload)
        .await
        .map_err(|e| AppError::ExecutionFailed(format!("Failed to enqueue install job: {}", e)))?;

    audit.log(
        &admin.0.username,
        admin.0.role.as_str(),
        "PLUGIN_INSTALL",
        Some(&id),
        &admin.0.client_ip,
        "ACCEPTED",
        Some(&format!("Enqueued job {}", job_id)),
    );

    Ok(HttpResponse::Accepted().json(JobActionResponse {
        job_id,
        status: "accepted".to_string(),
        message: format!("Installation of plugin '{}' queued", id),
    }))
}

pub async fn update_store_plugin(
    admin: RequireAdmin,
    audit: web::Data<Arc<AuditLogger>>,
    job_manager: web::Data<Arc<JobManager>>,
    manager: web::Data<Arc<PluginManager>>,
    store: web::Data<Arc<PluginStore>>,
    path: web::Path<String>,
) -> Result<HttpResponse, AppError> {
    let id = path.into_inner();
    let item = store
        .get_item(&id)
        .await?
        .ok_or_else(|| AppError::NotFound(format!("Plugin '{}' not found in store catalog", id)))?;

    let target_triple = installer::get_target_triple();
    let asset = item
        .downloads
        .get(target_triple)
        .or_else(|| item.downloads.get("x86_64-unknown-linux-gnu"))
        .ok_or_else(|| {
            AppError::NotFound(format!(
                "No compatible binary found for platform '{}'",
                target_triple
            ))
        })?;

    let payload = JobTaskPayload::PluginInstall {
        plugin_id: id.clone(),
        download_url: asset.url.clone(),
        expected_sha256: asset.sha256.clone(),
        manager: manager.get_ref().clone(),
    };

    let job_id = job_manager
        .enqueue_job(&format!("update_plugin_{}", id), payload)
        .await
        .map_err(|e| AppError::ExecutionFailed(format!("Failed to enqueue update job: {}", e)))?;

    audit.log(
        &admin.0.username,
        admin.0.role.as_str(),
        "PLUGIN_UPDATE",
        Some(&id),
        &admin.0.client_ip,
        "ACCEPTED",
        Some(&format!("Enqueued job {}", job_id)),
    );

    Ok(HttpResponse::Accepted().json(JobActionResponse {
        job_id,
        status: "accepted".to_string(),
        message: format!("Update of plugin '{}' queued", id),
    }))
}

pub async fn uninstall_store_plugin(
    admin: RequireAdmin,
    audit: web::Data<Arc<AuditLogger>>,
    job_manager: web::Data<Arc<JobManager>>,
    manager: web::Data<Arc<PluginManager>>,
    path: web::Path<String>,
) -> Result<HttpResponse, AppError> {
    let id = path.into_inner();

    let payload = JobTaskPayload::PluginUninstall {
        plugin_id: id.clone(),
        manager: manager.get_ref().clone(),
    };

    let job_id = job_manager
        .enqueue_job(&format!("uninstall_plugin_{}", id), payload)
        .await
        .map_err(|e| {
            AppError::ExecutionFailed(format!("Failed to enqueue uninstall job: {}", e))
        })?;

    audit.log(
        &admin.0.username,
        admin.0.role.as_str(),
        "PLUGIN_UNINSTALL",
        Some(&id),
        &admin.0.client_ip,
        "ACCEPTED",
        Some(&format!("Enqueued job {}", job_id)),
    );

    Ok(HttpResponse::Accepted().json(JobActionResponse {
        job_id,
        status: "accepted".to_string(),
        message: format!("Uninstallation of plugin '{}' queued", id),
    }))
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;

    #[actix_web::test]
    async fn test_api_plugins_empty_list() {
        let temp_dir = std::env::temp_dir().join(format!("wadm_api_test_{}", uuid::Uuid::new_v4()));
        let plugins_dir = temp_dir.join("plugins");
        let sockets_dir = temp_dir.join("sockets");
        fs::create_dir_all(&plugins_dir).unwrap();
        fs::create_dir_all(&sockets_dir).unwrap();

        let manager = Arc::new(PluginManager::new(plugins_dir, sockets_dir));
        let manager_data = web::Data::new(manager);

        let resp = list_plugins(manager_data.clone())
            .await
            .respond_to(&actix_web::test::TestRequest::default().to_http_request());
        assert_eq!(resp.status(), actix_web::http::StatusCode::OK);

        let not_found = get_plugin(manager_data, web::Path::from("non-existent".to_string())).await;
        assert!(not_found.is_err());

        let _ = fs::remove_dir_all(&temp_dir);
    }

    #[actix_web::test]
    async fn test_api_store_plugins_list() {
        let temp_dir =
            std::env::temp_dir().join(format!("wadm_api_store_test_{}", uuid::Uuid::new_v4()));
        let plugins_dir = temp_dir.join("plugins");
        let sockets_dir = temp_dir.join("sockets");
        fs::create_dir_all(&plugins_dir).unwrap();
        fs::create_dir_all(&sockets_dir).unwrap();

        let manager = Arc::new(PluginManager::new(plugins_dir, sockets_dir));
        let manager_data = web::Data::new(manager);
        let store = Arc::new(PluginStore::with_catalog_url(
            "http://127.0.0.1:9/unreachable.json".to_string(),
        ));
        let store_data = web::Data::new(store);

        let resp = list_store_plugins(manager_data, store_data)
            .await
            .expect("list_store_plugins failed");
        assert_eq!(resp.status(), actix_web::http::StatusCode::OK);

        let _ = fs::remove_dir_all(&temp_dir);
    }
}
