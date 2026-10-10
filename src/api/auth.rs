use actix_web::cookie::{time::Duration as CookieDuration, Cookie, SameSite};
use actix_web::{web, HttpRequest, HttpResponse, Responder};
use argon2::{
    password_hash::{rand_core::OsRng, PasswordHash, PasswordHasher, PasswordVerifier, SaltString},
    Argon2,
};
use chrono::{Duration, Utc};
use jsonwebtoken::{encode, EncodingKey, Header};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::fs;
use std::sync::Mutex;
use std::time::{Duration as StdDuration, Instant};
use totp_rs::{Algorithm, Secret, TOTP};

const AUTH_FILE: &str = "wadm-auth.json";
use once_cell::sync::Lazy;

pub fn get_data_dir() -> std::path::PathBuf {
    if let Ok(dir) = std::env::var("WADM_DATA_DIR") {
        let p = std::path::PathBuf::from(dir);
        if p.exists() || fs::create_dir_all(&p).is_ok() {
            return p;
        }
    }
    std::env::current_dir().unwrap_or_default()
}

pub static JWT_SECRET: Lazy<Vec<u8>> = Lazy::new(|| {
    let path = get_data_dir().join(".wadm_jwt_secret");
    if path.exists() {
        if let Ok(content) = fs::read(&path) {
            return content;
        }
    }
    use rand::rngs::OsRng;
    use rand::RngCore;
    let mut key = vec![0u8; 32];
    OsRng.fill_bytes(&mut key);
    let _ = fs::write(&path, &key);
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        let _ = fs::set_permissions(&path, fs::Permissions::from_mode(0o600));
    }
    key
});

pub struct LoginRateLimiter {
    attempts: Mutex<HashMap<String, Vec<Instant>>>,
}

impl LoginRateLimiter {
    pub fn new() -> Self {
        Self {
            attempts: Mutex::new(HashMap::new()),
        }
    }

    pub fn is_rate_limited(&self, ip: &str) -> bool {
        let mut attempts = self.attempts.lock().unwrap_or_else(|e| e.into_inner());
        let now = Instant::now();
        let window = StdDuration::from_secs(60);

        // Periodically prune expired IP records to prevent unbounded memory growth
        if attempts.len() > 50 {
            attempts.retain(|_, v| {
                v.retain(|&t| now.duration_since(t) < window);
                !v.is_empty()
            });
        }

        let entry = attempts.entry(ip.to_string()).or_default();
        entry.retain(|&t| now.duration_since(t) < window);

        if entry.len() >= 5 {
            true
        } else {
            entry.push(now);
            false
        }
    }

    pub fn reset(&self, ip: &str) {
        let mut attempts = self.attempts.lock().unwrap_or_else(|e| e.into_inner());
        attempts.remove(ip);
    }
}

pub static LOGIN_RATE_LIMITER: Lazy<LoginRateLimiter> = Lazy::new(LoginRateLimiter::new);

#[derive(Serialize, Deserialize, Clone)]
pub struct AuthStore {
    pub password_hash: String,
    pub totp_secret: String,
    pub setup_complete: bool,
}

pub use crate::auth::{Claims, UserRole};

#[derive(Deserialize)]
pub struct LoginRequest {
    #[serde(default)]
    pub username: Option<String>,
    pub password: String,
    #[serde(default)]
    pub code: Option<String>,
}

#[derive(Deserialize)]
pub struct SetupRequest {
    password: String,
    code: String,
    secret: String,
}

#[derive(Serialize)]
pub struct LoginResponse {
    pub token: String,
    pub username: String,
    pub role: String,
}

pub fn build_auth_cookie<'a>(req: &HttpRequest, token: &'a str) -> Cookie<'a> {
    let host = req.connection_info().host().to_string();
    let is_https = req.connection_info().scheme() == "https"
        || req
            .headers()
            .get("x-forwarded-proto")
            .and_then(|v| v.to_str().ok())
            == Some("https");
    let is_localhost =
        host.starts_with("localhost") || host.starts_with("127.0.0.1") || host.starts_with("[::1]");
    let secure = is_https
        || is_localhost
        || std::env::var("WADM_SECURE_COOKIE")
            .map(|v| v == "true" || v == "1")
            .unwrap_or(true);

    Cookie::build("wadm_token", token)
        .path("/")
        .http_only(true)
        .same_site(SameSite::Strict)
        .max_age(CookieDuration::seconds(86400))
        .secure(secure)
        .finish()
}

