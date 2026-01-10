use actix_web::{web, HttpResponse, Responder};
use argon2::{
    password_hash::{rand_core::OsRng, PasswordHash, PasswordHasher, PasswordVerifier, SaltString},
    Argon2,
};
use chrono::{Duration, Utc};
use jsonwebtoken::{encode, EncodingKey, Header};
use serde::{Deserialize, Serialize};
use std::fs;
use std::sync::Mutex;
use totp_rs::{Algorithm, Secret, TOTP};

const AUTH_FILE: &str = "wadm-auth.json";
pub const JWT_SECRET: &[u8] = b"super_secret_key_change_this_in_prod"; 

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



pub fn load_auth_store() -> Option<AuthStore> {
    log::info!("Attempting to load auth store from: {}", AUTH_FILE);
    match fs::read_to_string(AUTH_FILE) {
        Ok(content) => match serde_json::from_str(&content) {
            Ok(store) => {
                log::info!("Successfully loaded auth store.");
                Some(store)
            }
            Err(e) => {
                log::error!("CRITICAL: Failed to parse auth store: {}", e);
                
                panic!("Auth store corrupted. Manual intervention required.");
            }
        },
        Err(e) => {
            if e.kind() == std::io::ErrorKind::NotFound {
                log::warn!("Auth store file not found. Assuming first run/setup required.");
                None
            } else {
                log::error!("CRITICAL: Failed to read auth store file: {}", e);
                
                panic!("Failed to access auth store: {}", e);
            }
        }
    }
}

pub async fn get_auth_status(data: web::Data<Mutex<Option<AuthStore>>>) -> impl Responder {
    let store = data.lock().unwrap();

    HttpResponse::Ok().json(AuthStatus {
        setup_required: store.is_none(),
    })
}

pub async fn init_setup() -> impl Responder {
    
    let secret = Secret::generate_secret();
    let totp = TOTP::new(
        Algorithm::SHA1,
        6,
        1,
        30,
        secret.to_bytes().unwrap(),
        Some("WADM".to_string()),
        "admin@wadm".to_string(),
    )
    .unwrap();

    let qr = totp.get_qr_base64().unwrap();

    HttpResponse::Ok().json(SetupInitResponse {
        secret: secret.to_encoded().to_string(),
        qr,
    })
}

pub async fn confirm_setup(
    body: web::Json<SetupRequest>,
    data: web::Data<Mutex<Option<AuthStore>>>,
) -> impl Responder {
    let mut store_guard = data.lock().unwrap();

    
    if store_guard.is_some() {
        return HttpResponse::BadRequest().json("Setup already complete");
    }

    
    let secret_bytes = match Secret::Encoded(body.secret.clone()).to_bytes() {
        Ok(b) => b,
        Err(_) => return HttpResponse::BadRequest().json("Invalid secret format"),
    };

    let totp = match TOTP::new(
        Algorithm::SHA1,
        6,
        1,
        30,
        secret_bytes,
        None,
        "".to_string(),
    ) {
        Ok(t) => t,
        Err(e) => {
            log::error!("Failed to create TOTP instance: {}", e);
            return HttpResponse::InternalServerError().json("Failed to initialize TOTP");
        }
    };

    if !totp.check_current(&body.code).unwrap_or(false) {
        return HttpResponse::BadRequest().json("Invalid 2FA code");
    }

    
    let salt = SaltString::generate(&mut OsRng);
    let argon2 = Argon2::default();
    let password_hash = argon2
        .hash_password(body.password.as_bytes(), &salt)
        .unwrap()
        .to_string();

    
    let new_store = AuthStore {
        password_hash,
        totp_secret: body.secret.clone(),
        setup_complete: true,
    };

    match serde_json::to_string(&new_store) {
        Ok(json) => {
            if let Err(e) = fs::write(AUTH_FILE, json) {
                log::error!(
                    "CRITICAL: Failed to write auth file to {}: {}",
                    AUTH_FILE,
                    e
                );
                return HttpResponse::InternalServerError().json("Failed to save auth state");
            } else {
                log::info!("Successfully persisted auth state to {}", AUTH_FILE);
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
        .checked_add_signed(Duration::days(1))
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
        &EncodingKey::from_secret(JWT_SECRET),
    )
    .unwrap();

    HttpResponse::Ok().json(LoginResponse { token })
}

pub async fn login(
    body: web::Json<LoginRequest>,
    data: web::Data<Mutex<Option<AuthStore>>>,
) -> impl Responder {
    let store_guard = data.lock().unwrap();

    let store = match &*store_guard {
        Some(s) => s,
        None => return HttpResponse::BadRequest().json("Setup required"),
    };

    
    let parsed_hash = PasswordHash::new(&store.password_hash).unwrap();
    if Argon2::default()
        .verify_password(body.password.as_bytes(), &parsed_hash)
        .is_err()
    {
        return HttpResponse::Unauthorized().json("Invalid credentials");
    }

    
    let secret_bytes = Secret::Encoded(store.totp_secret.clone())
        .to_bytes()
        .unwrap();
    let totp = match TOTP::new(
        Algorithm::SHA1,
        6,
        1,
        30,
        secret_bytes,
        None,
        "".to_string(),
    ) {
        Ok(t) => t,
        Err(e) => {
            log::error!("Failed to create TOTP instance during login: {}", e);
            return HttpResponse::InternalServerError().json("Failed to verify TOTP");
        }
    };

    if !totp.check_current(&body.code).unwrap_or(false) {
        return HttpResponse::Unauthorized().json("Invalid 2FA code");
    }

    
    let expiration = Utc::now()
        .checked_add_signed(Duration::days(1))
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
        &EncodingKey::from_secret(JWT_SECRET),
    )
    .unwrap();

    HttpResponse::Ok().json(LoginResponse { token })
}
