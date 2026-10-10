use argon2::{
    password_hash::{rand_core::OsRng, PasswordHash, PasswordHasher, PasswordVerifier, SaltString},
    Argon2,
};
use chrono::Utc;
use rusqlite::{params, Connection};
use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::{Mutex, RwLock};

use super::models::{User, UserRole};
use crate::drivers::error::AppError;

pub struct UserDatabase {
    conn: Mutex<Connection>,
    revoked_tokens_cache: RwLock<HashMap<String, i64>>,
    user_revocations_cache: RwLock<HashMap<String, i64>>,
}

impl UserDatabase {
    pub fn new(path: PathBuf) -> Result<Self, rusqlite::Error> {
        let conn = Connection::open(&path)?;
        conn.execute_batch(
            "PRAGMA journal_mode = WAL;
             PRAGMA synchronous = NORMAL;
             CREATE TABLE IF NOT EXISTS users (
                 id INTEGER PRIMARY KEY AUTOINCREMENT,
                 username TEXT UNIQUE NOT NULL,
                 password_hash TEXT NOT NULL,
                 role TEXT NOT NULL,
                 created_at TEXT NOT NULL
             );
             CREATE INDEX IF NOT EXISTS idx_users_username ON users(username);

             CREATE TABLE IF NOT EXISTS revoked_tokens (
                 token_hash TEXT PRIMARY KEY,
                 jti TEXT,
                 username TEXT NOT NULL,
                 expires_at INTEGER NOT NULL,
                 revoked_at TEXT NOT NULL
             );
             CREATE INDEX IF NOT EXISTS idx_revoked_tokens_expires ON revoked_tokens(expires_at);
             CREATE INDEX IF NOT EXISTS idx_revoked_tokens_username ON revoked_tokens(username);

             CREATE TABLE IF NOT EXISTS user_token_revocations (
                 username TEXT PRIMARY KEY,
                 revoked_before INTEGER NOT NULL
             );",
        )?;

        let db = Self {
            conn: Mutex::new(conn),
            revoked_tokens_cache: RwLock::new(HashMap::new()),
            user_revocations_cache: RwLock::new(HashMap::new()),
        };
        db.auto_migrate_legacy_auth();
        db.init_revocation_cache();
        Ok(db)
    }

    #[cfg(test)]
    pub fn new_in_memory() -> Result<Self, rusqlite::Error> {
        let conn = Connection::open_in_memory()?;
        conn.execute_batch(
            "CREATE TABLE IF NOT EXISTS users (
                 id INTEGER PRIMARY KEY AUTOINCREMENT,
                 username TEXT UNIQUE NOT NULL,
                 password_hash TEXT NOT NULL,
                 role TEXT NOT NULL,
                 created_at TEXT NOT NULL
             );
             CREATE INDEX IF NOT EXISTS idx_users_username ON users(username);

             CREATE TABLE IF NOT EXISTS revoked_tokens (
                 token_hash TEXT PRIMARY KEY,
                 jti TEXT,
                 username TEXT NOT NULL,
                 expires_at INTEGER NOT NULL,
                 revoked_at TEXT NOT NULL
             );
             CREATE INDEX IF NOT EXISTS idx_revoked_tokens_expires ON revoked_tokens(expires_at);
             CREATE INDEX IF NOT EXISTS idx_revoked_tokens_username ON revoked_tokens(username);

             CREATE TABLE IF NOT EXISTS user_token_revocations (
                 username TEXT PRIMARY KEY,
                 revoked_before INTEGER NOT NULL
             );",
        )?;

