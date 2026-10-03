use actix_files::NamedFile;
use actix_multipart::Multipart;
use actix_web::{web, Error, HttpResponse, Responder};
use chrono::{DateTime, Local};
use futures_util::TryStreamExt;
use serde::{Deserialize, Serialize};
use std::fs;
use std::io::{Read, Write};
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

fn validate_and_sanitize_path(path_str: &str, allow_write: bool) -> Result<PathBuf, &'static str> {
    if path_str.trim().is_empty() {
        return Err("Path cannot be empty");
    }
    if path_str.contains('\0') {
        return Err("Path contains null byte");
    }

    let p = Path::new(path_str);
    // Explicitly reject relative traversal tokens
    for component in p.components() {
        if let std::path::Component::ParentDir = component {
            return Err("Path traversal '..' is forbidden");
        }
    }

    let canonical = if p.exists() {
        p.canonicalize().map_err(|_| "Failed to resolve path")?
    } else {
        // For new files/folders, resolve parent
        if let Some(parent) = p.parent() {
            let canonical_parent = if parent.as_os_str().is_empty() {
                std::env::current_dir().map_err(|_| "Failed to get current directory")?
            } else if parent.exists() {
                parent
                    .canonicalize()
                    .map_err(|_| "Failed to resolve parent directory")?
            } else {
                return Err("Parent directory does not exist");
            };

            if let Some(file_name) = p.file_name() {
                canonical_parent.join(file_name)
            } else {
                canonical_parent
            }
        } else {
            return Err("Invalid path hierarchy");
        }
    };

    let path_str_clean = canonical.to_string_lossy();

    // Sensitive files denied from reading
    if path_str_clean.starts_with("/etc/shadow")
        || path_str_clean.starts_with("/etc/gshadow")
        || path_str_clean.starts_with("/etc/sudoers")
        || path_str_clean == "/etc/master.passwd"
        || path_str_clean.starts_with("/etc/ssl/private")
        || (path_str_clean.starts_with("/proc/") && path_str_clean.ends_with("/environ"))
    {
        return Err("Access to sensitive system file is denied");
    }

    // SSH private keys denied from reading
    if let Some(file_name) = canonical.file_name().and_then(|n| n.to_str()) {
        let is_private_key_name = file_name.starts_with("id_")
            && !file_name.ends_with(".pub")
            && !file_name.ends_with(".cert");
        let is_key_file = file_name.ends_with(".pem") || file_name.ends_with(".key");
        let in_ssh_dir = canonical.components().any(|c| c.as_os_str() == ".ssh");
        if in_ssh_dir && (is_private_key_name || is_key_file) {
            return Err("Access to sensitive system file is denied");
        }
    }

    // Protect WADM's own secret keys and auth configuration
    if path_str_clean.ends_with(".wadm_jwt_secret") || path_str_clean.ends_with("wadm-auth.json") {
        return Err("Access to WADM internal credentials is forbidden");
    }

    if allow_write {
        if path_str_clean == "/" {
            return Err("Modifying filesystem root is forbidden");
        }

        // Critical system trees where arbitrary write can lead to privilege escalation / takeover
        let forbidden_write_prefixes = [
            "/etc",
            "/boot",
            "/usr",
            "/bin",
            "/sbin",
            "/lib",
            "/lib64",
            "/proc",
            "/sys",
            "/dev",
            "/root/.ssh",
        ];
        for prefix in forbidden_write_prefixes {
            if path_str_clean == prefix || path_str_clean.starts_with(&format!("{}/", prefix)) {
                return Err("Modifying critical system path is forbidden");
            }
        }

        // Global protection against authorized_keys backdoor injection across all users
        if let Some(file_name) = canonical.file_name().and_then(|n| n.to_str()) {
            if file_name == "authorized_keys" || file_name == "authorized_keys2" {
                return Err("Modifying critical system path is forbidden");
            }
        }
    }

    Ok(canonical)
}

