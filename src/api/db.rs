use actix_web::{web, HttpResponse, Responder, Error};
use actix_multipart::Multipart;
use futures_util::TryStreamExt;
use serde::{Deserialize, Serialize};
use std::process::Command;
use std::fs;
use std::path::Path;
use std::io::Write;
use chrono::Local;
use log::info;

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
pub struct QueryResult {
    pub columns: Vec<String>,
    pub rows: Vec<Vec<String>>,
}

#[derive(Serialize)]
pub struct BackupInfo {
    pub filename: String,
    pub size: u64,
    pub created_at: String,
}

fn is_valid_db_identifier(name: &str) -> bool {
    !name.is_empty() && !name.starts_with('-') && name.chars().all(|c| c.is_ascii_alphanumeric() || c == '_' || c == '-')
}

pub async fn list_dbs() -> impl Responder {
    let mut dbs = Vec::new();

    // 1. Native MySQL
    if let Ok(output) = Command::new("mysql")
        .args(&["-e", "SHOW DATABASES"])
        .output()
    {
        if output.status.success() {
            let stdout = String::from_utf8_lossy(&output.stdout);
            for line in stdout.lines().skip(1) {
                if !line.trim().is_empty() {
                    dbs.push(Database {
                        name: line.trim().to_string(),
                        engine: "mysql".to_string(),
                        size: "-".to_string(), 
                        container_id: None,
                    });
                }
            }
        }
    }

    // 2. Native Postgres
    if let Ok(output) = Command::new("sudo")
        .args(&["-n", "-u", "postgres", "psql", "-l", "-t", "-A", "-F", "|"])
        .output()
    {
        if output.status.success() {
            let stdout = String::from_utf8_lossy(&output.stdout);
            for line in stdout.lines() {
                let parts: Vec<&str> = line.split('|').collect();
                if parts.len() >= 1 && !parts[0].trim().is_empty() {
                    dbs.push(Database {
                        name: parts[0].to_string(),
                        engine: "postgres".to_string(),
                        size: "-".to_string(),
                        container_id: None,
                    });
                }
            }
        }
    }

    // 3. Docker Databases
    if let Ok(output) = Command::new("docker")
        .args(&["ps", "--format", "{{.ID}}|{{.Image}}|{{.Names}}"])
        .output()
    {
        if output.status.success() {
            let stdout = String::from_utf8_lossy(&output.stdout);
            for line in stdout.lines() {
                let parts: Vec<&str> = line.split('|').collect();
                if parts.len() < 3 { continue; }
                
                let id = parts[0];
                let image = parts[1].to_lowercase();

                if image.contains("postgres") {
                    if let Ok(db_out) = Command::new("docker")
                        .args(&["exec", id, "psql", "-U", "postgres", "-l", "-t", "-A", "-F", "|"])
                        .output()
                    {
                        if db_out.status.success() {
                            let db_stdout = String::from_utf8_lossy(&db_out.stdout);
                            for db_line in db_stdout.lines() {
                                let db_parts: Vec<&str> = db_line.split('|').collect();
                                if db_parts.len() >= 1 && !db_parts[0].trim().is_empty() {
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
                } else if image.contains("mysql") || image.contains("mariadb") {
                    if let Ok(db_out) = Command::new("docker")
                        .args(&["exec", id, "mysql", "-uroot", "-e", "SHOW DATABASES", "-N"])
                        .output()
                    {
                        if db_out.status.success() {
                            let db_stdout = String::from_utf8_lossy(&db_out.stdout);
                            for db_line in db_stdout.lines() {
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

    HttpResponse::Ok().json(dbs)
}

pub async fn list_tables(
    path: web::Path<(String, String)>,
    query: web::Query<std::collections::HashMap<String, String>>,
) -> impl Responder {
    let (engine, db_name) = path.into_inner();
    let container_id = query.get("container_id");
    
    if !is_valid_db_identifier(&engine) || !is_valid_db_identifier(&db_name) || container_id.map_or(false, |c| !is_valid_db_identifier(c)) {
        return HttpResponse::BadRequest().json("Invalid identifier");
    }

    let mut tables = Vec::new();

    if engine == "mysql" {
        let mut cmd = if let Some(cid) = container_id {
            let mut c = Command::new("docker");
            c.args(&["exec", cid, "mysql", "-uroot", "-D", &db_name, "-e", "SHOW TABLES", "-N"]);
            c
        } else {
            let mut c = Command::new("mysql");
            c.args(&["-D", &db_name, "-e", "SHOW TABLES", "-N"]);
            c
        };

        if let Ok(output) = cmd.output() {
            if output.status.success() {
                let stdout = String::from_utf8_lossy(&output.stdout);
                for line in stdout.lines() {
                    tables.push(TableInfo { name: line.trim().to_string() });
                }
            }
        }
    } else if engine == "postgres" {
        let mut cmd = if let Some(cid) = container_id {
            let mut c = Command::new("docker");
            c.args(&["exec", cid, "psql", "-U", "postgres", "-d", &db_name, "-t", "-A", "-c", "\\dt"]);
            c
        } else {
            let mut c = Command::new("sudo");
            c.args(&["-n", "-u", "postgres", "psql", "-d", &db_name, "-t", "-A", "-c", "\\dt"]);
            c
        };

        if let Ok(output) = cmd.output() {
            if output.status.success() {
                let stdout = String::from_utf8_lossy(&output.stdout);
                for line in stdout.lines() {
                    let parts: Vec<&str> = line.split('|').collect();
                    if parts.len() >= 2 {
                        tables.push(TableInfo { name: parts[1].to_string() });
                    }
                }
            }
        }
    }

    HttpResponse::Ok().json(tables)
}

pub async fn get_table_data(
    path: web::Path<(String, String, String)>,
    query: web::Query<std::collections::HashMap<String, String>>,
) -> impl Responder {
    let (engine, db_name, table_name) = path.into_inner();
    let container_id = query.get("container_id");
    
    if !is_valid_db_identifier(&engine) || !is_valid_db_identifier(&db_name) || !is_valid_db_identifier(&table_name) || container_id.map_or(false, |c| !is_valid_db_identifier(c)) {
        return HttpResponse::BadRequest().json("Invalid identifier");
    }

    let sql = format!("SELECT * FROM {} LIMIT 100", table_name);
    execute_sql_internal(&engine, &db_name, &sql, container_id.map(|s| s.as_str())).await
}

pub async fn execute_query(
    path: web::Path<(String, String)>,
    query_params: web::Query<std::collections::HashMap<String, String>>,
    body: web::Json<QueryRequest>,
) -> impl Responder {
    let (engine, db_name) = path.into_inner();
    let container_id = query_params.get("container_id");
    
    if !is_valid_db_identifier(&engine) || !is_valid_db_identifier(&db_name) || container_id.map_or(false, |c| !is_valid_db_identifier(c)) {
        return HttpResponse::BadRequest().json("Invalid identifier");
    }
    
    let is_mutation = body.query.to_uppercase().contains("UPDATE") || 
                      body.query.to_uppercase().contains("DELETE") || 
                      body.query.to_uppercase().contains("INSERT") || 
                      body.query.to_uppercase().contains("DROP") || 
                      body.query.to_uppercase().contains("ALTER");

    if is_mutation {
        info!("Executing database change on {} ({}): {}", db_name, engine, body.query);
    }

    let result = execute_sql_internal(&engine, &db_name, &body.query, container_id.map(|s| s.as_str())).await;
    
    if is_mutation && result.status().is_success() {
        info!("Successfully completed database change on {}.", db_name);
    } else if is_mutation {
        info!("Database change failed on {}.", db_name);
    }

    result
}

async fn execute_sql_internal(engine: &str, db: &str, query: &str, container_id: Option<&str>) -> HttpResponse {
    let mut columns = Vec::new();
    let mut rows = Vec::new();

    if engine == "mysql" {
        let mut cmd = if let Some(cid) = container_id {
            let mut c = Command::new("docker");
            c.args(&["exec", cid, "mysql", "-uroot", "-D", db, "-B", "-e", query]);
            c
        } else {
            let mut c = Command::new("mysql");
            c.args(&["-D", db, "-B", "-e", query]);
            c
        };

        if let Ok(output) = cmd.output() {
            if output.status.success() {
                let stdout = String::from_utf8_lossy(&output.stdout);
                let mut lines = stdout.lines();
                if let Some(header) = lines.next() {
                    columns = header.split('\t').map(|s| s.to_string()).collect();
                    for line in lines {
                        rows.push(line.split('\t').map(|s| s.to_string()).collect());
                    }
                }
                return HttpResponse::Ok().json(QueryResult { columns, rows });
            } else {
                return HttpResponse::BadRequest().json(String::from_utf8_lossy(&output.stderr));
            }
        }
    } else if engine == "postgres" {
        let mut cmd = if let Some(cid) = container_id {
            let mut c = Command::new("docker");
            c.args(&["exec", cid, "psql", "-U", "postgres", "-d", db, "-A", "-F", "\t", "-c", query]);
            c
        } else {
            let mut c = Command::new("sudo");
            c.args(&["-n", "-u", "postgres", "psql", "-d", db, "-A", "-F", "\t", "-c", query]);
            c
        };

        if let Ok(output) = cmd.output() {
            if output.status.success() {
                let stdout = String::from_utf8_lossy(&output.stdout);
                let mut lines = stdout.lines();
                if let Some(header) = lines.next() {
                    columns = header.split('\t').map(|s| s.to_string()).collect();
                    for line in lines {
                        if line.contains('(') && line.contains("row") { break; }
                        rows.push(line.split('\t').map(|s| s.to_string()).collect());
                    }
                }
                return HttpResponse::Ok().json(QueryResult { columns, rows });
            } else {
                return HttpResponse::BadRequest().json(String::from_utf8_lossy(&output.stderr));
            }
        }
    }

    HttpResponse::InternalServerError().json("Database engine not supported or command failed")
}

// BACKUP & RESTORE

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
                        created_at: metadata.created()
                            .map(|t| chrono::DateTime::<Local>::from(t).format("%Y-%m-%d %H:%M:%S").to_string())
                            .unwrap_or_else(|_| "Unknown".to_string()),
                    });
                }
            }
        }
    }
    
    backups.sort_by(|a, b| b.filename.cmp(&a.filename)); // Newest first (by name timestamp)
    HttpResponse::Ok().json(backups)
}

pub async fn create_backup(
    path: web::Path<(String, String)>,
    query: web::Query<std::collections::HashMap<String, String>>,
) -> impl Responder {
    let (engine, db) = path.into_inner();
    let container_id = query.get("container_id");
    
    if !is_valid_db_identifier(&engine) || !is_valid_db_identifier(&db) || container_id.map_or(false, |c| !is_valid_db_identifier(c)) {
        return HttpResponse::BadRequest().json("Invalid identifier");
    }

    let dir = get_backup_dir(&engine, &db);
    fs::create_dir_all(&dir).ok();

    let timestamp = Local::now().format("%Y%m%d_%H%M%S").to_string();
    let filename = format!("{}_{}.sql", db, timestamp);
    let filepath = format!("{}/{}", dir, filename);

    info!("Starting database backup for {} ({})...", db, engine);

    let success = if engine == "mysql" {
        if let Some(cid) = container_id {
            let output = Command::new("sh")
                .args(&["-c", &format!("docker exec {} mysqldump -uroot {} > {}", cid, db, filepath)])
                .output();
            output.map(|o| o.status.success()).unwrap_or(false)
        } else {
            let output = Command::new("sh")
                .args(&["-c", &format!("mysqldump -uroot {} > {}", db, filepath)])
                .output();
            output.map(|o| o.status.success()).unwrap_or(false)
        }
    } else if engine == "postgres" {
        if let Some(cid) = container_id {
            let output = Command::new("sh")
                .args(&["-c", &format!("docker exec {} pg_dump -U postgres {} > {}", cid, db, filepath)])
                .output();
            output.map(|o| o.status.success()).unwrap_or(false)
        } else {
            let output = Command::new("sh")
                .args(&["-c", &format!("sudo -n -u postgres pg_dump {} > {}", db, filepath)])
                .output();
            output.map(|o| o.status.success()).unwrap_or(false)
        }
    } else {
        false
    };

    if success {
        info!("Backup completed successfully: {}", filename);
        HttpResponse::Ok().json(format!("Backup created: {}", filename))
    } else {
        info!("Backup failed for {}.", db);
        HttpResponse::InternalServerError().json("Failed to create backup")
    }
}

pub async fn restore_backup(
    path: web::Path<(String, String, String)>,
    query: web::Query<std::collections::HashMap<String, String>>,
) -> impl Responder {
    let (engine, db, filename) = path.into_inner();
    let container_id = query.get("container_id");
    
    if !is_valid_db_identifier(&engine) || !is_valid_db_identifier(&db) || container_id.map_or(false, |c| !is_valid_db_identifier(c)) {
        return HttpResponse::BadRequest().json("Invalid identifier");
    }

    let filepath = format!("{}/{}", get_backup_dir(&engine, &db), filename);

    if !Path::new(&filepath).exists() {
        return HttpResponse::NotFound().json("Backup file not found");
    }

    info!("Initiating database restoration for {} from {}...", db, filename);

    let success = if engine == "mysql" {
        if let Some(cid) = container_id {
            let output = Command::new("sh")
                .args(&["-c", &format!("docker exec -i {} mysql -uroot {} < {}", cid, db, filepath)])
                .output();
            output.map(|o| o.status.success()).unwrap_or(false)
        } else {
            let output = Command::new("sh")
                .args(&["-c", &format!("mysql -uroot {} < {}", db, filepath)])
                .output();
            output.map(|o| o.status.success()).unwrap_or(false)
        }
    } else if engine == "postgres" {
        if let Some(cid) = container_id {
            let output = Command::new("sh")
                .args(&["-c", &format!("docker exec -i {} psql -U postgres -d {} < {}", cid, db, filepath)])
                .output();
            output.map(|o| o.status.success()).unwrap_or(false)
        } else {
            let output = Command::new("sh")
                .args(&["-c", &format!("sudo -n -u postgres psql -d {} < {}", db, filepath)])
                .output();
            output.map(|o| o.status.success()).unwrap_or(false)
        }
    } else {
        false
    };

    if success {
        info!("Database {} successfully restored from {}.", db, filename);
        HttpResponse::Ok().json("Restore successful")
    } else {
        info!("Database restoration failed for {}.", db);
        HttpResponse::InternalServerError().json("Restore failed")
    }
}

pub async fn download_backup(path: web::Path<(String, String, String)>) -> Result<actix_files::NamedFile, Error> {
    let (engine, db, filename) = path.into_inner();
    if !is_valid_db_identifier(&engine) || !is_valid_db_identifier(&db) || !is_valid_db_identifier(&filename.replace(".sql", "").replace(".zip", "")) {
        return Err(actix_web::error::ErrorBadRequest("Invalid identifier"));
    }
    let filepath = format!("{}/{}", get_backup_dir(&engine, &db), filename);
    
    info!("Exporting database backup: {}", filename);
    actix_files::NamedFile::open(filepath).map_err(|e| e.into())
}

pub async fn upload_backup(
    path: web::Path<(String, String)>,
    mut payload: Multipart,
) -> Result<HttpResponse, Error> {
    let (engine, db) = path.into_inner();
    
    if !is_valid_db_identifier(&engine) || !is_valid_db_identifier(&db) {
        return Ok(HttpResponse::BadRequest().json("Invalid identifier"));
    }

    let dir = get_backup_dir(&engine, &db);
    fs::create_dir_all(&dir).ok();

    info!("Importing external SQL file for {} ({})...", db, engine);

    while let Ok(Some(mut field)) = payload.try_next().await {
        let content_disposition = field.content_disposition();
        let filename = content_disposition
            .get_filename()
            .map(|f| f.to_string())
            .unwrap_or_else(|| format!("upload_{}.sql", Local::now().format("%Y%m%d_%H%M%S")));
        
        let filepath = format!("{}/{}", dir, filename);
        let mut f = fs::File::create(filepath)?;

        while let Ok(Some(chunk)) = field.try_next().await {
            f.write_all(&chunk)?;
        }
        info!("Imported SQL file saved as: {}", filename);
    }

    Ok(HttpResponse::Ok().json("File uploaded successfully"))
}

pub async fn delete_backup(path: web::Path<(String, String, String)>) -> impl Responder {
    let (engine, db, filename) = path.into_inner();
    
    if !is_valid_db_identifier(&engine) || !is_valid_db_identifier(&db) || !is_valid_db_identifier(&filename.replace(".sql", "").replace(".zip", "")) {
        return HttpResponse::BadRequest().json("Invalid identifier");
    }

    let filepath = format!("{}/{}", get_backup_dir(&engine, &db), filename);
    
    if fs::remove_file(filepath).is_ok() {
        info!("Deleted backup file: {}", filename);
        HttpResponse::Ok().json("Backup deleted")
    } else {
        HttpResponse::InternalServerError().json("Failed to delete backup")
    }
}
