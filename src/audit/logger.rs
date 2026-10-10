use chrono::Utc;
use rusqlite::{params, Connection};
use serde::{Deserialize, Serialize};
use std::path::PathBuf;
use std::sync::Mutex;

use crate::auth::AuthenticatedUser;
use crate::drivers::error::AppError;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AuditLogEntry {
    pub id: i64,
    pub timestamp: String,
    pub username: String,
    pub role: String,
    pub action: String,
    pub resource: Option<String>,
    pub ip_address: String,
    pub status: String,
    pub details: Option<String>,
}

#[derive(Debug, Clone, Deserialize, Default)]
pub struct AuditLogQuery {
    pub page: Option<usize>,
    pub limit: Option<usize>,
    pub user: Option<String>,
    pub action: Option<String>,
    pub status: Option<String>,
    pub search: Option<String>,
}

#[derive(Debug, Clone, Serialize)]
pub struct AuditLogPage {
    pub logs: Vec<AuditLogEntry>,
    pub total: i64,
    pub page: usize,
    pub limit: usize,
    pub total_pages: usize,
}

pub struct AuditLogger {
    conn: Mutex<Connection>,
}

impl AuditLogger {
    pub fn new(path: PathBuf) -> Result<Self, rusqlite::Error> {
        let conn = Connection::open(&path)?;
        conn.execute_batch(
            "PRAGMA journal_mode = WAL;
             PRAGMA synchronous = NORMAL;
             CREATE TABLE IF NOT EXISTS audit_logs (
                 id INTEGER PRIMARY KEY AUTOINCREMENT,
                 timestamp TEXT NOT NULL,
                 username TEXT NOT NULL,
                 role TEXT NOT NULL,
                 action TEXT NOT NULL,
                 resource TEXT,
                 ip_address TEXT NOT NULL,
                 status TEXT NOT NULL,
                 details TEXT
             );
             CREATE INDEX IF NOT EXISTS idx_audit_logs_timestamp ON audit_logs(timestamp DESC);
             CREATE INDEX IF NOT EXISTS idx_audit_logs_username ON audit_logs(username);
             CREATE INDEX IF NOT EXISTS idx_audit_logs_action ON audit_logs(action);
             CREATE INDEX IF NOT EXISTS idx_audit_logs_status ON audit_logs(status);",
        )?;

        Ok(Self {
            conn: Mutex::new(conn),
        })
    }

    #[cfg(test)]
    pub fn new_in_memory() -> Result<Self, rusqlite::Error> {
        let conn = Connection::open_in_memory()?;
        conn.execute_batch(
            "CREATE TABLE IF NOT EXISTS audit_logs (
                 id INTEGER PRIMARY KEY AUTOINCREMENT,
                 timestamp TEXT NOT NULL,
                 username TEXT NOT NULL,
                 role TEXT NOT NULL,
                 action TEXT NOT NULL,
                 resource TEXT,
                 ip_address TEXT NOT NULL,
                 status TEXT NOT NULL,
                 details TEXT
             );
             CREATE INDEX IF NOT EXISTS idx_audit_logs_timestamp ON audit_logs(timestamp DESC);",
        )?;