pub fn build_logout_cookie<'a>() -> Cookie<'a> {
    Cookie::build("wadm_token", "")
        .path("/")
        .http_only(true)
        .same_site(SameSite::Strict)
        .max_age(CookieDuration::ZERO)
        .finish()
}

#[derive(Serialize)]
struct SetupInitResponse {
    secret: String,
    qr: String,
}

#[derive(Serialize)]
struct AuthStatus {
    setup_required: bool,
}

fn get_auth_file_path() -> std::path::PathBuf {
    let filename = AUTH_FILE;

    // 0. Try explicitly configured data directory
    if let Ok(dir) = std::env::var("WADM_DATA_DIR") {
        let p = std::path::PathBuf::from(dir);
        let _ = fs::create_dir_all(&p);
        return p.join(filename);
    }

    // 1. Try current working directory
    let cwd_path = std::env::current_dir().unwrap_or_default().join(filename);
    if cwd_path.exists() {
        log::info!("Found auth file in CWD: {:?}", cwd_path);
        return cwd_path;
    }

    // 2. Try executable directory
    if let Ok(exe_path) = std::env::current_exe() {
        if let Some(exe_dir) = exe_path.parent() {
            let exe_dir_path = exe_dir.join(filename);
            if exe_dir_path.exists() {
                log::info!("Found auth file in executable dir: {:?}", exe_dir_path);
                return exe_dir_path;
            }
            log::info!(
                "Auth file not found. Defaulting path to executable dir: {:?}",
                exe_dir_path
            );
            return exe_dir_path;
        }
    }

    log::info!(
        "Could not determine executable dir. Defaulting to CWD: {:?}",
        cwd_path
    );
    cwd_path
}

pub fn load_auth_store() -> Option<AuthStore> {
    let path = get_auth_file_path();
    log::info!("Attempting to load auth store from: {:?}", path);
    match fs::read_to_string(&path) {
        Ok(content) => match serde_json::from_str(&content) {
            Ok(store) => {
                log::info!("Successfully loaded auth store.");
                Some(store)
            }
            Err(e) => {
                log::error!(
                    "CRITICAL: Failed to parse auth store: {}. Setup required / recovery mode.",
                    e
                );
                None
            }
        },
        Err(e) => {
            if e.kind() == std::io::ErrorKind::NotFound {
                log::warn!(
                    "Auth store file not found at {:?}. Assuming first run/setup required.",
                    path
                );
                None
            } else {
                log::error!(
                    "CRITICAL: Failed to read auth store file: {}. Setup required / recovery mode.",
                    e
                );
                None
            }
        }
    }
}

pub async fn get_auth_status(data: web::Data<Mutex<Option<AuthStore>>>) -> impl Responder {
    let store = data.lock().unwrap_or_else(|e| e.into_inner());

    HttpResponse::Ok().json(AuthStatus {
        setup_required: store.is_none(),
    })
}

pub async fn init_setup(data: web::Data<Mutex<Option<AuthStore>>>) -> impl Responder {
    let store_guard = data.lock().unwrap_or_else(|e| e.into_inner());
    if store_guard.is_some() {
        return HttpResponse::BadRequest().json("Setup already complete");
    }

    let secret = Secret::generate_secret();
    let secret_bytes = match secret.to_bytes() {
        Ok(b) => b,
        Err(_) => {
            return HttpResponse::InternalServerError().json("Failed to generate secret bytes")
        }
    };

    let totp = match TOTP::new(
        Algorithm::SHA256,
        6,
        1,
        30,
        secret_bytes,
        Some("WADM".to_string()),
        "admin@wadm".to_string(),
    ) {
        Ok(t) => t,
        Err(e) => {
            log::error!("Failed to create TOTP instance: {}", e);
            return HttpResponse::InternalServerError().json("Failed to create TOTP instance");
        }
    };

    let qr = match totp.get_qr_base64() {
        Ok(q) => q,
        Err(e) => {
            log::error!("Failed to generate QR code: {}", e);
            return HttpResponse::InternalServerError().json("Failed to generate QR code");
        }
    };

    HttpResponse::Ok().json(SetupInitResponse {
        secret: secret.to_encoded().to_string(),
        qr,
    })
}

