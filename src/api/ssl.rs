use actix_web::{web, HttpResponse, Responder};
use chrono::Utc;
use serde::{Deserialize, Serialize};
use std::fs::File;
use std::io::Write;
use std::sync::Arc;

use crate::api::jobs::{JobManager, JobTaskPayload};
use crate::audit::AuditLogger;
use crate::auth::RequireAdmin;
use crate::ssl::db::{CertMetaUpdate, SslConfig, SslDatabase};
use crate::ssl::generator::generate_self_signed;
use crate::ssl::tls::validate_and_parse_pem;
use crate::ssl::{get_cert_paths, get_certs_dir, AcmeChallengeStore};

#[derive(Debug, Serialize, Deserialize)]
pub struct GenerateSelfSignedRequest {
    pub domain: Option<String>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct CustomCertRequest {
    pub cert_pem: String,
    pub key_pem: String,
    pub domain: Option<String>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct LetsEncryptRequest {
    pub domain: String,
    pub email: String,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct ToggleSslRequest {
    pub enabled: bool,
    pub force_https: bool,
    pub auto_renew: Option<bool>,
}

#[derive(Debug, Serialize)]
pub struct SslStatusResponse {
    pub config: SslConfig,
    pub cert_exists: bool,
    pub days_left: Option<i64>,
    pub is_expired: bool,
}

pub async fn get_ssl_config(ssl_db: web::Data<Arc<SslDatabase>>) -> impl Responder {
    match ssl_db.get_config() {
        Ok(config) => {
            let (cert_path, _) = get_cert_paths();
            let cert_exists = cert_path.exists();

            let (days_left, is_expired) = if let Some(expires_at) = config.expires_at {
                let now = Utc::now().timestamp();
                let diff_sec = expires_at - now;
                let days = diff_sec / 86400;
                (Some(days), diff_sec <= 0)
            } else {
                (None, false)
            };

            HttpResponse::Ok().json(SslStatusResponse {
                config,
                cert_exists,
                days_left,
                is_expired,
            })
        }
        Err(e) => HttpResponse::InternalServerError().json(serde_json::json!({
            "error": format!("Failed to read SSL configuration: {}", e)
        })),
    }
}

pub async fn generate_self_signed_cert(
    admin: RequireAdmin,
    ssl_db: web::Data<Arc<SslDatabase>>,
    audit: web::Data<Arc<AuditLogger>>,
    body: web::Json<GenerateSelfSignedRequest>,
) -> impl Responder {
    let domain = body.domain.as_deref();

    match generate_self_signed(domain) {
        Ok(generated) => {
            let cert_str = generated.cert_path.to_string_lossy().to_string();
            let key_str = generated.key_path.to_string_lossy().to_string();

            if let Err(e) = ssl_db.update_certificate(&CertMetaUpdate {
                mode: "self_signed",
                domain,
                email: None,
                cert_path: &cert_str,
                key_path: &key_str,
                issuer: &generated.issuer,
                expires_at: Some(generated.expires_at),
            }) {
                return HttpResponse::InternalServerError().json(serde_json::json!({
                    "error": format!("Failed to update database: {}", e)
                }));
            }

            let _ = ssl_db.set_enabled(true, false);

            audit.log(
                &admin.0.username,
                admin.0.role.as_str(),
                "SSL_GENERATE_SELF_SIGNED",
                domain,
                &admin.0.client_ip,
                "SUCCESS",
                Some("Generated 10-year self-signed SSL certificate"),
            );

            HttpResponse::Ok().json(serde_json::json!({
                "status": "success",
                "message": "Self-signed certificate generated and applied successfully",
                "expires_at": generated.expires_at,
                "issuer": generated.issuer,
            }))
        }
        Err(e) => {
            audit.log(
                &admin.0.username,
                admin.0.role.as_str(),
                "SSL_GENERATE_SELF_SIGNED",
                domain,
                &admin.0.client_ip,
                "FAILURE",
                Some(&e),
            );
            HttpResponse::BadRequest().json(serde_json::json!({
                "error": e
            }))
        }
    }
}

pub async fn upload_custom_cert(
    admin: RequireAdmin,
    ssl_db: web::Data<Arc<SslDatabase>>,
    audit: web::Data<Arc<AuditLogger>>,
    body: web::Json<CustomCertRequest>,
) -> impl Responder {
    if body.cert_pem.trim().is_empty() || body.key_pem.trim().is_empty() {
        return HttpResponse::BadRequest().json(serde_json::json!({
            "error": "Certificate and Private Key PEM contents cannot be empty"
        }));
    }

    // Validate PEM format and ensure key matches certificate
    if let Err(e) = validate_and_parse_pem(&body.cert_pem, &body.key_pem) {
        audit.log(
            &admin.0.username,
            admin.0.role.as_str(),
            "SSL_UPLOAD_CUSTOM",
            body.domain.as_deref(),
            &admin.0.client_ip,
            "FAILURE",
            Some(&e),
        );
        return HttpResponse::BadRequest().json(serde_json::json!({
            "error": format!("Certificate validation failed: {}", e)
        }));
    }

    let certs_dir = get_certs_dir();
    let _ = std::fs::create_dir_all(&certs_dir);
    let (cert_path, key_path) = get_cert_paths();

    if let Err(e) = File::create(&cert_path).and_then(|mut f| f.write_all(body.cert_pem.as_bytes()))
    {
        return HttpResponse::InternalServerError().json(serde_json::json!({
            "error": format!("Failed to write certificate file: {}", e)
        }));
    }

    if let Err(e) = File::create(&key_path).and_then(|mut f| f.write_all(body.key_pem.as_bytes())) {
        return HttpResponse::InternalServerError().json(serde_json::json!({
            "error": format!("Failed to write private key file: {}", e)
        }));
    }

    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        let _ = std::fs::set_permissions(&key_path, std::fs::Permissions::from_mode(0o600));
        let _ = std::fs::set_permissions(&cert_path, std::fs::Permissions::from_mode(0o644));
    }

    let cert_str = cert_path.to_string_lossy().to_string();
    let key_str = key_path.to_string_lossy().to_string();

    if let Err(e) = ssl_db.update_certificate(&CertMetaUpdate {
        mode: "custom_cert",
        domain: body.domain.as_deref(),
        email: None,
        cert_path: &cert_str,
        key_path: &key_str,
        issuer: "Custom CA / Provided",
        expires_at: None,
    }) {
        return HttpResponse::InternalServerError().json(serde_json::json!({
            "error": format!("Failed to update database: {}", e)
        }));
    }

    let _ = ssl_db.set_enabled(true, false);

    audit.log(
        &admin.0.username,
        admin.0.role.as_str(),
        "SSL_UPLOAD_CUSTOM",
        body.domain.as_deref(),
        &admin.0.client_ip,
        "SUCCESS",
        Some("Uploaded and validated custom SSL certificate"),
    );

    HttpResponse::Ok().json(serde_json::json!({
        "status": "success",
        "message": "Custom certificate and private key saved and activated successfully"
    }))
}

pub async fn request_letsencrypt_cert(
    admin: RequireAdmin,
    job_manager: web::Data<Arc<JobManager>>,
    audit: web::Data<Arc<AuditLogger>>,
    body: web::Json<LetsEncryptRequest>,
) -> impl Responder {
    let domain = body.domain.trim().to_string();
    let email = body.email.trim().to_string();

    if domain.is_empty() {
        return HttpResponse::BadRequest().json(serde_json::json!({
            "error": "Domain name is required"
        }));
    }

    if email.is_empty() || !email.contains('@') {
        return HttpResponse::BadRequest().json(serde_json::json!({
            "error": "A valid email address is required for ACME registration"
        }));
    }

    match job_manager
        .enqueue_job(
            "acme_certificate_issue",
            JobTaskPayload::AcmeIssueCertificate {
                domain: domain.clone(),
                email: email.clone(),
            },
        )
        .await
    {
        Ok(job_id) => {
            audit.log(
                &admin.0.username,
                admin.0.role.as_str(),
                "SSL_REQUEST_LETSENCRYPT",
                Some(&domain),
                &admin.0.client_ip,
                "SUCCESS",
                Some(&format!("Enqueued Let's Encrypt issuance job {}", job_id)),
            );

            HttpResponse::Ok().json(serde_json::json!({
                "status": "pending",
                "job_id": job_id,
                "message": "Let's Encrypt certificate issuance queued. Monitor progress via Job Runner."
            }))
        }
        Err(e) => {
            audit.log(
                &admin.0.username,
                admin.0.role.as_str(),
                "SSL_REQUEST_LETSENCRYPT",
                Some(&domain),
                &admin.0.client_ip,
                "FAILURE",
                Some(&e),
            );

            HttpResponse::InternalServerError().json(serde_json::json!({
                "error": format!("Failed to queue job: {}", e)
            }))
        }
    }
}

pub async fn toggle_ssl(
    admin: RequireAdmin,
    ssl_db: web::Data<Arc<SslDatabase>>,
    audit: web::Data<Arc<AuditLogger>>,
    body: web::Json<ToggleSslRequest>,
) -> impl Responder {
    let (cert_path, key_path) = get_cert_paths();
    if body.enabled && (!cert_path.exists() || !key_path.exists()) {
        return HttpResponse::BadRequest().json(serde_json::json!({
            "error": "Cannot enable SSL: no certificates have been generated or uploaded yet"
        }));
    }

    if let Some(ar) = body.auto_renew {
        let _ = ssl_db.set_auto_renew(ar);
    }

    match ssl_db.set_enabled(body.enabled, body.force_https) {
        Ok(_) => {
            audit.log(
                &admin.0.username,
                admin.0.role.as_str(),
                "SSL_TOGGLE",
                None,
                &admin.0.client_ip,
                "SUCCESS",
                Some(&format!(
                    "SSL enabled: {}, Force HTTPS: {}, Auto Renew: {:?}",
                    body.enabled, body.force_https, body.auto_renew
                )),
            );

            HttpResponse::Ok().json(serde_json::json!({
                "status": "success",
                "enabled": body.enabled,
                "force_https": body.force_https,
                "message": if body.enabled {
                    "SSL enabled. Server is listening on HTTPS."
                } else {
                    "SSL disabled. Server is running in HTTP mode."
                }
            }))
        }
        Err(e) => HttpResponse::InternalServerError().json(serde_json::json!({
            "error": format!("Failed to update SSL state: {}", e)
        })),
    }
}

pub async fn acme_challenge_handler(
    token: web::Path<String>,
    challenge_store: web::Data<AcmeChallengeStore>,
) -> impl Responder {
    let token_str = token.into_inner();
    let store = challenge_store.read().await;

    if let Some(key_auth) = store.get(&token_str) {
        HttpResponse::Ok()
            .content_type("text/plain")
            .body(key_auth.clone())
    } else {
        HttpResponse::NotFound().body("ACME challenge token not found")
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use actix_web::test;

    #[actix_web::test]
    async fn test_acme_challenge_handler() {
        let store = crate::ssl::create_challenge_store();
        store.write().await.insert(
            "test-token-123".to_string(),
            "test-token-123.auth-key".to_string(),
        );

        let store_data = web::Data::new(store);

        // Found token
        let res = acme_challenge_handler(
            web::Path::from("test-token-123".to_string()),
            store_data.clone(),
        )
        .await
        .respond_to(&test::TestRequest::default().to_http_request());

        assert_eq!(res.status(), actix_web::http::StatusCode::OK);

        // Missing token
        let res_missing = acme_challenge_handler(
            web::Path::from("non-existent-token".to_string()),
            store_data,
        )
        .await
        .respond_to(&test::TestRequest::default().to_http_request());

        assert_eq!(res_missing.status(), actix_web::http::StatusCode::NOT_FOUND);
    }

    #[actix_web::test]
    async fn test_get_ssl_config_handler() {
        let db = Arc::new(SslDatabase::new_in_memory().unwrap());
        let db_data = web::Data::new(db);

        let res = get_ssl_config(db_data)
            .await
            .respond_to(&test::TestRequest::default().to_http_request());

        assert_eq!(res.status(), actix_web::http::StatusCode::OK);
    }
}
