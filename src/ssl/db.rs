use chrono::Utc;
use rusqlite::{params, Connection};
use serde::{Deserialize, Serialize};
use std::path::PathBuf;
use std::sync::Mutex;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SslConfig {
    pub enabled: bool,
    pub mode: String, // "self_signed", "custom_cert", "lets_encrypt"
    pub domain: Option<String>,
    pub email: Option<String>,
    pub cert_path: Option<String>,
    pub key_path: Option<String>,
    pub auto_renew: bool,
    pub force_https: bool,
    pub https_port: u16,
    pub http_port: u16,
    pub expires_at: Option<i64>,
    pub issuer: Option<String>,
    pub updated_at: String,
}

impl Default for SslConfig {
    fn default() -> Self {
        Self {
            enabled: false,
            mode: "self_signed".to_string(),
            domain: None,
            email: None,
            cert_path: None,
            key_path: None,
            auto_renew: true,
            force_https: false,
            https_port: 8168,
            http_port: 80,
            expires_at: None,
            issuer: None,
            updated_at: Utc::now().to_rfc3339(),
        }
    }
}

#[derive(Debug, Clone)]
pub struct CertMetaUpdate<'a> {
    pub mode: &'a str,
    pub domain: Option<&'a str>,
    pub email: Option<&'a str>,
    pub cert_path: &'a str,
    pub key_path: &'a str,
    pub issuer: &'a str,
    pub expires_at: Option<i64>,
}

pub struct SslDatabase {
    conn: Mutex<Connection>,
}

impl SslDatabase {
    pub fn new(db_path: PathBuf) -> Result<Self, rusqlite::Error> {
        let conn = Connection::open(&db_path)?;
        conn.execute_batch(
            "PRAGMA journal_mode = WAL;
             PRAGMA synchronous = NORMAL;
             CREATE TABLE IF NOT EXISTS ssl_config (
                 id INTEGER PRIMARY KEY CHECK (id = 1),
                 enabled BOOLEAN NOT NULL DEFAULT 0,
                 mode TEXT NOT NULL DEFAULT 'self_signed',
                 domain TEXT,
                 email TEXT,
                 cert_path TEXT,
                 key_path TEXT,
                 auto_renew BOOLEAN NOT NULL DEFAULT 1,
                 force_https BOOLEAN NOT NULL DEFAULT 0,
                 https_port INTEGER NOT NULL DEFAULT 8168,
                 http_port INTEGER NOT NULL DEFAULT 80,
                 expires_at INTEGER,
                 issuer TEXT,
                 updated_at TEXT NOT NULL
             );
             INSERT OR IGNORE INTO ssl_config (id, enabled, mode, auto_renew, force_https, https_port, http_port, updated_at)
             VALUES (1, 0, 'self_signed', 1, 0, 8168, 80, datetime('now'));",
        )?;
        Ok(Self {
            conn: Mutex::new(conn),
        })
    }