pub async fn confirm_setup(
    req: HttpRequest,
    body: web::Json<SetupRequest>,
    data: web::Data<Mutex<Option<AuthStore>>>,
    user_db: web::Data<std::sync::Arc<crate::auth::UserDatabase>>,
    audit: web::Data<std::sync::Arc<crate::audit::AuditLogger>>,
) -> impl Responder {
    let mut store_guard = data.lock().unwrap_or_else(|e| e.into_inner());

    if store_guard.is_some() {
        return HttpResponse::BadRequest().json("Setup already complete");
    }

    let secret_bytes = match Secret::Encoded(body.secret.clone()).to_bytes() {
        Ok(b) => b,
        Err(_) => return HttpResponse::BadRequest().json("Invalid secret format"),
    };

    // Try SHA256 first, fallback to SHA1
    let valid_code = if let Ok(totp) = TOTP::new(
        Algorithm::SHA256,
        6,
        1,
        30,
        secret_bytes.clone(),
        None,
        "".to_string(),
    ) {
        totp.check_current(&body.code).unwrap_or(false)
    } else {
        false
    } || if let Ok(totp) = TOTP::new(
        Algorithm::SHA1,
        6,
        1,
        30,
        secret_bytes,
        None,
        "".to_string(),
    ) {
        totp.check_current(&body.code).unwrap_or(false)
    } else {
        false
    };

    if !valid_code {
        return HttpResponse::BadRequest().json("Invalid 2FA code");
    }

    let salt = SaltString::generate(&mut OsRng);
    let argon2 = Argon2::default();
    let password_hash = match argon2.hash_password(body.password.as_bytes(), &salt) {
        Ok(h) => h.to_string(),
        Err(e) => {
            log::error!("Password hashing failed: {}", e);
            return HttpResponse::InternalServerError().json("Password hashing failed");
        }
    };

    let new_store = AuthStore {
        password_hash,
        totp_secret: body.secret.clone(),
        setup_complete: true,
    };

    match serde_json::to_string(&new_store) {
        Ok(json) => {
            let path = get_auth_file_path();
            if let Err(e) = fs::write(&path, json) {
                log::error!("CRITICAL: Failed to write auth file to {:?}: {}", path, e);
                return HttpResponse::InternalServerError().json("Failed to save auth state");
            } else {
                #[cfg(unix)]
                {
                    use std::os::unix::fs::PermissionsExt;
                    let _ = fs::set_permissions(&path, fs::Permissions::from_mode(0o600));
                }
                log::info!("Successfully persisted auth state to {:?}", path);
            }
        }
        Err(e) => {
            log::error!("CRITICAL: Failed to serialize auth store: {}", e);
            return HttpResponse::InternalServerError().json("Failed to serialize auth state");
        }
    }

    *store_guard = Some(new_store);

    let _ = user_db.create_user("admin", &body.password, UserRole::Admin);
    audit.log(
        "admin",
        "admin",
        "SETUP_CONFIRM",
        None,
        "127.0.0.1",
        "SUCCESS",
        Some("Initial admin setup completed"),
    );

    let expiration = Utc::now()
        .checked_add_signed(Duration::hours(2))
        .expect("valid timestamp")
        .timestamp();

    let claims = Claims {
        sub: "admin".to_string(),
        role: UserRole::Admin,
        iat: Utc::now().timestamp() as usize,
        exp: expiration as usize,
        jti: Some(uuid::Uuid::new_v4().to_string()),
    };

    let token = encode(
        &Header::default(),
        &claims,
        &EncodingKey::from_secret(JWT_SECRET.as_slice()),
    )
    .unwrap_or_default();

    let cookie = build_auth_cookie(&req, &token);
    HttpResponse::Ok().cookie(cookie).json(LoginResponse {
        token,
        username: "admin".to_string(),
        role: UserRole::Admin.as_str().to_string(),
    })
}

