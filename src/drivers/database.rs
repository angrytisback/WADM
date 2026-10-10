use super::error::AppError;
use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use std::path::Path;
use tokio::process::Command as TokioCommand;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct QueryResult {
    pub columns: Vec<String>,
    pub rows: Vec<Vec<String>>,
}

pub fn is_valid_db_identifier(name: &str) -> bool {
    !name.is_empty()
        && !name.starts_with('-')
        && name.len() <= 64
        && name
            .chars()
            .all(|c| c.is_ascii_alphanumeric() || c == '_' || c == '-')
}

#[allow(dead_code)]
#[async_trait]
pub trait DatabaseDriver: Send + Sync {
    fn engine(&self) -> &'static str;
    async fn list_databases(&self) -> Result<Vec<String>, AppError>;
    async fn list_tables(
        &self,
        db: &str,
        container_id: Option<&str>,
    ) -> Result<Vec<String>, AppError>;

    async fn execute_query(&self, db: &str, sql: &str) -> Result<QueryResult, AppError> {
        self.execute_query_container(db, sql, None).await
    }

    async fn execute_query_container(
        &self,
        db: &str,
        sql: &str,
        container_id: Option<&str>,
    ) -> Result<QueryResult, AppError>;

    fn build_backup_command(&self, db: &str, out_path: &Path) -> (String, Vec<String>) {
        self.build_backup_command_container(db, out_path, None)
    }

    fn build_backup_command_container(
        &self,
        db: &str,
        out_path: &Path,
        container_id: Option<&str>,
    ) -> (String, Vec<String>);

    fn build_restore_command(&self, db: &str, in_path: &Path) -> (String, Vec<String>) {
        self.build_restore_command_container(db, in_path, None)
    }

    fn build_restore_command_container(
        &self,
        db: &str,
        in_path: &Path,
        container_id: Option<&str>,
    ) -> (String, Vec<String>);
}

// ============================================================================
// MySQL Driver
// ============================================================================

pub struct MySqlDriver;

#[async_trait]
impl DatabaseDriver for MySqlDriver {
    fn engine(&self) -> &'static str {
        "mysql"
    }

    async fn list_databases(&self) -> Result<Vec<String>, AppError> {
        let output = TokioCommand::new("mysql")
            .args(["-e", "SHOW DATABASES"])
            .output()
            .await
            .map_err(|e| AppError::ExecutionFailed(e.to_string()))?;

        if !output.status.success() {
            return Err(AppError::ExecutionFailed(
                String::from_utf8_lossy(&output.stderr).to_string(),
            ));
        }

        let stdout = String::from_utf8_lossy(&output.stdout);
        let dbs = stdout
            .lines()
            .skip(1)
            .map(|l| l.trim().to_string())
            .filter(|l| !l.is_empty())
            .collect();

        Ok(dbs)
    }

    async fn list_tables(
        &self,
        db: &str,
        container_id: Option<&str>,
    ) -> Result<Vec<String>, AppError> {
        let mut cmd = if let Some(cid) = container_id {
            let mut c = TokioCommand::new("docker");
            c.args([
                "exec",
                cid,
                "mysql",
                "-uroot",
                "-D",
                db,
                "-e",
                "SHOW TABLES",
                "-N",
            ]);
            c
        } else {
            let mut c = TokioCommand::new("mysql");
            c.args(["-D", db, "-e", "SHOW TABLES", "-N"]);
            c
        };

        let output = cmd
            .output()
            .await
            .map_err(|e| AppError::ExecutionFailed(e.to_string()))?;

        if !output.status.success() {
            return Err(AppError::ExecutionFailed(
                String::from_utf8_lossy(&output.stderr).to_string(),
            ));
        }

        let stdout = String::from_utf8_lossy(&output.stdout);
        let tables = stdout
            .lines()
            .map(|l| l.trim().to_string())
            .filter(|l| !l.is_empty())
            .collect();
        Ok(tables)
    }

    async fn execute_query_container(
        &self,
        db: &str,
        sql: &str,
        container_id: Option<&str>,
    ) -> Result<QueryResult, AppError> {
        let mut cmd = if let Some(cid) = container_id {
            let mut c = TokioCommand::new("docker");
            c.args(["exec", cid, "mysql", "-uroot", "-D", db, "-B", "-e", sql]);
            c
        } else {
            let mut c = TokioCommand::new("mysql");
            c.args(["-D", db, "-B", "-e", sql]);
            c
        };

        let output = cmd
            .output()
            .await
            .map_err(|e| AppError::ExecutionFailed(e.to_string()))?;

        if !output.status.success() {
            return Err(AppError::ExecutionFailed(
                String::from_utf8_lossy(&output.stderr).to_string(),
            ));
        }

        let stdout = String::from_utf8_lossy(&output.stdout);
        let mut lines = stdout.lines();
        let mut columns = Vec::new();
        let mut rows = Vec::new();

        if let Some(header) = lines.next() {
            columns = header.split('\t').map(|s| s.to_string()).collect();
            for line in lines {
                rows.push(line.split('\t').map(|s| s.to_string()).collect());
            }
        }

        Ok(QueryResult { columns, rows })
    }

    fn build_backup_command_container(
        &self,
        db: &str,
        _out_path: &Path,
        container_id: Option<&str>,
    ) -> (String, Vec<String>) {
        if let Some(cid) = container_id {
            (
                "docker".to_string(),
                vec![
                    "exec".to_string(),
                    cid.to_string(),
                    "mysqldump".to_string(),
                    "-uroot".to_string(),
                    db.to_string(),
                ],
            )
        } else {
            (
                "mysqldump".to_string(),
                vec!["-uroot".to_string(), db.to_string()],
            )
        }
    }

    fn build_restore_command_container(
        &self,
        db: &str,
        _in_path: &Path,
        container_id: Option<&str>,
    ) -> (String, Vec<String>) {
        if let Some(cid) = container_id {
            (
                "docker".to_string(),
                vec![
                    "exec".to_string(),
                    "-i".to_string(),
                    cid.to_string(),
                    "mysql".to_string(),
                    "-uroot".to_string(),
                    "-D".to_string(),
                    db.to_string(),
                ],
            )
        } else {
            ("mysql".to_string(), vec!["-D".to_string(), db.to_string()])
        }
    }
}

