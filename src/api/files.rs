use actix_files::NamedFile;
use actix_multipart::Multipart;
use actix_web::{web, Error, HttpResponse, Responder};
use chrono::{DateTime, Local};
use futures_util::TryStreamExt;
use serde::{Deserialize, Serialize};
use std::fs;
use std::io::Write;
#[cfg(unix)]
use std::os::unix::fs::PermissionsExt;
use std::path::{Path, PathBuf};

#[derive(Serialize)]
pub struct FileInfo {
    pub name: String,
    pub path: String,
    pub is_dir: bool,
    pub size: u64,
    pub permissions: String,
    pub modified_at: String,
}

#[derive(Deserialize)]
pub struct FileRequest {
    pub path: String,
}

#[derive(Deserialize)]
pub struct FileWriteRequest {
    pub path: String,
    pub content: String,
}

#[derive(Deserialize)]
pub struct FileCreateRequest {
    pub path: String,
    pub is_dir: bool,
}

fn get_permissions_string(mode: u32) -> String {
    let user = format!(
        "{}{}{}",
        if mode & 0o400 != 0 { "r" } else { "-" },
        if mode & 0o200 != 0 { "w" } else { "-" },
        if mode & 0o100 != 0 { "x" } else { "-" }
    );
    let group = format!(
        "{}{}{}",
        if mode & 0o040 != 0 { "r" } else { "-" },
        if mode & 0o020 != 0 { "w" } else { "-" },
        if mode & 0o010 != 0 { "x" } else { "-" }
    );
    let other = format!(
        "{}{}{}",
        if mode & 0o004 != 0 { "r" } else { "-" },
        if mode & 0o002 != 0 { "w" } else { "-" },
        if mode & 0o001 != 0 { "x" } else { "-" }
    );
    format!("{}{}{}", user, group, other)
}

pub async fn list_files(query: web::Query<FileRequest>) -> impl Responder {
    let req_path = Path::new(&query.path);

    if !req_path.exists() || !req_path.is_dir() {
        return HttpResponse::BadRequest().json("Path does not exist or is not a directory");
    }

    let mut files = Vec::new();
    if let Ok(entries) = fs::read_dir(req_path) {
        for entry in entries.flatten() {
            if let Ok(metadata) = entry.metadata() {
                let is_dir = metadata.is_dir();
                let size = if is_dir { 0 } else { metadata.len() };
                #[cfg(unix)]
                let mode = metadata.permissions().mode();
                #[cfg(not(unix))]
                let mode = if metadata.permissions().readonly() {
                    0o444
                } else {
                    0o644
                };
                let modified = metadata
                    .modified()
                    .map(|sys_time| {
                        let dt: DateTime<Local> = sys_time.into();
                        dt.format("%Y-%m-%d %H:%M:%S").to_string()
                    })
                    .unwrap_or_else(|_| "Unknown".to_string());

                files.push(FileInfo {
                    name: entry.file_name().to_string_lossy().to_string(),
                    path: entry.path().to_string_lossy().to_string(),
                    is_dir,
                    size,
                    permissions: get_permissions_string(mode),
                    modified_at: modified,
                });
            }
        }
    }

    files.sort_by(|a, b| {
        if a.is_dir && !b.is_dir {
            std::cmp::Ordering::Less
        } else if !a.is_dir && b.is_dir {
            std::cmp::Ordering::Greater
        } else {
            a.name.to_lowercase().cmp(&b.name.to_lowercase())
        }
    });

    HttpResponse::Ok().json(files)
}

pub async fn read_file(query: web::Query<FileRequest>) -> impl Responder {
    let path = Path::new(&query.path);
    if !path.exists() || path.is_dir() {
        return HttpResponse::BadRequest().json("File does not exist or is a directory");
    }

    let metadata = fs::metadata(path).unwrap_or_else(|_| fs::metadata("/dev/null").unwrap());
    if metadata.len() > 10 * 1024 * 1024 {
        return HttpResponse::BadRequest().json("File is too large to read into memory (max 10MB)");
    }

    match fs::read_to_string(path) {
        Ok(content) => HttpResponse::Ok().json(content),
        Err(e) => {
            if e.kind() == std::io::ErrorKind::InvalidData {
                HttpResponse::BadRequest()
                    .json("File appears to be binary and cannot be edited as text.")
            } else {
                HttpResponse::InternalServerError().json(format!("Failed to read file: {}", e))
            }
        }
    }
}

pub async fn write_file(body: web::Json<FileWriteRequest>) -> impl Responder {
    let path = Path::new(&body.path);

    if path.exists() && path.is_dir() {
        return HttpResponse::BadRequest().json("Path is a directory");
    }

    match fs::write(path, &body.content) {
        Ok(_) => HttpResponse::Ok().json("File saved successfully"),
        Err(e) => HttpResponse::InternalServerError().json(format!("Failed to save file: {}", e)),
    }
}

pub async fn create_item(body: web::Json<FileCreateRequest>) -> impl Responder {
    let path = Path::new(&body.path);

    if path.exists() {
        return HttpResponse::BadRequest().json("Path already exists");
    }

    if body.is_dir {
        match fs::create_dir_all(path) {
            Ok(_) => HttpResponse::Ok().json("Directory created successfully"),
            Err(e) => HttpResponse::InternalServerError()
                .json(format!("Failed to create directory: {}", e)),
        }
    } else {
        match fs::File::create(path) {
            Ok(_) => HttpResponse::Ok().json("File created successfully"),
            Err(e) => {
                HttpResponse::InternalServerError().json(format!("Failed to create file: {}", e))
            }
        }
    }
}

pub async fn delete_item(query: web::Query<FileRequest>) -> impl Responder {
    let path = Path::new(&query.path);

    if !path.exists() {
        return HttpResponse::NotFound().json("Path does not exist");
    }

    let result = if path.is_dir() {
        fs::remove_dir_all(path)
    } else {
        fs::remove_file(path)
    };

    match result {
        Ok(_) => HttpResponse::Ok().json("Deleted successfully"),
        Err(e) => HttpResponse::InternalServerError().json(format!("Failed to delete: {}", e)),
    }
}

pub async fn download_file(query: web::Query<FileRequest>) -> Result<NamedFile, Error> {
    let path = PathBuf::from(&query.path);

    if !path.exists() || path.is_dir() {
        return Err(actix_web::error::ErrorBadRequest(
            "Path does not exist or is a directory",
        ));
    }

    NamedFile::open(path).map_err(|e| e.into())
}

pub async fn upload_file(
    query: web::Query<FileRequest>,
    mut payload: Multipart,
) -> Result<HttpResponse, Error> {
    let target_dir = Path::new(&query.path);
    if !target_dir.exists() || !target_dir.is_dir() {
        return Ok(HttpResponse::BadRequest().json("Target directory does not exist"));
    }

    while let Ok(Some(mut field)) = payload.try_next().await {
        let content_disposition = field.content_disposition();
        if let Some(filename) = content_disposition.get_filename() {
            let filepath = target_dir.join(filename);

            let mut f = fs::File::create(&filepath)?;
            while let Ok(Some(chunk)) = field.try_next().await {
                f.write_all(&chunk)?;
            }
        }
    }

    Ok(HttpResponse::Ok().json("Upload complete"))
}