pub async fn login(
    req: HttpRequest,
    body: web::Json<LoginRequest>,
    data: web::Data<Mutex<Option<AuthStore>>>,
    user_db: web::Data<std::sync::Arc<crate::auth::UserDatabase>>,
    audit: web::Data<std::sync::Arc<crate::audit::AuditLogger>>,
) -> impl Responder {
    let client_ip = req
        .connection_info()
        .realip_remote_addr()
        .unwrap_or("unknown")
        .to_string();

    if LOGIN_RATE_LIMITER.is_rate_limited(&client_ip) {
        log::warn!(
            "Rate limit exceeded for login attempts from IP: {}",
            client_ip
        );
        audit.log(
            body.username.as_deref().unwrap_or("admin"),
            "unknown",
            "LOGIN_RATE_LIMITED",
            None,
            &client_ip,
            "DENIED",
            Some("Too many login attempts"),
        );
        return HttpResponse::TooManyRequests()
            .json("Too many login attempts. Please wait 1 minute.");
    }

    let username = body
        .username
        .as_deref()
        .map(|s| s.trim())
        .filter(|s| !s.is_empty())
        .unwrap_or("admin");

    let is_admin_user = username == "admin";

    if is_admin_user {
        let store_guard = data.lock().unwrap_or_else(|e| e.into_inner());

        let store = match &*store_guard {
            Some(s) => s,
            None => {
                audit.log(
                    "admin",
                    "unknown",
                    "LOGIN_FAILED",
                    None,
                    &client_ip,
                    "DENIED",
                    Some("Setup required"),
                );
                return HttpResponse::BadRequest().json("Setup required");
            }
        };

        // Try user_db first, fallback to store
        let pwd_ok = match user_db.authenticate("admin", &body.password) {
            Ok(u) => u.role == UserRole::Admin,
            Err(_) => {
                let parsed_hash = match PasswordHash::new(&store.password_hash) {
                    Ok(h) => h,
                    Err(_) => {
                        return HttpResponse::InternalServerError().json("Invalid stored hash")
                    }
                };
                Argon2::default()
                    .verify_password(body.password.as_bytes(), &parsed_hash)
                    .is_ok()
            }
        };

        if !pwd_ok {
            audit.log(
                "admin",
                "unknown",
                "LOGIN_FAILED",
                None,
                &client_ip,
                "DENIED",
                Some("Invalid password"),
            );
            return HttpResponse::Unauthorized().json("Invalid credentials");
        }

        let secret_bytes = match Secret::Encoded(store.totp_secret.clone()).to_bytes() {
            Ok(b) => b,
            Err(_) => return HttpResponse::InternalServerError().json("Invalid secret format"),
        };

        let code_input = body.code.as_deref().unwrap_or("");
        // Check SHA256 first, then SHA1 for backward compatibility
        let valid_2fa = if let Ok(totp) = TOTP::new(
            Algorithm::SHA256,
            6,
            1,
            30,
            secret_bytes.clone(),
            None,
            "".to_string(),
        ) {
            totp.check_current(code_input).unwrap_or(false)
        } else {
            false
        } || if let Ok(totp) = TOTP::new(
            Algorithm::SHA1,
            6,
            1,
            30,
            secret_bytes,
            None,
            "".to_string(),
        ) {
            totp.check_current(code_input).unwrap_or(false)
        } else {
            false
        };

        if !valid_2fa {
            audit.log(
                "admin",
                "admin",
                "LOGIN_FAILED",
                None,
                &client_ip,
                "DENIED",
                Some("Invalid 2FA code"),
            );
            return HttpResponse::Unauthorized().json("Invalid 2FA code");
        }

        // Login successful: reset rate limiter
        LOGIN_RATE_LIMITER.reset(&client_ip);
        audit.log(
            "admin",
            "admin",
            "LOGIN_SUCCESS",
            None,
            &client_ip,
            "SUCCESS",
            None,
        );

        let expiration = Utc::now()
            .checked_add_signed(Duration::hours(2))
            .expect("valid timestamp")
            .timestamp();

        let claims = Claims {
            sub: "admin".to_string(),
            role: UserRole::Admin,
            iat: Utc::now().timestamp() as usize,
            exp: expiration as usize,
            jti: Some(uuid::Uuid::new_v4().to_string()),
        };

        let token = encode(
            &Header::default(),
            &claims,
            &EncodingKey::from_secret(JWT_SECRET.as_slice()),
        )
        .unwrap_or_default();

        let cookie = build_auth_cookie(&req, &token);
        HttpResponse::Ok().cookie(cookie).json(LoginResponse {
            token,
            username: "admin".to_string(),
            role: UserRole::Admin.as_str().to_string(),
        })
    } else {
        match user_db.authenticate(username, &body.password) {
            Ok(user) => {
                LOGIN_RATE_LIMITER.reset(&client_ip);
                audit.log(
                    &user.username,
                    user.role.as_str(),
                    "LOGIN_SUCCESS",
                    None,
                    &client_ip,
                    "SUCCESS",
                    None,
                );

                let expiration = Utc::now()
                    .checked_add_signed(Duration::hours(2))
                    .expect("valid timestamp")
                    .timestamp();

                let claims = Claims {
                    sub: user.username.clone(),
                    role: user.role,
                    iat: Utc::now().timestamp() as usize,
                    exp: expiration as usize,
                    jti: Some(uuid::Uuid::new_v4().to_string()),
                };

                let token = encode(
                    &Header::default(),
                    &claims,
                    &EncodingKey::from_secret(JWT_SECRET.as_slice()),
                )
                .unwrap_or_default();

                let cookie = build_auth_cookie(&req, &token);
                HttpResponse::Ok().cookie(cookie).json(LoginResponse {
                    token,
                    username: user.username,
                    role: user.role.as_str().to_string(),
                })
            }
            Err(_) => {
                audit.log(
                    username,
                    "unknown",
                    "LOGIN_FAILED",
                    None,
                    &client_ip,
                    "DENIED",
                    Some("Invalid credentials"),
                );
                HttpResponse::Unauthorized().json("Invalid credentials")
            }
        }
    }
}

