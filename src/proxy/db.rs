use rusqlite::{params, Connection};
use serde::{Deserialize, Serialize};
use std::path::PathBuf;
use std::sync::Mutex;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct ProxyRoute {
    pub id: String,
    pub app_id: String,
    #[serde(default)]
    pub domain: Option<String>,
    #[serde(default)]
    pub path_prefix: Option<String>,
    pub target_url: String, // e.g. "http://127.0.0.1:18080"
    #[serde(default = "default_true")]
    pub websocket_support: bool,
    pub created_at: String,
}

fn default_true() -> bool {
    true
}

pub struct ProxyDatabase {
    conn: Mutex<Connection>,
}

impl ProxyDatabase {
    pub fn new(path: PathBuf) -> Result<Self, rusqlite::Error> {
        let conn = Connection::open(&path)?;
        conn.execute_batch(
            "PRAGMA journal_mode = WAL;
             PRAGMA synchronous = NORMAL;
             CREATE TABLE IF NOT EXISTS proxy_routes (
                 id TEXT PRIMARY KEY,
                 app_id TEXT NOT NULL,
                 domain TEXT,
                 path_prefix TEXT,
                 target_url TEXT NOT NULL,
                 websocket_support INTEGER NOT NULL DEFAULT 1,
                 created_at TEXT NOT NULL
             );
             CREATE INDEX IF NOT EXISTS idx_proxy_routes_app_id ON proxy_routes(app_id);
             CREATE INDEX IF NOT EXISTS idx_proxy_routes_domain ON proxy_routes(domain);
             CREATE INDEX IF NOT EXISTS idx_proxy_routes_path_prefix ON proxy_routes(path_prefix);",
        )?;
        Ok(Self {
            conn: Mutex::new(conn),
        })
    }

