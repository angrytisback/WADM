use actix_web::{web, HttpResponse};
use std::sync::Arc;

use crate::audit::{AuditLogQuery, AuditLogger};
use crate::auth::AuthenticatedUser;
use crate::drivers::error::AppError;

pub async fn get_audit_logs(
    _user: AuthenticatedUser,
    audit_logger: web::Data<Arc<AuditLogger>>,
    query: web::Query<AuditLogQuery>,
) -> Result<HttpResponse, AppError> {
    let page = audit_logger.query_logs(query.into_inner())?;
    Ok(HttpResponse::Ok().json(page))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::auth::UserRole;

    #[actix_web::test]
    async fn test_audit_logs_endpoint() {
        let logger = Arc::new(AuditLogger::new_in_memory().unwrap());
        let logger_data = web::Data::new(logger);

        let user = AuthenticatedUser {
            username: "viewer".to_string(),
            role: UserRole::Viewer,
            client_ip: "127.0.0.1".to_string(),
        };

        let resp = get_audit_logs(user, logger_data, web::Query(AuditLogQuery::default())).await;
        assert!(resp.is_ok());
    }
}