pub async fn logout(
    req: HttpRequest,
    user_db: web::Data<std::sync::Arc<crate::auth::UserDatabase>>,
    audit: web::Data<std::sync::Arc<crate::audit::AuditLogger>>,
) -> impl Responder {
    let client_ip = req
        .connection_info()
        .realip_remote_addr()
        .unwrap_or("unknown")
        .to_string();

    if let Some(token) = crate::auth::extract_token(&req) {
        let secret = JWT_SECRET.as_slice();
        if let Ok(token_data) = jsonwebtoken::decode::<Claims>(
            &token,
            &jsonwebtoken::DecodingKey::from_secret(secret),
            &jsonwebtoken::Validation::new(jsonwebtoken::Algorithm::HS256),
        ) {
            let token_hash = crate::auth::hash_token(&token);
            let _ = user_db.revoke_token(
                &token_hash,
                token_data.claims.jti.as_deref(),
                &token_data.claims.sub,
                token_data.claims.exp as i64,
            );
            audit.log(
                &token_data.claims.sub,
                token_data.claims.role.as_str(),
                "LOGOUT",
                None,
                &client_ip,
                "SUCCESS",
                Some("User logged out and token was revoked"),
            );
        } else {
            let token_hash = crate::auth::hash_token(&token);
            let exp = Utc::now().timestamp() + 86400;
            let _ = user_db.revoke_token(&token_hash, None, "unknown", exp);
            audit.log(
                "unknown",
                "unknown",
                "LOGOUT",
                None,
                &client_ip,
                "SUCCESS",
                Some("Session cleared on logout"),
            );
        }
    }

    let cookie = build_logout_cookie();
    HttpResponse::Ok()
        .cookie(cookie)
        .json(serde_json::json!({ "message": "Logged out successfully" }))
}

