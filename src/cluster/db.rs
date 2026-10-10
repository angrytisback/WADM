use chrono::{DateTime, Utc};
use rusqlite::{params, Connection};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::path::PathBuf;
use std::sync::Mutex;
use uuid::Uuid;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct Node {
    pub id: String,
    pub name: String,
    pub hostname: String,
    pub ip_address: String,
    pub status: String, // "online" | "offline" | "pending"
    #[serde(skip_serializing)]
    pub auth_token: String,
    pub version: String,
    pub specs: String,
    pub last_heartbeat: Option<String>,
    pub created_at: String,
}

#[allow(dead_code)]
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NodeJoinToken {
    pub token: String,
    pub expires_at: String,
    pub used: bool,
}

pub fn hash_node_token(token: &str) -> String {
    let mut hasher = Sha256::new();
    hasher.update(token.as_bytes());
    format!("{:x}", hasher.finalize())
}

pub struct ClusterDatabase {
    conn: Mutex<Connection>,
}

impl ClusterDatabase {
    pub fn new(path: PathBuf) -> Result<Self, rusqlite::Error> {
        let conn = Connection::open(&path)?;
        conn.execute_batch(
            "PRAGMA journal_mode = WAL;
             PRAGMA synchronous = NORMAL;
             CREATE TABLE IF NOT EXISTS nodes (
                 id TEXT PRIMARY KEY,
                 name TEXT NOT NULL,
                 hostname TEXT NOT NULL,
                 ip_address TEXT NOT NULL,
                 status TEXT NOT NULL DEFAULT 'pending',
                 auth_token TEXT NOT NULL,
                 version TEXT NOT NULL,
                 specs TEXT NOT NULL,
                 last_heartbeat TEXT,
                 created_at TEXT NOT NULL
             );
             CREATE INDEX IF NOT EXISTS idx_nodes_status ON nodes(status);
             CREATE INDEX IF NOT EXISTS idx_nodes_auth_token ON nodes(auth_token);

             CREATE TABLE IF NOT EXISTS node_join_tokens (
                 token TEXT PRIMARY KEY,
                 expires_at TEXT NOT NULL,
                 used INTEGER NOT NULL DEFAULT 0
             );
             CREATE INDEX IF NOT EXISTS idx_node_join_tokens_used ON node_join_tokens(used);",
        )?;

        Ok(Self {
            conn: Mutex::new(conn),
        })
    }

    #[cfg(test)]
    pub fn new_in_memory() -> Result<Self, rusqlite::Error> {
        let conn = Connection::open_in_memory()?;
        conn.execute_batch(
            "CREATE TABLE IF NOT EXISTS nodes (
                 id TEXT PRIMARY KEY,
                 name TEXT NOT NULL,
                 hostname TEXT NOT NULL,
                 ip_address TEXT NOT NULL,
                 status TEXT NOT NULL DEFAULT 'pending',
                 auth_token TEXT NOT NULL,
                 version TEXT NOT NULL,
                 specs TEXT NOT NULL,
                 last_heartbeat TEXT,
                 created_at TEXT NOT NULL
             );
             CREATE INDEX IF NOT EXISTS idx_nodes_status ON nodes(status);
             CREATE INDEX IF NOT EXISTS idx_nodes_auth_token ON nodes(auth_token);

             CREATE TABLE IF NOT EXISTS node_join_tokens (
                 token TEXT PRIMARY KEY,
                 expires_at TEXT NOT NULL,
                 used INTEGER NOT NULL DEFAULT 0
             );
             CREATE INDEX IF NOT EXISTS idx_node_join_tokens_used ON node_join_tokens(used);",
        )?;