    #[cfg(test)]
    pub fn new_in_memory() -> Result<Self, rusqlite::Error> {
        let conn = Connection::open_in_memory()?;
        conn.execute_batch(
            "CREATE TABLE IF NOT EXISTS proxy_routes (
                 id TEXT PRIMARY KEY,
                 app_id TEXT NOT NULL,
                 domain TEXT,
                 path_prefix TEXT,
                 target_url TEXT NOT NULL,
                 websocket_support INTEGER NOT NULL DEFAULT 1,
                 created_at TEXT NOT NULL
             );",
        )?;
        Ok(Self {
            conn: Mutex::new(conn),
        })
    }

    pub fn create_or_update_route(&self, route: &ProxyRoute) -> Result<(), rusqlite::Error> {
        let conn = self.conn.lock().unwrap();
        // Remove prior route for this app_id if replacing
        conn.execute(
            "DELETE FROM proxy_routes WHERE app_id = ?1",
            params![route.app_id],
        )?;
        conn.execute(
            "INSERT INTO proxy_routes (id, app_id, domain, path_prefix, target_url, websocket_support, created_at)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7)",
            params![
                route.id,
                route.app_id,
                route.domain,
                route.path_prefix,
                route.target_url,
                if route.websocket_support { 1 } else { 0 },
                route.created_at
            ],
        )?;
        Ok(())
    }

    pub fn get_route_by_app_id(&self, app_id: &str) -> Result<Option<ProxyRoute>, rusqlite::Error> {
        let conn = self.conn.lock().unwrap();
        let mut stmt = conn.prepare(
            "SELECT id, app_id, domain, path_prefix, target_url, websocket_support, created_at
             FROM proxy_routes WHERE app_id = ?1 LIMIT 1",
        )?;
        let mut rows = stmt.query(params![app_id])?;
        if let Some(row) = rows.next()? {
            let ws_int: i32 = row.get(5)?;
            Ok(Some(ProxyRoute {
                id: row.get(0)?,
                app_id: row.get(1)?,
                domain: row.get(2)?,
                path_prefix: row.get(3)?,
                target_url: row.get(4)?,
                websocket_support: ws_int != 0,
                created_at: row.get(6)?,
            }))
        } else {
            Ok(None)
        }
    }

    pub fn get_route_by_domain(&self, domain: &str) -> Result<Option<ProxyRoute>, rusqlite::Error> {
        let conn = self.conn.lock().unwrap();
        let mut stmt = conn.prepare(
            "SELECT id, app_id, domain, path_prefix, target_url, websocket_support, created_at
             FROM proxy_routes WHERE LOWER(domain) = LOWER(?1) LIMIT 1",
        )?;
        let mut rows = stmt.query(params![domain])?;
        if let Some(row) = rows.next()? {
            let ws_int: i32 = row.get(5)?;
            Ok(Some(ProxyRoute {
                id: row.get(0)?,
                app_id: row.get(1)?,
                domain: row.get(2)?,
                path_prefix: row.get(3)?,
                target_url: row.get(4)?,
                websocket_support: ws_int != 0,
                created_at: row.get(6)?,
            }))
        } else {
            Ok(None)
        }
    }

    #[allow(dead_code)]
    pub fn get_route_by_path_prefix(
        &self,
        prefix: &str,
    ) -> Result<Option<ProxyRoute>, rusqlite::Error> {
        let conn = self.conn.lock().unwrap();
        let mut stmt = conn.prepare(
            "SELECT id, app_id, domain, path_prefix, target_url, websocket_support, created_at
             FROM proxy_routes WHERE path_prefix = ?1 LIMIT 1",
        )?;
        let mut rows = stmt.query(params![prefix])?;
        if let Some(row) = rows.next()? {
            let ws_int: i32 = row.get(5)?;
            Ok(Some(ProxyRoute {
                id: row.get(0)?,
                app_id: row.get(1)?,
                domain: row.get(2)?,
                path_prefix: row.get(3)?,
                target_url: row.get(4)?,
                websocket_support: ws_int != 0,
                created_at: row.get(6)?,
            }))
        } else {
            Ok(None)
        }
    }

    pub fn list_routes(&self) -> Result<Vec<ProxyRoute>, rusqlite::Error> {
        let conn = self.conn.lock().unwrap();
        let mut stmt = conn.prepare(
            "SELECT id, app_id, domain, path_prefix, target_url, websocket_support, created_at
             FROM proxy_routes ORDER BY created_at DESC",
        )?;
        let rows = stmt.query_map([], |row| {
            let ws_int: i32 = row.get(5)?;
            Ok(ProxyRoute {
                id: row.get(0)?,
                app_id: row.get(1)?,
                domain: row.get(2)?,
                path_prefix: row.get(3)?,
                target_url: row.get(4)?,
                websocket_support: ws_int != 0,
                created_at: row.get(6)?,
            })
        })?;

        let mut list = Vec::new();
        for r in rows {
            list.push(r?);
        }
        Ok(list)
    }

    pub fn delete_route_by_app_id(&self, app_id: &str) -> Result<bool, rusqlite::Error> {
        let conn = self.conn.lock().unwrap();
        let affected = conn.execute(
            "DELETE FROM proxy_routes WHERE app_id = ?1",
            params![app_id],
        )?;
        Ok(affected > 0)
    }

    #[allow(dead_code)]
    pub fn delete_route_by_id(&self, id: &str) -> Result<bool, rusqlite::Error> {
        let conn = self.conn.lock().unwrap();
        let affected = conn.execute("DELETE FROM proxy_routes WHERE id = ?1", params![id])?;
        Ok(affected > 0)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_proxy_database_crud() {
        let db = ProxyDatabase::new_in_memory().expect("Failed to initialize in-memory DB");

        let route = ProxyRoute {
            id: "route-1".to_string(),
            app_id: "nextcloud".to_string(),
            domain: Some("cloud.example.com".to_string()),
            path_prefix: Some("/apps/nextcloud".to_string()),
            target_url: "http://127.0.0.1:18080".to_string(),
            websocket_support: true,
            created_at: "2026-10-10T20:00:00Z".to_string(),
        };

        db.create_or_update_route(&route)
            .expect("Insert route failed");

        let fetched_app = db.get_route_by_app_id("nextcloud").expect("Fetch failed");
        assert!(fetched_app.is_some());
        assert_eq!(fetched_app.unwrap().target_url, "http://127.0.0.1:18080");

        let fetched_domain = db
            .get_route_by_domain("cloud.example.com")
            .expect("Fetch domain failed");
        assert!(fetched_domain.is_some());
        assert_eq!(fetched_domain.unwrap().app_id, "nextcloud");

        let fetched_prefix = db
            .get_route_by_path_prefix("/apps/nextcloud")
            .expect("Fetch prefix failed");
        assert!(fetched_prefix.is_some());

        let all = db.list_routes().expect("List failed");
        assert_eq!(all.len(), 1);

        let deleted = db
            .delete_route_by_app_id("nextcloud")
            .expect("Delete failed");
        assert!(deleted);

        let after_delete = db.get_route_by_app_id("nextcloud").expect("Fetch failed");
        assert!(after_delete.is_none());
    }
}