pub async fn list_files(query: web::Query<FileRequest>) -> impl Responder {
    let req_path = match validate_and_sanitize_path(&query.path, false) {
        Ok(p) => p,
        Err(err) => return HttpResponse::BadRequest().json(err),
    };

    if !req_path.exists() || !req_path.is_dir() {
        return HttpResponse::BadRequest().json("Path does not exist or is not a directory");
    }

    let mut files = Vec::new();
    if let Ok(entries) = fs::read_dir(&req_path) {
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
    let path = match validate_and_sanitize_path(&query.path, false) {
        Ok(p) => p,
        Err(err) => return HttpResponse::BadRequest().json(err),
    };

    if !path.exists() || path.is_dir() {
        return HttpResponse::BadRequest().json("File does not exist or is a directory");
    }

    let metadata = match fs::metadata(&path) {
        Ok(m) => m,
        Err(e) => {
            return HttpResponse::InternalServerError()
                .json(format!("Failed to read metadata: {}", e))
        }
    };

    if !metadata.is_file() {
        return HttpResponse::BadRequest()
            .json("Only regular files can be read (devices, pipes, and sockets are forbidden)");
    }

    if metadata.len() > 10 * 1024 * 1024 {
        return HttpResponse::BadRequest().json("File is too large to read into memory (max 10MB)");
    }

    let file = match fs::File::open(&path) {
        Ok(f) => f,
        Err(e) => {
            return HttpResponse::InternalServerError().json(format!("Failed to open file: {}", e))
        }
    };

    let mut buffer = String::new();
    let mut handle = file.take(10 * 1024 * 1024 + 1);
    match handle.read_to_string(&mut buffer) {
        Ok(bytes_read) => {
            if bytes_read > 10 * 1024 * 1024 {
                HttpResponse::BadRequest().json("File is too large to read into memory (max 10MB)")
            } else {
                HttpResponse::Ok().json(buffer)
            }
        }
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
    let path = match validate_and_sanitize_path(&body.path, true) {
        Ok(p) => p,
        Err(err) => return HttpResponse::BadRequest().json(err),
    };

    if path.exists() && path.is_dir() {
        return HttpResponse::BadRequest().json("Path is a directory");
    }

    match fs::write(&path, &body.content) {
        Ok(_) => HttpResponse::Ok().json("File saved successfully"),
        Err(e) => HttpResponse::InternalServerError().json(format!("Failed to save file: {}", e)),
    }
}

pub async fn create_item(body: web::Json<FileCreateRequest>) -> impl Responder {
    let path = match validate_and_sanitize_path(&body.path, true) {
        Ok(p) => p,
        Err(err) => return HttpResponse::BadRequest().json(err),
    };

    if path.exists() {
        return HttpResponse::BadRequest().json("Path already exists");
    }

    if body.is_dir {
        match fs::create_dir_all(&path) {
            Ok(_) => HttpResponse::Ok().json("Directory created successfully"),
            Err(e) => HttpResponse::InternalServerError()
                .json(format!("Failed to create directory: {}", e)),
        }
    } else {
        match fs::File::create(&path) {
            Ok(_) => HttpResponse::Ok().json("File created successfully"),
            Err(e) => {
                HttpResponse::InternalServerError().json(format!("Failed to create file: {}", e))
            }
        }
    }
}

pub async fn delete_item(query: web::Query<FileRequest>) -> impl Responder {
    let path = match validate_and_sanitize_path(&query.path, true) {
        Ok(p) => p,
        Err(err) => return HttpResponse::BadRequest().json(err),
    };

    if !path.exists() {
        return HttpResponse::NotFound().json("Path does not exist");
    }

    let path_str = path.to_string_lossy();
    if path_str == "/"
        || path_str == "/root"
        || path_str == "/home"
        || path_str == "/var"
        || path_str == "/tmp"
        || path_str == "/usr"
        || path_str == "/etc"
        || path_str == "/opt"
        || path_str == "/srv"
        || path_str == "/mnt"
        || path_str == "/media"
    {
        return HttpResponse::BadRequest().json("Deleting system root directories is forbidden");
    }

    let result = if path.is_dir() {
        fs::remove_dir_all(&path)
    } else {
        fs::remove_file(&path)
    };

    match result {
        Ok(_) => HttpResponse::Ok().json("Deleted successfully"),
        Err(e) => HttpResponse::InternalServerError().json(format!("Failed to delete: {}", e)),
    }
}

pub async fn download_file(query: web::Query<FileRequest>) -> Result<NamedFile, Error> {
    let path = match validate_and_sanitize_path(&query.path, false) {
        Ok(p) => p,
        Err(err) => return Err(actix_web::error::ErrorBadRequest(err)),
    };

    if !path.exists() || path.is_dir() {
        return Err(actix_web::error::ErrorBadRequest(
            "Path does not exist or is a directory",
        ));
    }

    let metadata = fs::metadata(&path).map_err(actix_web::error::ErrorInternalServerError)?;
    if !metadata.is_file() {
        return Err(actix_web::error::ErrorBadRequest(
            "Only regular files can be downloaded",
        ));
    }

    NamedFile::open(path).map_err(|e| e.into())
}

pub async fn upload_file(
    query: web::Query<FileRequest>,
    mut payload: Multipart,
) -> Result<HttpResponse, Error> {
    let target_dir = match validate_and_sanitize_path(&query.path, true) {
        Ok(p) => p,
        Err(err) => return Ok(HttpResponse::BadRequest().json(err)),
    };

    if !target_dir.exists() || !target_dir.is_dir() {
        return Ok(HttpResponse::BadRequest().json("Target directory does not exist"));
    }

    while let Ok(Some(mut field)) = payload.try_next().await {
        let content_disposition = field.content_disposition();
        if let Some(filename) = content_disposition.get_filename() {
            // Strip any directory traversal components from the filename itself
            let safe_filename = match Path::new(filename).file_name() {
                Some(f) => f,
                None => continue,
            };

            let filepath = target_dir.join(safe_filename);
            let safe_filepath_str = filepath.to_string_lossy();
            if let Err(err) = validate_and_sanitize_path(&safe_filepath_str, true) {
                return Ok(HttpResponse::BadRequest().json(err));
            }

            let mut f = fs::File::create(&filepath)?;
            while let Ok(Some(chunk)) = field.try_next().await {
                f.write_all(&chunk)?;
            }
        }
    }

    Ok(HttpResponse::Ok().json("Upload complete"))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_path_sanitization_blocked() {
        assert!(validate_and_sanitize_path("", false).is_err());
        assert!(validate_and_sanitize_path("foo/../bar", false).is_err());
        assert!(validate_and_sanitize_path("/etc/shadow", false).is_err());
        assert!(validate_and_sanitize_path("/etc/shadow-", false).is_err());
        assert!(validate_and_sanitize_path("/etc/sudoers", false).is_err());
        assert!(validate_and_sanitize_path("/etc/sudoers.d/custom", true).is_err());
        assert!(validate_and_sanitize_path("/proc/sys/kernel", true).is_err());
        assert!(validate_and_sanitize_path("/etc/passwd", true).is_err());
        assert!(validate_and_sanitize_path("/etc/cron.d/backdoor", true).is_err());
        assert!(validate_and_sanitize_path("/root/.ssh/authorized_keys", true).is_err());
        assert!(validate_and_sanitize_path("/home/user/.ssh/authorized_keys", true).is_err());
        assert!(validate_and_sanitize_path("/root/.ssh/id_rsa", false).is_err());
        assert!(validate_and_sanitize_path("/home/user/.ssh/id_ed25519", false).is_err());
        assert!(validate_and_sanitize_path("/proc/1/environ", false).is_err());
        assert!(validate_and_sanitize_path("/", true).is_err());
        assert!(validate_and_sanitize_path(".wadm_jwt_secret", true).is_err());
        assert!(validate_and_sanitize_path("wadm-auth.json", true).is_err());
    }

    #[test]
    fn test_path_sanitization_hardened_invariants() {
        // 1. Write protection strictly forbids writing to /etc/passwd:
        let write_passwd = validate_and_sanitize_path("/etc/passwd", true);
        assert!(
            write_passwd.is_err(),
            "/etc/passwd must be forbidden for writing!"
        );

        // 2. Sensitive shadow backups and keys are blocked from reading:
        assert!(validate_and_sanitize_path("/etc/shadow-", false).is_err());
        assert!(validate_and_sanitize_path("/etc/shadow.bak", false).is_err());
        assert!(validate_and_sanitize_path("/root/.ssh/id_rsa", false).is_err());
        assert!(validate_and_sanitize_path("/root/.ssh/id_ed25519", false).is_err());

        // 3. Regular world-readable system files can be read if present:
        if Path::new("/etc/passwd").exists() {
            let read_passwd = validate_and_sanitize_path("/etc/passwd", false);
            assert!(
                read_passwd.is_ok(),
                "/etc/passwd read should be allowed for system inspection"
            );
        }
    }

    #[test]
    fn test_device_file_is_not_regular_file() {
        if Path::new("/dev/zero").exists() {
            let meta = fs::metadata("/dev/zero").unwrap();
            assert!(
                !meta.is_file(),
                "/dev/zero must not be considered a regular file"
            );
        }
    }
}