        Ok(Self {
            conn: Mutex::new(conn),
        })
    }

    pub fn create_join_token(
        &self,
        token: &str,
        expires_at: DateTime<Utc>,
    ) -> Result<(), rusqlite::Error> {
        let conn = self.conn.lock().unwrap();
        conn.execute(
            "INSERT INTO node_join_tokens (token, expires_at, used) VALUES (?1, ?2, 0)",
            params![token, expires_at.to_rfc3339()],
        )?;
        Ok(())
    }

    pub fn validate_and_consume_join_token(&self, token: &str) -> Result<bool, rusqlite::Error> {
        let conn = self.conn.lock().unwrap();
        let mut stmt =
            conn.prepare("SELECT expires_at, used FROM node_join_tokens WHERE token = ?1")?;
        let mut rows = stmt.query(params![token])?;

        if let Some(row) = rows.next()? {
            let expires_at_str: String = row.get(0)?;
            let used: i32 = row.get(1)?;

            if used != 0 {
                return Ok(false);
            }

            if let Ok(exp) = DateTime::parse_from_rfc3339(&expires_at_str) {
                if Utc::now() > exp.with_timezone(&Utc) {
                    return Ok(false);
                }
            } else {
                return Ok(false);
            }

            conn.execute(
                "UPDATE node_join_tokens SET used = 1 WHERE token = ?1",
                params![token],
            )?;
            Ok(true)
        } else {
            Ok(false)
        }
    }

    #[allow(clippy::too_many_arguments)]
    pub fn register_or_update_node(
        &self,
        node_id: Option<&str>,
        name: &str,
        hostname: &str,
        ip_address: &str,
        auth_token_hash: &str,
        version: &str,
        specs: &str,
    ) -> Result<Node, rusqlite::Error> {
        let conn = self.conn.lock().unwrap();
        let now = Utc::now().to_rfc3339();

        let id = if let Some(existing_id) = node_id {
            existing_id.to_string()
        } else {
            Uuid::new_v4().to_string()
        };

        // Check if node exists by id
        let mut check_stmt = conn.prepare("SELECT id FROM nodes WHERE id = ?1")?;
        let exists = check_stmt.exists(params![&id])?;

        if exists {
            conn.execute(
                "UPDATE nodes SET name = ?1, hostname = ?2, ip_address = ?3, status = 'online',
                 auth_token = ?4, version = ?5, specs = ?6, last_heartbeat = ?7 WHERE id = ?8",
                params![
                    name,
                    hostname,
                    ip_address,
                    auth_token_hash,
                    version,
                    specs,
                    &now,
                    &id,
                ],
            )?;
        } else {
            conn.execute(
                "INSERT INTO nodes (id, name, hostname, ip_address, status, auth_token, version, specs, last_heartbeat, created_at)
                 VALUES (?1, ?2, ?3, ?4, 'online', ?5, ?6, ?7, ?8, ?9)",
                params![
                    &id,
                    name,
                    hostname,
                    ip_address,
                    auth_token_hash,
                    version,
                    specs,
                    &now,
                    &now,
                ],
            )?;
        }

        Ok(Node {
            id,
            name: name.to_string(),
            hostname: hostname.to_string(),
            ip_address: ip_address.to_string(),
            status: "online".to_string(),
            auth_token: auth_token_hash.to_string(),
            version: version.to_string(),
            specs: specs.to_string(),
            last_heartbeat: Some(now.clone()),
            created_at: now,
        })
    }

    pub fn get_node_by_auth_token_hash(
        &self,
        auth_token_hash: &str,
    ) -> Result<Option<Node>, rusqlite::Error> {
        let conn = self.conn.lock().unwrap();
        let mut stmt = conn.prepare(
            "SELECT id, name, hostname, ip_address, status, auth_token, version, specs, last_heartbeat, created_at
             FROM nodes WHERE auth_token = ?1",
        )?;
        let mut rows = stmt.query(params![auth_token_hash])?;

        if let Some(row) = rows.next()? {
            Ok(Some(Node {
                id: row.get(0)?,
                name: row.get(1)?,
                hostname: row.get(2)?,
                ip_address: row.get(3)?,
                status: row.get(4)?,
                auth_token: row.get(5)?,
                version: row.get(6)?,
                specs: row.get(7)?,
                last_heartbeat: row.get(8)?,
                created_at: row.get(9)?,
            }))
        } else {
            Ok(None)
        }
    }

    pub fn get_node(&self, id: &str) -> Result<Option<Node>, rusqlite::Error> {
        let conn = self.conn.lock().unwrap();
        let mut stmt = conn.prepare(
            "SELECT id, name, hostname, ip_address, status, auth_token, version, specs, last_heartbeat, created_at
             FROM nodes WHERE id = ?1",
        )?;
        let mut rows = stmt.query(params![id])?;

        if let Some(row) = rows.next()? {
            Ok(Some(Node {
                id: row.get(0)?,
                name: row.get(1)?,
                hostname: row.get(2)?,
                ip_address: row.get(3)?,
                status: row.get(4)?,
                auth_token: row.get(5)?,
                version: row.get(6)?,
                specs: row.get(7)?,
                last_heartbeat: row.get(8)?,
                created_at: row.get(9)?,
            }))
        } else {
            Ok(None)
        }
    }

    pub fn list_nodes(&self) -> Result<Vec<Node>, rusqlite::Error> {
        let conn = self.conn.lock().unwrap();
        let mut stmt = conn.prepare(
            "SELECT id, name, hostname, ip_address, status, auth_token, version, specs, last_heartbeat, created_at
             FROM nodes ORDER BY created_at ASC",
        )?;

        let rows = stmt.query_map([], |row| {
            Ok(Node {
                id: row.get(0)?,
                name: row.get(1)?,
                hostname: row.get(2)?,
                ip_address: row.get(3)?,
                status: row.get(4)?,
                auth_token: row.get(5)?,
                version: row.get(6)?,
                specs: row.get(7)?,
                last_heartbeat: row.get(8)?,
                created_at: row.get(9)?,
            })
        })?;

        let mut nodes = Vec::new();
        for node in rows {
            nodes.push(node?);
        }
        Ok(nodes)
    }

    pub fn update_heartbeat(
        &self,
        id: &str,
        specs: Option<&str>,
        version: Option<&str>,
    ) -> Result<(), rusqlite::Error> {
        let conn = self.conn.lock().unwrap();
        let now = Utc::now().to_rfc3339();

        if let (Some(s), Some(v)) = (specs, version) {
            conn.execute(
                "UPDATE nodes SET status = 'online', last_heartbeat = ?1, specs = ?2, version = ?3 WHERE id = ?4",
                params![&now, s, v, id],
            )?;
        } else if let Some(s) = specs {
            conn.execute(
                "UPDATE nodes SET status = 'online', last_heartbeat = ?1, specs = ?2 WHERE id = ?3",
                params![&now, s, id],
            )?;
        } else if let Some(v) = version {
            conn.execute(
                "UPDATE nodes SET status = 'online', last_heartbeat = ?1, version = ?2 WHERE id = ?3",
                params![&now, v, id],
            )?;
        } else {
            conn.execute(
                "UPDATE nodes SET status = 'online', last_heartbeat = ?1 WHERE id = ?2",
                params![&now, id],
            )?;
        }
        Ok(())
    }

    pub fn update_status(&self, id: &str, status: &str) -> Result<(), rusqlite::Error> {
        let conn = self.conn.lock().unwrap();
        conn.execute(
            "UPDATE nodes SET status = ?1 WHERE id = ?2",
            params![status, id],
        )?;
        Ok(())
    }

    pub fn mark_stale_nodes_offline(
        &self,
        threshold_seconds: i64,
    ) -> Result<Vec<String>, rusqlite::Error> {
        let conn = self.conn.lock().unwrap();
        let mut stmt =
            conn.prepare("SELECT id, last_heartbeat FROM nodes WHERE status = 'online'")?;
        let rows = stmt.query_map([], |row| {
            let id: String = row.get(0)?;
            let last_hb: Option<String> = row.get(1)?;
            Ok((id, last_hb))
        })?;

        let now = Utc::now();
        let mut stale_ids = Vec::new();

        for item in rows {
            let (id, last_hb_opt) = item?;
            let is_stale = match last_hb_opt {
                Some(hb_str) => match DateTime::parse_from_rfc3339(&hb_str) {
                    Ok(parsed) => {
                        let duration = now.signed_duration_since(parsed.with_timezone(&Utc));
                        duration.num_seconds() > threshold_seconds
                    }
                    Err(_) => true,
                },
                None => true,
            };

            if is_stale {
                stale_ids.push(id);
            }
        }

        for id in &stale_ids {
            conn.execute(
                "UPDATE nodes SET status = 'offline' WHERE id = ?1",
                params![id],
            )?;
        }

        Ok(stale_ids)
    }

    pub fn delete_node(&self, id: &str) -> Result<bool, rusqlite::Error> {
        let conn = self.conn.lock().unwrap();
        let count = conn.execute("DELETE FROM nodes WHERE id = ?1", params![id])?;
        Ok(count > 0)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::Duration;

    #[test]
    fn test_join_token_lifecycle() {
        let db = ClusterDatabase::new_in_memory().unwrap();
        let token = "test-token-123";

        // Create valid token
        let exp = Utc::now() + Duration::hours(1);
        db.create_join_token(token, exp).unwrap();

        // Validate and consume once -> true
        assert!(db.validate_and_consume_join_token(token).unwrap());

        // Validate second time -> false (already used)
        assert!(!db.validate_and_consume_join_token(token).unwrap());

        // Test expired token
        let expired_token = "expired-token-456";
        let past_exp = Utc::now() - Duration::hours(1);
        db.create_join_token(expired_token, past_exp).unwrap();
        assert!(!db.validate_and_consume_join_token(expired_token).unwrap());
    }

    #[test]
    fn test_node_registration_and_queries() {
        let db = ClusterDatabase::new_in_memory().unwrap();
        let token = "node-secret-abc";
        let token_hash = hash_node_token(token);

        let node = db
            .register_or_update_node(
                None,
                "worker-fra-01",
                "fra-srv",
                "192.168.1.100",
                &token_hash,
                "0.96.0",
                r#"{"cpu_cores":4}"#,
            )
            .unwrap();

        assert_eq!(node.name, "worker-fra-01");
        assert_eq!(node.status, "online");

        // Query by id
        let fetched = db.get_node(&node.id).unwrap().expect("Node not found");
        assert_eq!(fetched.id, node.id);

        // Query by token hash
        let by_token = db
            .get_node_by_auth_token_hash(&token_hash)
            .unwrap()
            .expect("Node by token not found");
        assert_eq!(by_token.id, node.id);

        // List nodes
        let list = db.list_nodes().unwrap();
        assert_eq!(list.len(), 1);

        // Update heartbeat
        db.update_heartbeat(&node.id, Some(r#"{"cpu_cores":8}"#), None)
            .unwrap();
        let updated = db.get_node(&node.id).unwrap().unwrap();
        assert_eq!(updated.specs, r#"{"cpu_cores":8}"#);

        // Delete node
        assert!(db.delete_node(&node.id).unwrap());
        assert!(db.get_node(&node.id).unwrap().is_none());
    }

    #[test]
    fn test_stale_node_marking() {
        let db = ClusterDatabase::new_in_memory().unwrap();
        let token_hash = hash_node_token("secret");

        let node = db
            .register_or_update_node(
                None,
                "stale-node",
                "stale-host",
                "10.0.0.1",
                &token_hash,
                "0.96.0",
                "{}",
            )
            .unwrap();

        // Initially online
        assert_eq!(node.status, "online");

        // Manually set last_heartbeat to 30 seconds ago
        let old_time = (Utc::now() - Duration::seconds(30)).to_rfc3339();
        {
            let conn = db.conn.lock().unwrap();
            conn.execute(
                "UPDATE nodes SET last_heartbeat = ?1 WHERE id = ?2",
                params![&old_time, &node.id],
            )
            .unwrap();
        }

        // Mark stale nodes with 15 second threshold
        let stale = db.mark_stale_nodes_offline(15).unwrap();
        assert_eq!(stale, vec![node.id.clone()]);

        let refreshed = db.get_node(&node.id).unwrap().unwrap();
        assert_eq!(refreshed.status, "offline");
    }
}