    #[cfg(test)]
    pub fn new_in_memory() -> Result<Self, rusqlite::Error> {
        let conn = Connection::open_in_memory()?;
        conn.execute_batch(
            "CREATE TABLE IF NOT EXISTS ssl_config (
                 id INTEGER PRIMARY KEY CHECK (id = 1),
                 enabled BOOLEAN NOT NULL DEFAULT 0,
                 mode TEXT NOT NULL DEFAULT 'self_signed',
                 domain TEXT,
                 email TEXT,
                 cert_path TEXT,
                 key_path TEXT,
                 auto_renew BOOLEAN NOT NULL DEFAULT 1,
                 force_https BOOLEAN NOT NULL DEFAULT 0,
                 https_port INTEGER NOT NULL DEFAULT 8168,
                 http_port INTEGER NOT NULL DEFAULT 80,
                 expires_at INTEGER,
                 issuer TEXT,
                 updated_at TEXT NOT NULL
             );
             INSERT OR IGNORE INTO ssl_config (id, enabled, mode, auto_renew, force_https, https_port, http_port, updated_at)
             VALUES (1, 0, 'self_signed', 1, 0, 8168, 80, datetime('now'));",
        )?;
        Ok(Self {
            conn: Mutex::new(conn),
        })
    }

    pub fn get_config(&self) -> Result<SslConfig, rusqlite::Error> {
        let conn = self.conn.lock().unwrap();
        let mut stmt = conn.prepare(
            "SELECT enabled, mode, domain, email, cert_path, key_path, auto_renew, force_https,
                    https_port, http_port, expires_at, issuer, updated_at
             FROM ssl_config WHERE id = 1",
        )?;

        let mut rows = stmt.query([])?;
        if let Some(row) = rows.next()? {
            let enabled_int: i32 = row.get(0)?;
            let auto_renew_int: i32 = row.get(6)?;
            let force_https_int: i32 = row.get(7)?;
            let https_port: u16 = row.get(8)?;
            let http_port: u16 = row.get(9)?;

            Ok(SslConfig {
                enabled: enabled_int != 0,
                mode: row.get(1)?,
                domain: row.get(2)?,
                email: row.get(3)?,
                cert_path: row.get(4)?,
                key_path: row.get(5)?,
                auto_renew: auto_renew_int != 0,
                force_https: force_https_int != 0,
                https_port,
                http_port,
                expires_at: row.get(10)?,
                issuer: row.get(11)?,
                updated_at: row.get(12)?,
            })
        } else {
            Ok(SslConfig::default())
        }
    }

    pub fn update_certificate(&self, update: &CertMetaUpdate<'_>) -> Result<(), rusqlite::Error> {
        let now = Utc::now().to_rfc3339();
        let conn = self.conn.lock().unwrap();
        conn.execute(
            "UPDATE ssl_config SET
                mode = ?1,
                domain = ?2,
                email = ?3,
                cert_path = ?4,
                key_path = ?5,
                issuer = ?6,
                expires_at = ?7,
                updated_at = ?8
             WHERE id = 1",
            params![
                update.mode,
                update.domain,
                update.email,
                update.cert_path,
                update.key_path,
                update.issuer,
                update.expires_at,
                now
            ],
        )?;
        Ok(())
    }

    pub fn set_enabled(&self, enabled: bool, force_https: bool) -> Result<(), rusqlite::Error> {
        let now = Utc::now().to_rfc3339();
        let conn = self.conn.lock().unwrap();
        conn.execute(
            "UPDATE ssl_config SET
                enabled = ?1,
                force_https = ?2,
                updated_at = ?3
             WHERE id = 1",
            params![
                if enabled { 1 } else { 0 },
                if force_https { 1 } else { 0 },
                now
            ],
        )?;
        Ok(())
    }

    pub fn set_auto_renew(&self, auto_renew: bool) -> Result<(), rusqlite::Error> {
        let now = Utc::now().to_rfc3339();
        let conn = self.conn.lock().unwrap();
        conn.execute(
            "UPDATE ssl_config SET
                auto_renew = ?1,
                updated_at = ?2
             WHERE id = 1",
            params![if auto_renew { 1 } else { 0 }, now],
        )?;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_ssl_database_crud() {
        let db = SslDatabase::new_in_memory().unwrap();
        let config = db.get_config().unwrap();
        assert!(!config.enabled);
        assert_eq!(config.mode, "self_signed");
        assert_eq!(config.https_port, 8168);

        db.update_certificate(&CertMetaUpdate {
            mode: "self_signed",
            domain: Some("panel.local"),
            email: None,
            cert_path: "/etc/cert.pem",
            key_path: "/etc/key.pem",
            issuer: "WADM Self-Signed CA",
            expires_at: Some(1893456000),
        })
        .unwrap();

        db.set_enabled(true, true).unwrap();

        let updated = db.get_config().unwrap();
        assert!(updated.enabled);
        assert!(updated.force_https);
        assert_eq!(updated.domain.as_deref(), Some("panel.local"));
        assert_eq!(updated.cert_path.as_deref(), Some("/etc/cert.pem"));
        assert_eq!(updated.issuer.as_deref(), Some("WADM Self-Signed CA"));
    }
}
