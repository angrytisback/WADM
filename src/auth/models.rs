use crate::drivers::error::AppError;
use actix_web::dev::Payload;
use actix_web::{FromRequest, HttpMessage, HttpRequest};
use futures_util::future::{err, ok, Ready};
use jsonwebtoken::{decode, Algorithm, DecodingKey, Validation};
use serde::{Deserialize, Serialize};
use std::fmt;

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum UserRole {
    Viewer = 1,
    Operator = 2,
    Admin = 3,
}

impl UserRole {
    pub fn as_str(&self) -> &'static str {
        match self {
            UserRole::Viewer => "viewer",
            UserRole::Operator => "operator",
            UserRole::Admin => "admin",
        }
    }

    pub fn from_str(s: &str) -> Option<Self> {
        match s.to_lowercase().trim() {
            "viewer" => Some(UserRole::Viewer),
            "operator" => Some(UserRole::Operator),
            "admin" => Some(UserRole::Admin),
            _ => None,
        }
    }
}

impl fmt::Display for UserRole {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.as_str())
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct User {
    pub id: i64,
    pub username: String,
    pub role: UserRole,
    pub created_at: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Claims {
    pub sub: String,
    pub role: UserRole,
    pub exp: usize,
    pub iat: usize,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub jti: Option<String>,
}

#[derive(Debug, Clone)]
pub struct AuthenticatedUser {
    pub username: String,
    pub role: UserRole,
    pub client_ip: String,
}

impl AuthenticatedUser {
    pub fn require_role(&self, minimum_role: UserRole) -> Result<(), AppError> {
        if self.role >= minimum_role {
            Ok(())
        } else {
            Err(AppError::Forbidden("Insufficient permissions".to_string()))
        }
    }

    pub fn require_admin(&self) -> Result<(), AppError> {
        self.require_role(UserRole::Admin)
    }

    pub fn require_operator(&self) -> Result<(), AppError> {
        self.require_role(UserRole::Operator)
    }

    #[allow(dead_code)]
    pub fn can_operate(&self) -> bool {
        self.role >= UserRole::Operator
    }

    #[allow(dead_code)]
    pub fn is_admin(&self) -> bool {
        self.role == UserRole::Admin
    }
}

pub fn hash_token(token: &str) -> String {
    use sha2::{Digest, Sha256};
    let mut hasher = Sha256::new();
    hasher.update(token.as_bytes());
    format!("{:x}", hasher.finalize())
}

pub fn extract_client_ip(req: &HttpRequest) -> String {
    if let Some(forwarded) = req.headers().get("x-forwarded-for") {
        if let Ok(f_str) = forwarded.to_str() {
            if let Some(first) = f_str.split(',').next() {
                let ip = first.trim();
                if !ip.is_empty() {
                    return ip.to_string();
                }
            }
        }
    }

    req.connection_info()
        .realip_remote_addr()
        .unwrap_or("127.0.0.1")
        .to_string()
}

pub fn extract_token(req: &HttpRequest) -> Option<String> {
    // 1. Check HttpOnly cookie first
    if let Some(cookie) = req.cookie("wadm_token") {
        let val = cookie.value().trim();
        if !val.is_empty() {
            return Some(val.to_string());
        }
    }

    // 2. Check Authorization Bearer header
    if let Some(auth_val) = req.headers().get("Authorization") {
        if let Ok(auth_str) = auth_val.to_str() {
            let parts: Vec<&str> = auth_str.split_whitespace().collect();
            if parts.len() == 2 && parts[0].eq_ignore_ascii_case("bearer") {
                return Some(parts[1].to_string());
            }
        }
    }

    // 3. Check Sec-WebSocket-Protocol
    if let Some(proto) = req.headers().get("Sec-WebSocket-Protocol") {
        if let Ok(p) = proto.to_str() {
            let trimmed = p.trim();
            if !trimmed.is_empty() {
                return Some(trimmed.to_string());
            }
        }
    }

    // 4. Check query string (?token=...)
    let query = req.query_string();
    for pair in query.split('&') {
        let mut kv = pair.split('=');
        if let (Some(k), Some(v)) = (kv.next(), kv.next()) {
            if k == "token" && !v.is_empty() {
                return Some(v.to_string());
            }
        }
    }

    None
}

impl FromRequest for AuthenticatedUser {
    type Error = AppError;
    type Future = Ready<Result<Self, Self::Error>>;

    fn from_request(req: &HttpRequest, _payload: &mut Payload) -> Self::Future {
        let client_ip = extract_client_ip(req);

        // 1. Check if Claims was already inserted into request extensions by AuthMiddleware
        if let Some(claims) = req.extensions().get::<Claims>() {
            return ok(AuthenticatedUser {
                username: claims.sub.clone(),
                role: claims.role,
                client_ip,
            });
        }

        // 2. Fallback: extract and decode token directly
        let token = match extract_token(req) {
            Some(t) => t,
            None => {
                return err(AppError::Unauthorized(
                    "Missing authentication token".to_string(),
                ));
            }
        };

        let secret = crate::api::auth::JWT_SECRET.as_slice();
        match decode::<Claims>(
            &token,
            &DecodingKey::from_secret(secret),
            &Validation::new(Algorithm::HS256),
        ) {
            Ok(token_data) => {
                if let Some(user_db) = req
                    .app_data::<actix_web::web::Data<std::sync::Arc<crate::auth::UserDatabase>>>()
                {
                    let token_hash = hash_token(&token);
                    if user_db.is_token_revoked(
                        &token_hash,
                        &token_data.claims.sub,
                        token_data.claims.iat,
                    ) {
                        return err(AppError::Unauthorized("Token has been revoked".to_string()));
                    }
                }

                ok(AuthenticatedUser {
                    username: token_data.claims.sub,
                    role: token_data.claims.role,
                    client_ip,
                })
            }
            Err(_) => err(AppError::Unauthorized(
                "Invalid authentication token".to_string(),
            )),
        }
    }
}

pub struct RequireAdmin(pub AuthenticatedUser);

impl FromRequest for RequireAdmin {
    type Error = AppError;
    type Future = Ready<Result<Self, Self::Error>>;

    fn from_request(req: &HttpRequest, payload: &mut Payload) -> Self::Future {
        match AuthenticatedUser::from_request(req, payload).into_inner() {
            Ok(user) => match user.require_admin() {
                Ok(_) => ok(RequireAdmin(user)),
                Err(e) => err(e),
            },
            Err(e) => err(e),
        }
    }
}

#[allow(dead_code)]
pub struct RequireOperator(pub AuthenticatedUser);

impl FromRequest for RequireOperator {
    type Error = AppError;
    type Future = Ready<Result<Self, Self::Error>>;

    fn from_request(req: &HttpRequest, payload: &mut Payload) -> Self::Future {
        match AuthenticatedUser::from_request(req, payload).into_inner() {
            Ok(user) => match user.require_operator() {
                Ok(_) => ok(RequireOperator(user)),
                Err(e) => err(e),
            },
            Err(e) => err(e),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_user_role_hierarchy() {
        assert!(UserRole::Admin > UserRole::Operator);
        assert!(UserRole::Operator > UserRole::Viewer);
        assert!(UserRole::Admin > UserRole::Viewer);

        assert_eq!(UserRole::from_str("admin"), Some(UserRole::Admin));
        assert_eq!(UserRole::from_str("operator"), Some(UserRole::Operator));
        assert_eq!(UserRole::from_str("viewer"), Some(UserRole::Viewer));
        assert_eq!(UserRole::from_str("invalid"), None);
    }

    #[test]
    fn test_authenticated_user_permissions() {
        let admin = AuthenticatedUser {
            username: "admin".to_string(),
            role: UserRole::Admin,
            client_ip: "127.0.0.1".to_string(),
        };
        assert!(admin.require_admin().is_ok());
        assert!(admin.require_operator().is_ok());
        assert!(admin.require_role(UserRole::Viewer).is_ok());

        let operator = AuthenticatedUser {
            username: "op".to_string(),
            role: UserRole::Operator,
            client_ip: "127.0.0.1".to_string(),
        };
        assert!(operator.require_admin().is_err());
        assert!(operator.require_operator().is_ok());
        assert!(operator.require_role(UserRole::Viewer).is_ok());

        let viewer = AuthenticatedUser {
            username: "view".to_string(),
            role: UserRole::Viewer,
            client_ip: "127.0.0.1".to_string(),
        };
        assert!(viewer.require_admin().is_err());
        assert!(viewer.require_operator().is_err());
        assert!(viewer.require_role(UserRole::Viewer).is_ok());
    }

    #[test]
    fn test_hash_token_deterministic() {
        let token = "my_sample_jwt_token";
        let h1 = hash_token(token);
        let h2 = hash_token(token);
        assert_eq!(h1, h2);
        assert_eq!(h1.len(), 64);
        assert_ne!(h1, hash_token("different_token"));
    }

    #[test]
    fn test_extract_token_priorities() {
        use actix_web::test::TestRequest;

        // 1. Cookie priority
        let req1 = TestRequest::default()
            .insert_header((
                actix_web::http::header::COOKIE,
                "wadm_token=cookie_token; other=123",
            ))
            .insert_header((
                actix_web::http::header::AUTHORIZATION,
                "Bearer header_token",
            ))
            .to_http_request();
        assert_eq!(extract_token(&req1), Some("cookie_token".to_string()));

        // 2. Bearer header fallback
        let req2 = TestRequest::default()
            .insert_header((
                actix_web::http::header::AUTHORIZATION,
                "Bearer header_token",
            ))
            .to_http_request();
        assert_eq!(extract_token(&req2), Some("header_token".to_string()));

        // 3. Query string fallback
        let req3 = TestRequest::default()
            .uri("/api/test?token=query_token")
            .to_http_request();
        assert_eq!(extract_token(&req3), Some("query_token".to_string()));
    }
}