        let db = Self {
            conn: Mutex::new(conn),
            revoked_tokens_cache: RwLock::new(HashMap::new()),
            user_revocations_cache: RwLock::new(HashMap::new()),
        };
        db.init_revocation_cache();
        Ok(db)
    }

    fn init_revocation_cache(&self) {
        let now = Utc::now().timestamp();
        let conn = self.conn.lock().unwrap();

        // 1. Load active revoked tokens
        if let Ok(mut stmt) =
            conn.prepare("SELECT token_hash, expires_at FROM revoked_tokens WHERE expires_at > ?1")
        {
            if let Ok(rows) = stmt.query_map(params![now], |row| {
                let hash: String = row.get(0)?;
                let exp: i64 = row.get(1)?;
                Ok((hash, exp))
            }) {
                let mut cache = self.revoked_tokens_cache.write().unwrap();
                for item in rows.flatten() {
                    cache.insert(item.0, item.1);
                }
            }
        };

        // 2. Load user token revocations
        if let Ok(mut stmt) =
            conn.prepare("SELECT username, revoked_before FROM user_token_revocations")
        {
            if let Ok(rows) = stmt.query_map([], |row| {
                let uname: String = row.get(0)?;
                let before: i64 = row.get(1)?;
                Ok((uname, before))
            }) {
                let mut cache = self.user_revocations_cache.write().unwrap();
                for item in rows.flatten() {
                    cache.insert(item.0, item.1);
                }
            }
        };
    }

    fn auto_migrate_legacy_auth(&self) {
        let count: i64 = {
            let conn = self.conn.lock().unwrap();
            conn.query_row("SELECT COUNT(*) FROM users", [], |row| row.get(0))
                .unwrap_or(0)
        };

        if count == 0 {
            // Check legacy wadm-auth.json
            if let Some(store) = crate::api::auth::load_auth_store() {
                if !store.password_hash.is_empty() {
                    let now = Utc::now().to_rfc3339();
                    let conn = self.conn.lock().unwrap();
                    let _ = conn.execute(
                        "INSERT OR IGNORE INTO users (username, password_hash, role, created_at)
                         VALUES (?1, ?2, ?3, ?4)",
                        params!["admin", store.password_hash, UserRole::Admin.as_str(), now],
                    );
                    log::info!("Migrated legacy admin user from wadm-auth.json to users table");
                }
            }
        }
    }

    pub fn authenticate(&self, username: &str, password: &str) -> Result<User, AppError> {
        let (user, password_hash) = {
            let conn = self.conn.lock().unwrap();
            let mut stmt = conn
                .prepare("SELECT id, username, password_hash, role, created_at FROM users WHERE username = ?1")
                .map_err(|e| AppError::ExecutionFailed(e.to_string()))?;

            let res = stmt.query_row(params![username], |row| {
                let id: i64 = row.get(0)?;
                let uname: String = row.get(1)?;
                let hash: String = row.get(2)?;
                let role_str: String = row.get(3)?;
                let created_at: String = row.get(4)?;
                let role = UserRole::from_str(&role_str).unwrap_or(UserRole::Viewer);
                Ok((
                    User {
                        id,
                        username: uname,
                        role,
                        created_at,
                    },
                    hash,
                ))
            });

            match res {
                Ok(data) => data,
                Err(rusqlite::Error::QueryReturnedNoRows) => {
                    return Err(AppError::Unauthorized("Invalid credentials".to_string()));
                }
                Err(e) => return Err(AppError::ExecutionFailed(e.to_string())),
            }
        };

        let parsed_hash = PasswordHash::new(&password_hash).map_err(|_| {
            AppError::ExecutionFailed("Invalid password hash in database".to_string())
        })?;

        Argon2::default()
            .verify_password(password.as_bytes(), &parsed_hash)
            .map_err(|_| AppError::Unauthorized("Invalid credentials".to_string()))?;

        Ok(user)
    }

    pub fn create_user(
        &self,
        username: &str,
        password: &str,
        role: UserRole,
    ) -> Result<User, AppError> {
        let username_clean = username.trim();
        if username_clean.len() < 3 || username_clean.len() > 32 {
            return Err(AppError::InvalidInput(
                "Username must be between 3 and 32 characters".to_string(),
            ));
        }

        if !username_clean
            .chars()
            .all(|c| c.is_alphanumeric() || c == '_' || c == '-')
        {
            return Err(AppError::InvalidInput(
                "Username may only contain alphanumeric characters, underscores and hyphens"
                    .to_string(),
            ));
        }

        if password.len() < 6 {
            return Err(AppError::InvalidInput(
                "Password must be at least 6 characters long".to_string(),
            ));
        }

        let salt = SaltString::generate(&mut OsRng);
        let password_hash = Argon2::default()
            .hash_password(password.as_bytes(), &salt)
            .map_err(|e| AppError::ExecutionFailed(format!("Password hashing failed: {}", e)))?
            .to_string();

        let now = Utc::now().to_rfc3339();
        let conn = self.conn.lock().unwrap();

        let res = conn.execute(
            "INSERT INTO users (username, password_hash, role, created_at)
             VALUES (?1, ?2, ?3, ?4)",
            params![username_clean, password_hash, role.as_str(), now],
        );

        match res {
            Ok(_) => {
                let id = conn.last_insert_rowid();
                Ok(User {
                    id,
                    username: username_clean.to_string(),
                    role,
                    created_at: now,
                })
            }
            Err(rusqlite::Error::SqliteFailure(err, _))
                if err.code == rusqlite::ErrorCode::ConstraintViolation =>
            {
                Err(AppError::InvalidInput(format!(
                    "Username '{}' already exists",
                    username_clean
                )))
            }
            Err(e) => Err(AppError::ExecutionFailed(e.to_string())),
        }
    }

    pub fn list_users(&self) -> Result<Vec<User>, AppError> {
        let conn = self.conn.lock().unwrap();
        let mut stmt = conn
            .prepare("SELECT id, username, role, created_at FROM users ORDER BY id ASC")
            .map_err(|e| AppError::ExecutionFailed(e.to_string()))?;

        let rows = stmt
            .query_map([], |row| {
                let id: i64 = row.get(0)?;
                let username: String = row.get(1)?;
                let role_str: String = row.get(2)?;
                let created_at: String = row.get(3)?;
                let role = UserRole::from_str(&role_str).unwrap_or(UserRole::Viewer);
                Ok(User {
                    id,
                    username,
                    role,
                    created_at,
                })
            })
            .map_err(|e| AppError::ExecutionFailed(e.to_string()))?;

        let users = rows.flatten().collect();
        Ok(users)
    }

    #[allow(dead_code)]
    pub fn get_user_by_username(&self, username: &str) -> Result<Option<User>, AppError> {
        let conn = self.conn.lock().unwrap();
        let mut stmt = conn
            .prepare("SELECT id, username, role, created_at FROM users WHERE username = ?1")
            .map_err(|e| AppError::ExecutionFailed(e.to_string()))?;

        let mut rows = stmt
            .query_map(params![username], |row| {
                let id: i64 = row.get(0)?;
                let uname: String = row.get(1)?;
                let role_str: String = row.get(2)?;
                let created_at: String = row.get(3)?;
                let role = UserRole::from_str(&role_str).unwrap_or(UserRole::Viewer);
                Ok(User {
                    id,
                    username: uname,
                    role,
                    created_at,
                })
            })
            .map_err(|e| AppError::ExecutionFailed(e.to_string()))?;

        if let Some(r) = rows.next() {
            Ok(Some(
                r.map_err(|e| AppError::ExecutionFailed(e.to_string()))?,
            ))
        } else {
            Ok(None)
        }
    }

    #[allow(dead_code)]
    pub fn count_admins(&self) -> Result<usize, AppError> {
        let conn = self.conn.lock().unwrap();
        let count: i64 = conn
            .query_row(
                "SELECT COUNT(*) FROM users WHERE role = 'admin'",
                [],
                |row| row.get(0),
            )
            .map_err(|e| AppError::ExecutionFailed(e.to_string()))?;
        Ok(count as usize)
    }

    pub fn update_role(&self, id: i64, new_role: UserRole) -> Result<(), AppError> {
        let conn = self.conn.lock().unwrap();

        // Check if modifying an admin
        let current_role_str: String = conn
            .query_row("SELECT role FROM users WHERE id = ?1", params![id], |row| {
                row.get(0)
            })
            .map_err(|_| AppError::NotFound(format!("User with id {} not found", id)))?;

        if current_role_str == "admin" && new_role != UserRole::Admin {
            let admin_count: i64 = conn
                .query_row(
                    "SELECT COUNT(*) FROM users WHERE role = 'admin'",
                    [],
                    |row| row.get(0),
                )
                .map_err(|e| AppError::ExecutionFailed(e.to_string()))?;

            if admin_count <= 1 {
                return Err(AppError::InvalidInput(
                    "Cannot demote the only remaining administrator".to_string(),
                ));
            }
        }

        conn.execute(
            "UPDATE users SET role = ?1 WHERE id = ?2",
            params![new_role.as_str(), id],
        )
        .map_err(|e| AppError::ExecutionFailed(e.to_string()))?;

        Ok(())
    }

    pub fn update_password(&self, id: i64, new_password: &str) -> Result<(), AppError> {
        if new_password.len() < 6 {
            return Err(AppError::InvalidInput(
                "Password must be at least 6 characters long".to_string(),
            ));
        }

        let username: String = {
            let conn = self.conn.lock().unwrap();
            conn.query_row(
                "SELECT username FROM users WHERE id = ?1",
                params![id],
                |row| row.get(0),
            )
            .map_err(|_| AppError::NotFound(format!("User with id {} not found", id)))?
        };

        let salt = SaltString::generate(&mut OsRng);
        let password_hash = Argon2::default()
            .hash_password(new_password.as_bytes(), &salt)
            .map_err(|e| AppError::ExecutionFailed(format!("Password hashing failed: {}", e)))?
            .to_string();

        {
            let conn = self.conn.lock().unwrap();
            let rows = conn
                .execute(
                    "UPDATE users SET password_hash = ?1 WHERE id = ?2",
                    params![password_hash, id],
                )
                .map_err(|e| AppError::ExecutionFailed(e.to_string()))?;

            if rows == 0 {
                return Err(AppError::NotFound(format!("User with id {} not found", id)));
            }
        }

        // Invalidate all previously issued tokens for this user
        self.revoke_all_for_user(&username)?;

        Ok(())
    }

    pub fn delete_user(&self, id: i64) -> Result<(), AppError> {
        let (username, role_str): (String, String) = {
            let conn = self.conn.lock().unwrap();
            conn.query_row(
                "SELECT username, role FROM users WHERE id = ?1",
                params![id],
                |row| Ok((row.get(0)?, row.get(1)?)),
            )
            .map_err(|_| AppError::NotFound(format!("User with id {} not found", id)))?
        };

        if role_str == "admin" {
            let conn = self.conn.lock().unwrap();
            let admin_count: i64 = conn
                .query_row(
                    "SELECT COUNT(*) FROM users WHERE role = 'admin'",
                    [],
                    |row| row.get(0),
                )
                .map_err(|e| AppError::ExecutionFailed(e.to_string()))?;

            if admin_count <= 1 {
                return Err(AppError::InvalidInput(
                    "Cannot delete the only remaining administrator".to_string(),
                ));
            }
        }

        {
            let conn = self.conn.lock().unwrap();
            conn.execute("DELETE FROM users WHERE id = ?1", params![id])
                .map_err(|e| AppError::ExecutionFailed(e.to_string()))?;
        }

        // Invalidate all tokens for this deleted user
        self.revoke_all_for_user(&username)?;

        Ok(())
    }

    pub fn revoke_token(
        &self,
        token_hash: &str,
        jti: Option<&str>,
        username: &str,
        expires_at: i64,
    ) -> Result<(), AppError> {
        let now_str = Utc::now().to_rfc3339();
        {
            let conn = self.conn.lock().unwrap();
            conn.execute(
                "INSERT OR REPLACE INTO revoked_tokens (token_hash, jti, username, expires_at, revoked_at)
                 VALUES (?1, ?2, ?3, ?4, ?5)",
                params![token_hash, jti, username, expires_at, now_str],
            )
            .map_err(|e| AppError::ExecutionFailed(e.to_string()))?;
        }

        let mut cache = self.revoked_tokens_cache.write().unwrap();
        cache.insert(token_hash.to_string(), expires_at);

        Ok(())
    }

    pub fn revoke_all_for_user(&self, username: &str) -> Result<(), AppError> {
        let now = Utc::now().timestamp();
        {
            let conn = self.conn.lock().unwrap();
            conn.execute(
                "INSERT INTO user_token_revocations (username, revoked_before)
                 VALUES (?1, ?2)
                 ON CONFLICT(username) DO UPDATE SET revoked_before = excluded.revoked_before",
                params![username, now],
            )
            .map_err(|e| AppError::ExecutionFailed(e.to_string()))?;
        }

        let mut cache = self.user_revocations_cache.write().unwrap();
        cache.insert(username.to_string(), now);

        Ok(())
    }

    pub fn is_token_revoked(&self, token_hash: &str, username: &str, iat: usize) -> bool {
        let now = Utc::now().timestamp();

        // 1. Check if user has global token revocation issued after/at token's iat
        if let Ok(user_cache) = self.user_revocations_cache.read() {
            if let Some(&revoked_before) = user_cache.get(username) {
                if (iat as i64) <= revoked_before {
                    return true;
                }
            }
        }

        // 2. Check if specific token hash is in revoked tokens cache
        if let Ok(tokens_cache) = self.revoked_tokens_cache.read() {
            if let Some(&expires_at) = tokens_cache.get(token_hash) {
                if expires_at > now {
                    return true;
                }
            }
        }

        false
    }

    #[allow(dead_code)]
    pub fn prune_expired_revocations(&self) -> Result<usize, AppError> {
        let now = Utc::now().timestamp();
        let deleted = {
            let conn = self.conn.lock().unwrap();
            conn.execute(
                "DELETE FROM revoked_tokens WHERE expires_at <= ?1",
                params![now],
            )
            .map_err(|e| AppError::ExecutionFailed(e.to_string()))?
        };

        let mut cache = self.revoked_tokens_cache.write().unwrap();
        cache.retain(|_, &mut exp| exp > now);

        Ok(deleted)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_user_database_lifecycle() {
        let db = UserDatabase::new_in_memory().unwrap();

        // Create user
        let user = db
            .create_user("operator_test", "supersecret", UserRole::Operator)
            .unwrap();
        assert_eq!(user.username, "operator_test");
        assert_eq!(user.role, UserRole::Operator);

        // Authenticate success
        let auth_res = db.authenticate("operator_test", "supersecret");
        assert!(auth_res.is_ok());
        assert_eq!(auth_res.unwrap().username, "operator_test");

        // Authenticate failure
        let fail_res = db.authenticate("operator_test", "wrongpassword");
        assert!(fail_res.is_err());

        // List
        let list = db.list_users().unwrap();
        assert_eq!(list.len(), 1);

        // Update role
        db.update_role(user.id, UserRole::Admin).unwrap();
        let updated = db.get_user_by_username("operator_test").unwrap().unwrap();
        assert_eq!(updated.role, UserRole::Admin);

        // Update password
        db.update_password(user.id, "newsecretpwd").unwrap();
        assert!(db.authenticate("operator_test", "newsecretpwd").is_ok());

        // Create second admin and test admin demote safeguard
        let admin2 = db
            .create_user("admin2", "secret22", UserRole::Admin)
            .unwrap();
        assert_eq!(db.count_admins().unwrap(), 2);
        // Demoting one admin succeeds
        assert!(db.update_role(admin2.id, UserRole::Viewer).is_ok());
        assert_eq!(db.count_admins().unwrap(), 1);
        // Demoting last admin fails
        assert!(db.update_role(user.id, UserRole::Viewer).is_err());
        // Deleting last admin fails
        assert!(db.delete_user(user.id).is_err());
    }

    #[test]
    fn test_token_revocation() {
        let db = UserDatabase::new_in_memory().unwrap();
        let now = Utc::now().timestamp();

        let token_hash = "fake_sha256_hash_12345";
        let username = "alice";
        let exp = now + 3600;
        let iat = (now - 10) as usize;

        // Not revoked initially
        assert!(!db.is_token_revoked(token_hash, username, iat));

        // Revoke token specifically
        db.revoke_token(token_hash, Some("jti-1"), username, exp)
            .unwrap();
        assert!(db.is_token_revoked(token_hash, username, iat));

        // Unrelated token for alice is NOT revoked yet
        assert!(!db.is_token_revoked("other_hash", username, iat));

        // Global user revocation
        db.revoke_all_for_user(username).unwrap();
        // Now any token issued before/at that timestamp is revoked
        assert!(db.is_token_revoked("other_hash", username, iat));

        // A freshly issued token (iat in future) is not revoked
        let future_iat = (now + 50) as usize;
        assert!(!db.is_token_revoked("future_hash", username, future_iat));
    }

    #[test]
    fn test_password_update_and_delete_revokes_tokens() {
        let db = UserDatabase::new_in_memory().unwrap();
        let now = Utc::now().timestamp();
        let user = db
            .create_user("bob", "password123", UserRole::Viewer)
            .unwrap();

        let token_iat = (now - 5) as usize;
        assert!(!db.is_token_revoked("bob_hash", "bob", token_iat));

        // Changing password revokes active tokens
        db.update_password(user.id, "new_password456").unwrap();
        assert!(db.is_token_revoked("bob_hash", "bob", token_iat));

        // Deleting user also revokes active tokens
        let _admin = db
            .create_user("admin", "admin123", UserRole::Admin)
            .unwrap();
        db.delete_user(user.id).unwrap();
        assert!(db.is_token_revoked("bob_hash2", "bob", token_iat));
    }
}
