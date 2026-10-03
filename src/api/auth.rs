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

#[derive(Serialize, Deserialize)]
pub struct Claims {
    pub sub: String,
    pub exp: usize,
    pub iat: usize,
}

#[derive(Deserialize)]
pub struct LoginRequest {
    password: String,
    code: String,
}

#[derive(Deserialize)]
pub struct SetupRequest {
    password: String,
    code: String,
    secret: String,
}

#[derive(Serialize)]
struct LoginResponse {
    token: String,
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
    body: web::Json<SetupRequest>,
    data: web::Data<Mutex<Option<AuthStore>>>,
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

    let role = "admin";
    let expiration = Utc::now()
        .checked_add_signed(Duration::hours(2))
        .expect("valid timestamp")
        .timestamp();

    let claims = Claims {
        sub: role.to_owned(),
        iat: Utc::now().timestamp() as usize,
        exp: expiration as usize,
    };

    let token = encode(
        &Header::default(),
        &claims,
        &EncodingKey::from_secret(JWT_SECRET.as_slice()),
    )
    .unwrap_or_default();

    HttpResponse::Ok().json(LoginResponse { token })
}

pub async fn login(
    req: HttpRequest,
    body: web::Json<LoginRequest>,
    data: web::Data<Mutex<Option<AuthStore>>>,
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
        return HttpResponse::TooManyRequests()
            .json("Too many login attempts. Please wait 1 minute.");
    }

    let store_guard = data.lock().unwrap_or_else(|e| e.into_inner());

    let store = match &*store_guard {
        Some(s) => s,
        None => return HttpResponse::BadRequest().json("Setup required"),
    };

    let parsed_hash = match PasswordHash::new(&store.password_hash) {
        Ok(h) => h,
        Err(_) => return HttpResponse::InternalServerError().json("Invalid stored hash"),
    };

    if Argon2::default()
        .verify_password(body.password.as_bytes(), &parsed_hash)
        .is_err()
    {
        return HttpResponse::Unauthorized().json("Invalid credentials");
    }

    let secret_bytes = match Secret::Encoded(store.totp_secret.clone()).to_bytes() {
        Ok(b) => b,
        Err(_) => return HttpResponse::InternalServerError().json("Invalid secret format"),
    };

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

    if !valid_2fa {
        return HttpResponse::Unauthorized().json("Invalid 2FA code");
    }

    // Login successful: reset rate limiter
    LOGIN_RATE_LIMITER.reset(&client_ip);

    let expiration = Utc::now()
        .checked_add_signed(Duration::hours(2))
        .expect("valid timestamp")
        .timestamp();

    let claims = Claims {
        sub: "admin".to_string(),
        iat: Utc::now().timestamp() as usize,
        exp: expiration as usize,
    };

    let token = encode(
        &Header::default(),
        &claims,
        &EncodingKey::from_secret(JWT_SECRET.as_slice()),
    )
    .unwrap_or_default();

    HttpResponse::Ok().json(LoginResponse { token })
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
}