pub async fn get_me(user: crate::auth::AuthenticatedUser) -> impl Responder {
    HttpResponse::Ok().json(serde_json::json!({
        "username": user.username,
        "role": user.role.as_str()
    }))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_jwt_secret_generation() {
        let secret = JWT_SECRET.as_slice();
        assert_eq!(secret.len(), 32);
        assert!(!secret.iter().all(|&b| b == 0));
    }

    #[test]
    fn test_login_rate_limiter() {
        let limiter = LoginRateLimiter::new();
        let test_ip = "192.0.2.1";

        // First 5 attempts should not be rate limited
        for _ in 0..5 {
            assert!(!limiter.is_rate_limited(test_ip));
        }

        // 6th attempt should be rate limited
        assert!(limiter.is_rate_limited(test_ip));

        // After reset, should allow again
        limiter.reset(test_ip);
        assert!(!limiter.is_rate_limited(test_ip));
    }

    #[test]
    fn test_auth_cookie_attributes() {
        use actix_web::test::TestRequest;

        let req = TestRequest::default().to_http_request();
        let cookie = build_auth_cookie(&req, "test_jwt_value");

        assert_eq!(cookie.name(), "wadm_token");
        assert_eq!(cookie.value(), "test_jwt_value");
        assert_eq!(cookie.path(), Some("/"));
        assert_eq!(cookie.http_only(), Some(true));
        assert_eq!(cookie.same_site(), Some(SameSite::Strict));
        assert_eq!(
            cookie.max_age(),
            Some(actix_web::cookie::time::Duration::seconds(86400))
        );

        let logout_cookie = build_logout_cookie();
        assert_eq!(logout_cookie.name(), "wadm_token");
        assert_eq!(logout_cookie.value(), "");
        assert_eq!(logout_cookie.path(), Some("/"));
        assert_eq!(logout_cookie.http_only(), Some(true));
        assert_eq!(logout_cookie.same_site(), Some(SameSite::Strict));
        assert_eq!(
            logout_cookie.max_age(),
            Some(actix_web::cookie::time::Duration::ZERO)
        );
    }

    #[actix_web::test]
    async fn test_logout_endpoint_clears_cookie_and_revokes_token() {
        use actix_web::test::{self, TestRequest};
        use actix_web::App;

        let user_db = std::sync::Arc::new(crate::auth::UserDatabase::new_in_memory().unwrap());
        let audit = std::sync::Arc::new(crate::audit::AuditLogger::new_in_memory().unwrap());

        let user = user_db
            .create_user("charlie", "password123", UserRole::Viewer)
            .unwrap();
        let expiration = Utc::now().timestamp() + 3600;
        let claims = Claims {
            sub: user.username.clone(),
            role: user.role,
            iat: Utc::now().timestamp() as usize,
            exp: expiration as usize,
            jti: Some(uuid::Uuid::new_v4().to_string()),
        };
        let token = encode(
            &Header::default(),
            &claims,
            &EncodingKey::from_secret(JWT_SECRET.as_slice()),
        )
        .unwrap();

        let token_hash = crate::auth::hash_token(&token);
        assert!(!user_db.is_token_revoked(&token_hash, &user.username, claims.iat));

        let app = test::init_service(
            App::new()
                .app_data(web::Data::new(user_db.clone()))
                .app_data(web::Data::new(audit.clone()))
                .service(web::resource("/logout").route(web::post().to(logout))),
        )
        .await;

        let req = TestRequest::post()
            .uri("/logout")
            .insert_header((
                actix_web::http::header::COOKIE,
                format!("wadm_token={}", token),
            ))
            .to_request();

        let resp = test::call_service(&app, req).await;
        assert_eq!(resp.status(), actix_web::http::StatusCode::OK);

        // Verify Set-Cookie header contains Max-Age=0
        let set_cookie = resp.headers().get(actix_web::http::header::SET_COOKIE);
        assert!(set_cookie.is_some());
        let cookie_str = set_cookie.unwrap().to_str().unwrap();
        assert!(cookie_str.contains("wadm_token="));
        assert!(cookie_str.contains("Max-Age=0"));

        // Verify token is now marked as revoked in UserDatabase
        assert!(user_db.is_token_revoked(&token_hash, &user.username, claims.iat));
    }

    #[actix_web::test]
    async fn test_get_me_endpoint_returns_user() {
        use actix_web::test::{self, TestRequest};
        use actix_web::App;

        let user_db = std::sync::Arc::new(crate::auth::UserDatabase::new_in_memory().unwrap());
        let claims = Claims {
            sub: "dave".to_string(),
            role: UserRole::Operator,
            iat: Utc::now().timestamp() as usize,
            exp: Utc::now().timestamp() as usize + 3600,
            jti: Some(uuid::Uuid::new_v4().to_string()),
        };
        let token = encode(
            &Header::default(),
            &claims,
            &EncodingKey::from_secret(JWT_SECRET.as_slice()),
        )
        .unwrap();

        let app = test::init_service(
            App::new()
                .app_data(web::Data::new(user_db.clone()))
                .service(web::resource("/me").route(web::get().to(get_me))),
        )
        .await;

        let req = TestRequest::get()
            .uri("/me")
            .insert_header((
                actix_web::http::header::COOKIE,
                format!("wadm_token={}", token),
            ))
            .to_request();

        let resp = test::call_service(&app, req).await;
        assert_eq!(resp.status(), actix_web::http::StatusCode::OK);
        let body: serde_json::Value = test::read_body_json(resp).await;
        assert_eq!(body["username"], "dave");
        assert_eq!(body["role"], "operator");
    }
}
