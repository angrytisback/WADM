use actix_web::{web, HttpResponse};
use serde::Deserialize;
use std::sync::Arc;

use crate::audit::AuditLogger;
use crate::auth::{RequireAdmin, UserDatabase, UserRole};
use crate::drivers::error::AppError;

#[derive(Deserialize)]
pub struct CreateUserRequest {
    pub username: String,
    pub password: String,
    pub role: String,
}

#[derive(Deserialize)]
pub struct UpdateRoleRequest {
    pub role: String,
}

#[derive(Deserialize)]
pub struct UpdatePasswordRequest {
    pub password: String,
}

pub async fn list_users(
    _admin: RequireAdmin,
    user_db: web::Data<Arc<UserDatabase>>,
) -> Result<HttpResponse, AppError> {
    let users = user_db.list_users()?;
    Ok(HttpResponse::Ok().json(users))
}

pub async fn create_user(
    admin: RequireAdmin,
    user_db: web::Data<Arc<UserDatabase>>,
    audit: web::Data<Arc<AuditLogger>>,
    body: web::Json<CreateUserRequest>,
) -> Result<HttpResponse, AppError> {
    let role = UserRole::from_str(&body.role).ok_or_else(|| {
        AppError::InvalidInput(format!(
            "Invalid role '{}'. Allowed roles: viewer, operator, admin",
            body.role
        ))
    })?;

    match user_db.create_user(&body.username, &body.password, role) {
        Ok(user) => {
            audit.log(
                &admin.0.username,
                admin.0.role.as_str(),
                "USER_CREATE",
                Some(&user.username),
                &admin.0.client_ip,
                "SUCCESS",
                Some(&format!("Created user with role {}", role.as_str())),
            );
            Ok(HttpResponse::Ok().json(user))
        }
        Err(e) => {
            audit.log(
                &admin.0.username,
                admin.0.role.as_str(),
                "USER_CREATE",
                Some(&body.username),
                &admin.0.client_ip,
                "FAILED",
                Some(&e.to_string()),
            );
            Err(e)
        }
    }
}

pub async fn update_user_role(
    admin: RequireAdmin,
    user_db: web::Data<Arc<UserDatabase>>,
    audit: web::Data<Arc<AuditLogger>>,
    path: web::Path<i64>,
    body: web::Json<UpdateRoleRequest>,
) -> Result<HttpResponse, AppError> {
    let user_id = path.into_inner();
    let new_role = UserRole::from_str(&body.role).ok_or_else(|| {
        AppError::InvalidInput(format!(
            "Invalid role '{}'. Allowed roles: viewer, operator, admin",
            body.role
        ))
    })?;

    match user_db.update_role(user_id, new_role) {
        Ok(_) => {
            audit.log(
                &admin.0.username,
                admin.0.role.as_str(),
                "USER_UPDATE_ROLE",
                Some(&format!("user_id:{}", user_id)),
                &admin.0.client_ip,
                "SUCCESS",
                Some(&format!("Role changed to {}", new_role.as_str())),
            );
            Ok(HttpResponse::Ok().json(serde_json::json!({
                "status": "ok",
                "message": format!("Role updated to {}", new_role.as_str())
            })))
        }
        Err(e) => {
            audit.log(
                &admin.0.username,
                admin.0.role.as_str(),
                "USER_UPDATE_ROLE",
                Some(&format!("user_id:{}", user_id)),
                &admin.0.client_ip,
                "FAILED",
                Some(&e.to_string()),
            );
            Err(e)
        }
    }
}

pub async fn update_user_password(
    admin: RequireAdmin,
    user_db: web::Data<Arc<UserDatabase>>,
    audit: web::Data<Arc<AuditLogger>>,
    path: web::Path<i64>,
    body: web::Json<UpdatePasswordRequest>,
) -> Result<HttpResponse, AppError> {
    let user_id = path.into_inner();
    match user_db.update_password(user_id, &body.password) {
        Ok(_) => {
            audit.log(
                &admin.0.username,
                admin.0.role.as_str(),
                "USER_RESET_PASSWORD",
                Some(&format!("user_id:{}", user_id)),
                &admin.0.client_ip,
                "SUCCESS",
                None,
            );
            Ok(HttpResponse::Ok().json(serde_json::json!({
                "status": "ok",
                "message": "Password reset successfully"
            })))
        }
        Err(e) => {
            audit.log(
                &admin.0.username,
                admin.0.role.as_str(),
                "USER_RESET_PASSWORD",
                Some(&format!("user_id:{}", user_id)),
                &admin.0.client_ip,
                "FAILED",
                Some(&e.to_string()),
            );
            Err(e)
        }
    }
}

pub async fn delete_user(
    admin: RequireAdmin,
    user_db: web::Data<Arc<UserDatabase>>,
    audit: web::Data<Arc<AuditLogger>>,
    path: web::Path<i64>,
) -> Result<HttpResponse, AppError> {
    let user_id = path.into_inner();

    match user_db.delete_user(user_id) {
        Ok(_) => {
            audit.log(
                &admin.0.username,
                admin.0.role.as_str(),
                "USER_DELETE",
                Some(&format!("user_id:{}", user_id)),
                &admin.0.client_ip,
                "SUCCESS",
                None,
            );
            Ok(HttpResponse::Ok().json(serde_json::json!({
                "status": "ok",
                "message": "User deleted successfully"
            })))
        }
        Err(e) => {
            audit.log(
                &admin.0.username,
                admin.0.role.as_str(),
                "USER_DELETE",
                Some(&format!("user_id:{}", user_id)),
                &admin.0.client_ip,
                "FAILED",
                Some(&e.to_string()),
            );
            Err(e)
        }
    }
}