        Ok(Self {
            conn: Mutex::new(conn),
        })
    }

    #[allow(clippy::too_many_arguments)]
    pub fn log(
        &self,
        username: &str,
        role: &str,
        action: &str,
        resource: Option<&str>,
        ip_address: &str,
        status: &str,
        details: Option<&str>,
    ) {
        let now = Utc::now().to_rfc3339();
        let conn = self.conn.lock().unwrap();
        if let Err(e) = conn.execute(
            "INSERT INTO audit_logs (timestamp, username, role, action, resource, ip_address, status, details)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8)",
            params![
                now,
                username,
                role,
                action,
                resource,
                ip_address,
                status,
                details
            ],
        ) {
            log::error!("CRITICAL: Failed to write to audit log: {}", e);
        }
    }

    pub fn log_denied(
        &self,
        user: &AuthenticatedUser,
        action: &str,
        resource: Option<&str>,
        reason: Option<&str>,
    ) {
        self.log(
            &user.username,
            user.role.as_str(),
            action,
            resource,
            &user.client_ip,
            "DENIED",
            reason,
        );
    }

    pub fn query_logs(&self, query: AuditLogQuery) -> Result<AuditLogPage, AppError> {
        let page = query.page.unwrap_or(1).max(1);
        let limit = query.limit.unwrap_or(50).clamp(1, 200);
        let offset = (page - 1) * limit;

        let conn = self.conn.lock().unwrap();

        let mut conditions = Vec::new();
        let mut sql_params: Vec<Box<dyn rusqlite::ToSql>> = Vec::new();

        if let Some(user) = query.user.filter(|s| !s.trim().is_empty()) {
            conditions.push("username LIKE ?");
            sql_params.push(Box::new(format!("%{}%", user.trim())));
        }

        if let Some(action) = query.action.filter(|s| !s.trim().is_empty()) {
            conditions.push("action LIKE ?");
            sql_params.push(Box::new(format!("%{}%", action.trim())));
        }

        if let Some(status) = query.status.filter(|s| !s.trim().is_empty()) {
            conditions.push("status = ?");
            sql_params.push(Box::new(status.trim().to_uppercase()));
        }

        if let Some(search) = query.search.filter(|s| !s.trim().is_empty()) {
            conditions.push("(resource LIKE ? OR details LIKE ? OR action LIKE ?)");
            let term = format!("%{}%", search.trim());
            sql_params.push(Box::new(term.clone()));
            sql_params.push(Box::new(term.clone()));
            sql_params.push(Box::new(term));
        }

        let where_clause = if conditions.is_empty() {
            String::new()
        } else {
            format!("WHERE {}", conditions.join(" AND "))
        };

        // Count total
        let count_sql = format!("SELECT COUNT(*) FROM audit_logs {}", where_clause);
        let mut count_stmt = conn
            .prepare(&count_sql)
            .map_err(|e| AppError::ExecutionFailed(e.to_string()))?;

        let params_refs: Vec<&dyn rusqlite::ToSql> = sql_params.iter().map(|b| &**b).collect();
        let total: i64 = count_stmt
            .query_row(
                rusqlite::params_from_iter(params_refs.iter().copied()),
                |r| r.get(0),
            )
            .map_err(|e| AppError::ExecutionFailed(e.to_string()))?;

        // Query entries
        let select_sql = format!(
            "SELECT id, timestamp, username, role, action, resource, ip_address, status, details
             FROM audit_logs {}
             ORDER BY id DESC
             LIMIT ? OFFSET ?",
            where_clause
        );

        let mut select_stmt = conn
            .prepare(&select_sql)
            .map_err(|e| AppError::ExecutionFailed(e.to_string()))?;

        let mut all_params_refs = params_refs;
        let limit_val = limit as i64;
        let offset_val = offset as i64;
        all_params_refs.push(&limit_val);
        all_params_refs.push(&offset_val);

        let rows = select_stmt
            .query_map(
                rusqlite::params_from_iter(all_params_refs.iter().copied()),
                |row| {
                    Ok(AuditLogEntry {
                        id: row.get(0)?,
                        timestamp: row.get(1)?,
                        username: row.get(2)?,
                        role: row.get(3)?,
                        action: row.get(4)?,
                        resource: row.get(5)?,
                        ip_address: row.get(6)?,
                        status: row.get(7)?,
                        details: row.get(8)?,
                    })
                },
            )
            .map_err(|e| AppError::ExecutionFailed(e.to_string()))?;

        let logs = rows.flatten().collect();

        let total_pages = if total == 0 {
            1
        } else {
            ((total as f64) / (limit as f64)).ceil() as usize
        };

        Ok(AuditLogPage {
            logs,
            total,
            page,
            limit,
            total_pages,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_audit_logger_flow() {
        let logger = AuditLogger::new_in_memory().unwrap();

        logger.log(
            "admin",
            "admin",
            "SERVICE_RESTART",
            Some("nginx.service"),
            "127.0.0.1",
            "SUCCESS",
            Some("Clean restart"),
        );

        logger.log(
            "viewer",
            "viewer",
            "SERVICE_RESTART",
            Some("nginx.service"),
            "192.168.1.10",
            "DENIED",
            Some("Insufficient permissions"),
        );

        let page = logger.query_logs(AuditLogQuery::default()).unwrap();
        assert_eq!(page.total, 2);
        assert_eq!(page.logs.len(), 2);
        assert_eq!(page.logs[0].status, "DENIED");
        assert_eq!(page.logs[1].status, "SUCCESS");

        // Filter by status
        let denied_page = logger
            .query_logs(AuditLogQuery {
                status: Some("DENIED".to_string()),
                ..Default::default()
            })
            .unwrap();
        assert_eq!(denied_page.total, 1);
        assert_eq!(denied_page.logs[0].username, "viewer");
    }
}