// ============================================================================
// PostgreSQL Driver
// ============================================================================

pub struct PostgresDriver;

#[async_trait]
impl DatabaseDriver for PostgresDriver {
    fn engine(&self) -> &'static str {
        "postgres"
    }

    async fn list_databases(&self) -> Result<Vec<String>, AppError> {
        let output = TokioCommand::new("sudo")
            .args(["-n", "-u", "postgres", "psql", "-l", "-t", "-A", "-F", "|"])
            .output()
            .await
            .map_err(|e| AppError::ExecutionFailed(e.to_string()))?;

        if !output.status.success() {
            return Err(AppError::ExecutionFailed(
                String::from_utf8_lossy(&output.stderr).to_string(),
            ));
        }

        let stdout = String::from_utf8_lossy(&output.stdout);
        let mut dbs = Vec::new();
        for line in stdout.lines() {
            let parts: Vec<&str> = line.split('|').collect();
            if !parts.is_empty() && !parts[0].trim().is_empty() {
                dbs.push(parts[0].trim().to_string());
            }
        }

        Ok(dbs)
    }

    async fn list_tables(
        &self,
        db: &str,
        container_id: Option<&str>,
    ) -> Result<Vec<String>, AppError> {
        let mut cmd = if let Some(cid) = container_id {
            let mut c = TokioCommand::new("docker");
            c.args([
                "exec", cid, "psql", "-U", "postgres", "-d", db, "-t", "-A", "-c", "\\dt",
            ]);
            c
        } else {
            let mut c = TokioCommand::new("sudo");
            c.args([
                "-n", "-u", "postgres", "psql", "-d", db, "-t", "-A", "-c", "\\dt",
            ]);
            c
        };

        let output = cmd
            .output()
            .await
            .map_err(|e| AppError::ExecutionFailed(e.to_string()))?;

        if !output.status.success() {
            return Err(AppError::ExecutionFailed(
                String::from_utf8_lossy(&output.stderr).to_string(),
            ));
        }

        let stdout = String::from_utf8_lossy(&output.stdout);
        let mut tables = Vec::new();
        for line in stdout.lines() {
            let parts: Vec<&str> = line.split('|').collect();
            if parts.len() >= 2 {
                tables.push(parts[1].trim().to_string());
            }
        }

        Ok(tables)
    }

    async fn execute_query_container(
        &self,
        db: &str,
        sql: &str,
        container_id: Option<&str>,
    ) -> Result<QueryResult, AppError> {
        let mut cmd = if let Some(cid) = container_id {
            let mut c = TokioCommand::new("docker");
            c.args([
                "exec", cid, "psql", "-U", "postgres", "-d", db, "-A", "-F", "\t", "-c", sql,
            ]);
            c
        } else {
            let mut c = TokioCommand::new("sudo");
            c.args([
                "-n", "-u", "postgres", "psql", "-d", db, "-A", "-F", "\t", "-c", sql,
            ]);
            c
        };

        let output = cmd
            .output()
            .await
            .map_err(|e| AppError::ExecutionFailed(e.to_string()))?;

        if !output.status.success() {
            return Err(AppError::ExecutionFailed(
                String::from_utf8_lossy(&output.stderr).to_string(),
            ));
        }

        let stdout = String::from_utf8_lossy(&output.stdout);
        let mut lines = stdout.lines();
        let mut columns = Vec::new();
        let mut rows = Vec::new();

        if let Some(header) = lines.next() {
            columns = header.split('\t').map(|s| s.to_string()).collect();
            for line in lines {
                if line.contains('(') && line.contains("row") {
                    break;
                }
                rows.push(line.split('\t').map(|s| s.to_string()).collect());
            }
        }

        Ok(QueryResult { columns, rows })
    }

    fn build_backup_command_container(
        &self,
        db: &str,
        _out_path: &Path,
        container_id: Option<&str>,
    ) -> (String, Vec<String>) {
        if let Some(cid) = container_id {
            (
                "docker".to_string(),
                vec![
                    "exec".to_string(),
                    cid.to_string(),
                    "pg_dump".to_string(),
                    "-U".to_string(),
                    "postgres".to_string(),
                    db.to_string(),
                ],
            )
        } else {
            (
                "sudo".to_string(),
                vec![
                    "-n".to_string(),
                    "-u".to_string(),
                    "postgres".to_string(),
                    "pg_dump".to_string(),
                    db.to_string(),
                ],
            )
        }
    }

    fn build_restore_command_container(
        &self,
        db: &str,
        _in_path: &Path,
        container_id: Option<&str>,
    ) -> (String, Vec<String>) {
        if let Some(cid) = container_id {
            (
                "docker".to_string(),
                vec![
                    "exec".to_string(),
                    "-i".to_string(),
                    cid.to_string(),
                    "psql".to_string(),
                    "-U".to_string(),
                    "postgres".to_string(),
                    "-d".to_string(),
                    db.to_string(),
                ],
            )
        } else {
            (
                "sudo".to_string(),
                vec![
                    "-n".to_string(),
                    "-u".to_string(),
                    "postgres".to_string(),
                    "psql".to_string(),
                    "-d".to_string(),
                    db.to_string(),
                ],
            )
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_valid_db_identifiers() {
        assert!(is_valid_db_identifier("production_db"));
        assert!(is_valid_db_identifier("app-data-1"));
        assert!(is_valid_db_identifier("wadm"));

        assert!(!is_valid_db_identifier(""));
        assert!(!is_valid_db_identifier("-db"));
        assert!(!is_valid_db_identifier("db; DROP TABLE users;"));
        assert!(!is_valid_db_identifier("db space"));
    }

    #[test]
    fn test_mysql_command_builder() {
        let driver = MySqlDriver;
        assert_eq!(driver.engine(), "mysql");

        let path = Path::new("/tmp/test.sql");
        let (cmd, args) = driver.build_backup_command("testdb", path);
        assert_eq!(cmd, "mysqldump");
        assert_eq!(args, vec!["-uroot", "testdb"]);

        let (cmd, args) =
            driver.build_backup_command_container("testdb", path, Some("container_123"));
        assert_eq!(cmd, "docker");
        assert_eq!(
            args,
            vec!["exec", "container_123", "mysqldump", "-uroot", "testdb"]
        );
    }

    #[test]
    fn test_postgres_command_builder() {
        let driver = PostgresDriver;
        assert_eq!(driver.engine(), "postgres");

        let path = Path::new("/tmp/test.sql");
        let (cmd, args) = driver.build_backup_command("testdb", path);
        assert_eq!(cmd, "sudo");
        assert_eq!(args, vec!["-n", "-u", "postgres", "pg_dump", "testdb"]);

        let (cmd, args) =
            driver.build_backup_command_container("testdb", path, Some("container_456"));
        assert_eq!(cmd, "docker");
        assert_eq!(
            args,
            vec![
                "exec",
                "container_456",
                "pg_dump",
                "-U",
                "postgres",
                "testdb"
            ]
        );
    }
}
