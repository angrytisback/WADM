use chrono::Utc;
use flate2::read::GzDecoder;
use sha2::{Digest, Sha256};
use std::path::{Path, PathBuf};
use std::sync::Arc;
use tar::Archive;
use tokio::sync::broadcast;

use super::manager::PluginManager;
use super::manifest::PluginManifest;
use crate::api::jobs::{JobDatabase, JobLogEvent};
use crate::drivers::error::AppError;

pub fn get_target_triple() -> &'static str {
    #[cfg(target_arch = "x86_64")]
    {
        "x86_64-unknown-linux-gnu"
    }
    #[cfg(target_arch = "aarch64")]
    {
        "aarch64-unknown-linux-gnu"
    }
    #[cfg(not(any(target_arch = "x86_64", target_arch = "aarch64")))]
    {
        "x86_64-unknown-linux-gnu"
    }
}

pub fn send_job_log(
    db: &Arc<JobDatabase>,
    tx_log: &broadcast::Sender<JobLogEvent>,
    job_id: &str,
    event_type: &str,
    line: String,
    status: &str,
    progress: i32,
) {
    let _ = db.append_log(job_id, &line);
    if progress >= 0 {
        let _ = db.update_status(job_id, status, progress, None);
    }
    let _ = tx_log.send(JobLogEvent {
        job_id: job_id.to_string(),
        event_type: event_type.to_string(),
        line,
        status: status.to_string(),
        progress,
        timestamp: Utc::now().to_rfc3339(),
    });
}

pub async fn download_and_verify(
    url: &str,
    expected_sha256: &str,
    db: &Arc<JobDatabase>,
    tx_log: &broadcast::Sender<JobLogEvent>,
    job_id: &str,
) -> Result<Vec<u8>, AppError> {
    send_job_log(
        db,
        tx_log,
        job_id,
        "status",
        format!("Connecting to download source: {}", url),
        "running",
        20,
    );

    let bytes = if let Some(path_str) = url.strip_prefix("file://") {
        send_job_log(
            db,
            tx_log,
            job_id,
            "log",
            format!("Reading local archive: {}", path_str),
            "running",
            30,
        );
        tokio::fs::read(path_str).await.map_err(|e| {
            AppError::ExecutionFailed(format!(
                "Failed to read local archive '{}': {}",
                path_str, e
            ))
        })?
    } else {
        let client = reqwest::Client::builder()
            .timeout(std::time::Duration::from_secs(120))
            .build()
            .map_err(|e| {
                AppError::ExecutionFailed(format!("Failed to build HTTP client: {}", e))
            })?;

        let response = client
            .get(url)
            .header("User-Agent", "WADM-Plugin-Installer/1.0")
            .send()
            .await
            .map_err(|e| AppError::ExecutionFailed(format!("Download request failed: {}", e)))?;

        if !response.status().is_success() {
            return Err(AppError::ExecutionFailed(format!(
                "HTTP error {} downloading plugin archive from {}",
                response.status(),
                url
            )));
        }

        send_job_log(
            db,
            tx_log,
            job_id,
            "log",
            "Transferring archive payload...".to_string(),
            "running",
            50,
        );

        response
            .bytes()
            .await
            .map_err(|e| {
                AppError::ExecutionFailed(format!("Failed to read download stream: {}", e))
            })?
            .to_vec()
    };

    send_job_log(
        db,
        tx_log,
        job_id,
        "status",
        "Calculating cryptographic SHA-256 hash...".to_string(),
        "running",
        70,
    );

    let mut hasher = Sha256::new();
    hasher.update(&bytes);
    let actual_sha256 = format!("{:x}", hasher.finalize());

    if !actual_sha256.eq_ignore_ascii_case(expected_sha256) {
        let err_msg = format!(
            "Cryptographic verification failed: expected SHA-256 '{}', got '{}'",
            expected_sha256, actual_sha256
        );
        send_job_log(db, tx_log, job_id, "error", err_msg.clone(), "failed", -1);
        return Err(AppError::InvalidInput(err_msg));
    }

    send_job_log(
        db,
        tx_log,
        job_id,
        "log",
        format!("SHA-256 integrity verified successfully: {}", actual_sha256),
        "running",
        75,
    );

    Ok(bytes)
}

pub fn safe_extract(
    archive_bytes: &[u8],
    target_dir: &Path,
    expected_plugin_id: &str,
    db: &Arc<JobDatabase>,
    tx_log: &broadcast::Sender<JobLogEvent>,
    job_id: &str,
) -> Result<(), AppError> {
    send_job_log(
        db,
        tx_log,
        job_id,
        "status",
        format!("Unpacking archive securely into {}", target_dir.display()),
        "running",
        80,
    );

    let gz = GzDecoder::new(archive_bytes);
    let mut archive = Archive::new(gz);

    for entry_result in archive.entries().map_err(|e| {
        AppError::ExecutionFailed(format!("Invalid or corrupt gzip/tar archive: {}", e))
    })? {
        let mut entry = entry_result.map_err(|e| {
            AppError::ExecutionFailed(format!("Failed to read archive header: {}", e))
        })?;

        let entry_path = entry
            .path()
            .map_err(|e| AppError::ExecutionFailed(format!("Invalid path in archive: {}", e)))?
            .to_path_buf();

        // 1. Prevent absolute path traversal
        if entry_path.is_absolute() {
            return Err(AppError::InvalidInput(format!(
                "Path traversal attack detected: absolute path '{}' in archive",
                entry_path.display()
            )));
        }

        // 2. Prevent directory traversal components (.., /, prefix)
        for component in entry_path.components() {
            match component {
                std::path::Component::ParentDir => {
                    return Err(AppError::InvalidInput(format!(
                        "Path traversal attack detected: parent traversal '..' in entry '{}'",
                        entry_path.display()
                    )));
                }
                std::path::Component::RootDir | std::path::Component::Prefix(_) => {
                    return Err(AppError::InvalidInput(format!(
                        "Path traversal attack detected: root/prefix component in entry '{}'",
                        entry_path.display()
                    )));
                }
                _ => {}
            }
        }

        // 3. Reject symlinks / hardlinks to block symlink attacks
        let entry_type = entry.header().entry_type();
        if entry_type.is_symlink() || entry_type.is_hard_link() {
            return Err(AppError::InvalidInput(format!(
                "Symlink / hardlink entries are forbidden in plugin packages: '{}'",
                entry_path.display()
            )));
        }

        let destination = target_dir.join(&entry_path);

        // Ensure parent directory exists
        if let Some(parent) = destination.parent() {
            std::fs::create_dir_all(parent).map_err(|e| {
                AppError::ExecutionFailed(format!(
                    "Failed to create target subdirectory '{}': {}",
                    parent.display(),
                    e
                ))
            })?;
        }

        entry.unpack(&destination).map_err(|e| {
            AppError::ExecutionFailed(format!(
                "Failed to unpack entry '{}': {}",
                destination.display(),
                e
            ))
        })?;

        // On Unix, ensure proper execution bits if the header specifies executable permissions
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            if let Ok(metadata) = std::fs::metadata(&destination) {
                if metadata.is_file() {
                    let mode = entry.header().mode().unwrap_or(0o644);
                    if mode & 0o111 != 0 {
                        let _ = std::fs::set_permissions(
                            &destination,
                            std::fs::Permissions::from_mode(0o755),
                        );
                    }
                }
            }
        }
    }

    // 4. Validate manifest.json
    send_job_log(
        db,
        tx_log,
        job_id,
        "status",
        "Validating plugin manifest.json...".to_string(),
        "running",
        88,
    );

    let manifest_path = target_dir.join("manifest.json");
    if !manifest_path.is_file() {
        return Err(AppError::InvalidInput(
            "Plugin package is missing mandatory 'manifest.json' file".to_string(),
        ));
    }

    let manifest_content = std::fs::read_to_string(&manifest_path)
        .map_err(|e| AppError::ExecutionFailed(format!("Failed to read 'manifest.json': {}", e)))?;

    let manifest: PluginManifest = serde_json::from_str(&manifest_content).map_err(|e| {
        AppError::InvalidInput(format!(
            "Plugin 'manifest.json' contains invalid JSON: {}",
            e
        ))
    })?;

    if manifest.id != expected_plugin_id {
        return Err(AppError::InvalidInput(format!(
            "Package manifest id '{}' does not match expected plugin id '{}'",
            manifest.id, expected_plugin_id
        )));
    }

    // 5. Ensure the executable binary exists and has execute permissions (0o755)
    let exec_path = if Path::new(&manifest.executable).is_absolute() {
        PathBuf::from(&manifest.executable)
    } else {
        target_dir.join(&manifest.executable)
    };

    if !exec_path.exists() {
        return Err(AppError::NotFound(format!(
            "Plugin executable '{}' declared in manifest not found in package",
            exec_path.display()
        )));
    }

    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        let _ = std::fs::set_permissions(&exec_path, std::fs::Permissions::from_mode(0o755));
    }

    send_job_log(
        db,
        tx_log,
        job_id,
        "log",
        format!(
            "Manifest validated: '{}' v{} with executable '{}'",
            manifest.name,
            manifest.version,
            exec_path.display()
        ),
        "running",
        90,
    );

    Ok(())
}

pub async fn install_plugin(
    job_id: &str,
    plugin_id: &str,
    download_url: &str,
    expected_sha256: &str,
    db: &Arc<JobDatabase>,
    tx_log: &broadcast::Sender<JobLogEvent>,
    manager: &Arc<PluginManager>,
) -> Result<String, String> {
    send_job_log(
        db,
        tx_log,
        job_id,
        "status",
        format!(
            "Starting installation pipeline for plugin '{}'...",
            plugin_id
        ),
        "running",
        10,
    );

    // Stop existing instance if running
    send_job_log(
        db,
        tx_log,
        job_id,
        "log",
        format!(
            "Checking and stopping any running instance of '{}'...",
            plugin_id
        ),
        "running",
        15,
    );
    let _ = manager.stop_plugin(plugin_id).await;

    // Download & Verify
    let archive_bytes = download_and_verify(download_url, expected_sha256, db, tx_log, job_id)
        .await
        .map_err(|e| format!("Download/verification failed: {}", e))?;

    // Prepare target folder
    let target_dir = manager.plugins_dir.join(plugin_id);
    if target_dir.exists() {
        send_job_log(
            db,
            tx_log,
            job_id,
            "log",
            format!(
                "Cleaning prior installation directory '{}'...",
                target_dir.display()
            ),
            "running",
            78,
        );
        let _ = std::fs::remove_dir_all(&target_dir);
    }
    std::fs::create_dir_all(&target_dir).map_err(|e| {
        format!(
            "Failed to create destination directory '{}': {}",
            target_dir.display(),
            e
        )
    })?;

    // Safe extraction
    if let Err(e) = safe_extract(&archive_bytes, &target_dir, plugin_id, db, tx_log, job_id) {
        let _ = std::fs::remove_dir_all(&target_dir);
        return Err(format!("Extraction error: {}", e));
    }

    // Try starting the newly installed plugin
    send_job_log(
        db,
        tx_log,
        job_id,
        "status",
        format!("Launching plugin daemon for '{}'...", plugin_id),
        "running",
        95,
    );

    match manager.start_plugin(plugin_id).await {
        Ok(info) => {
            let success_msg = format!(
                "Plugin '{}' v{} installed and started successfully (PID: {:?})",
                info.manifest.name, info.manifest.version, info.pid
            );
            send_job_log(
                db,
                tx_log,
                job_id,
                "log",
                success_msg.clone(),
                "running",
                100,
            );
            Ok(success_msg)
        }
        Err(e) => {
            let warn_msg = format!(
                "Plugin '{}' installed successfully, but daemon start failed: {}. It can be inspected or started manually.",
                plugin_id, e
            );
            send_job_log(db, tx_log, job_id, "log", warn_msg.clone(), "running", 100);
            Ok(warn_msg)
        }
    }
}

pub async fn uninstall_plugin(
    job_id: &str,
    plugin_id: &str,
    db: &Arc<JobDatabase>,
    tx_log: &broadcast::Sender<JobLogEvent>,
    manager: &Arc<PluginManager>,
) -> Result<String, String> {
    send_job_log(
        db,
        tx_log,
        job_id,
        "status",
        format!(
            "Starting uninstallation pipeline for plugin '{}'...",
            plugin_id
        ),
        "running",
        10,
    );

    // Stop process if active
    send_job_log(
        db,
        tx_log,
        job_id,
        "log",
        format!("Stopping plugin '{}'...", plugin_id),
        "running",
        30,
    );
    let _ = manager.stop_plugin(plugin_id).await;

    // Remove socket if present
    let socket_path = manager.sockets_dir.join(format!("{}.sock", plugin_id));
    if socket_path.exists() {
        let _ = std::fs::remove_file(&socket_path);
    }

    // Remove plugin files
    let plugin_dir = manager.plugins_dir.join(plugin_id);
    send_job_log(
        db,
        tx_log,
        job_id,
        "log",
        format!("Deleting plugin files at '{}'...", plugin_dir.display()),
        "running",
        70,
    );

    if plugin_dir.exists() {
        std::fs::remove_dir_all(&plugin_dir).map_err(|e| {
            format!(
                "Failed to delete plugin files at '{}': {}",
                plugin_dir.display(),
                e
            )
        })?;
    }

    let success_msg = format!("Plugin '{}' uninstalled cleanly from system", plugin_id);
    send_job_log(
        db,
        tx_log,
        job_id,
        "log",
        success_msg.clone(),
        "running",
        100,
    );
    Ok(success_msg)
}

#[cfg(test)]
mod tests {
    use super::*;
    use flate2::write::GzEncoder;
    use flate2::Compression;
    use std::io::Write;

    fn build_test_archive(entries: &[(&str, &[u8], u32)]) -> (Vec<u8>, String) {
        let mut tar_builder = tar::Builder::new(Vec::new());

        for (path, content, mode) in entries {
            let mut header = tar::Header::new_gnu();
            header.set_size(content.len() as u64);
            header.set_mode(*mode);
            header.set_cksum();
            tar_builder
                .append_data(&mut header, *path, *content)
                .unwrap();
        }

        let tar_bytes = tar_builder.into_inner().unwrap();
        let mut gz_encoder = GzEncoder::new(Vec::new(), Compression::default());
        gz_encoder.write_all(&tar_bytes).unwrap();
        let gz_bytes = gz_encoder.finish().unwrap();

        let mut hasher = Sha256::new();
        hasher.update(&gz_bytes);
        let sha256 = format!("{:x}", hasher.finalize());

        (gz_bytes, sha256)
    }

    fn build_raw_tar_gz(path: &str, content: &[u8]) -> Vec<u8> {
        let mut header = [0u8; 512];
        let path_bytes = path.as_bytes();
        header[..path_bytes.len()].copy_from_slice(path_bytes);
        header[100..108].copy_from_slice(b"0000644\0");
        header[108..116].copy_from_slice(b"0000000\0");
        header[116..124].copy_from_slice(b"0000000\0");
        let size_octal = format!("{:011o}\0", content.len());
        header[124..136].copy_from_slice(size_octal.as_bytes());
        header[136..148].copy_from_slice(b"00000000000\0");
        header[156] = b'0';
        for b in &mut header[148..156] {
            *b = b' ';
        }
        let sum: u32 = header.iter().map(|&b| b as u32).sum();
        let sum_octal = format!("{:06o}\0 ", sum);
        header[148..156].copy_from_slice(sum_octal.as_bytes());

        let mut tar_data = Vec::new();
        tar_data.extend_from_slice(&header);
        tar_data.extend_from_slice(content);
        let rem = content.len() % 512;
        if rem > 0 {
            tar_data.extend_from_slice(&vec![0u8; 512 - rem]);
        }
        tar_data.extend_from_slice(&[0u8; 1024]);

        let mut encoder = GzEncoder::new(Vec::new(), Compression::default());
        encoder.write_all(&tar_data).unwrap();
        encoder.finish().unwrap()
    }

    #[test]
    fn test_safe_extract_success() {
        let temp_dir =
            std::env::temp_dir().join(format!("wadm_install_test_{}", uuid::Uuid::new_v4()));
        std::fs::create_dir_all(&temp_dir).unwrap();

        let manifest_json = r#"{
            "id": "test-pkg",
            "name": "Test Package",
            "version": "1.0.0",
            "description": "Test",
            "author": "Tester",
            "executable": "test-bin",
            "ui": {
                "tab_id": "test-tab",
                "title": "Test Tab",
                "icon": "FaCog",
                "entry_type": "declarative_schema"
            },
            "capabilities": ["test"]
        }"#;

        let script = "#!/bin/sh\necho ok\n";

        let (archive_bytes, sha256) = build_test_archive(&[
            ("manifest.json", manifest_json.as_bytes(), 0o644),
            ("test-bin", script.as_bytes(), 0o755),
        ]);

        let job_db = Arc::new(JobDatabase::new_in_memory().unwrap());
        let (tx, _rx) = broadcast::channel(16);

        let extract_res =
            safe_extract(&archive_bytes, &temp_dir, "test-pkg", &job_db, &tx, "job-1");
        assert!(extract_res.is_ok());
        assert!(temp_dir.join("manifest.json").exists());
        assert!(temp_dir.join("test-bin").exists());

        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            let meta = std::fs::metadata(temp_dir.join("test-bin")).unwrap();
            assert_eq!(meta.permissions().mode() & 0o111, 0o111);
        }

        assert_eq!(sha256.len(), 64);
        let _ = std::fs::remove_dir_all(&temp_dir);
    }

    #[test]
    fn test_safe_extract_rejects_path_traversal() {
        let temp_dir =
            std::env::temp_dir().join(format!("wadm_install_bad_{}", uuid::Uuid::new_v4()));
        std::fs::create_dir_all(&temp_dir).unwrap();

        let archive_bytes = build_raw_tar_gz("../../../evil.txt", b"malicious payload");

        let job_db = Arc::new(JobDatabase::new_in_memory().unwrap());
        let (tx, _rx) = broadcast::channel(16);

        let res = safe_extract(&archive_bytes, &temp_dir, "evil-pkg", &job_db, &tx, "job-2");
        assert!(res.is_err());
        let err_str = res.unwrap_err().to_string();
        assert!(err_str.contains("Path traversal attack detected"));

        let _ = std::fs::remove_dir_all(&temp_dir);
    }

    #[test]
    fn test_safe_extract_rejects_absolute_path() {
        let temp_dir =
            std::env::temp_dir().join(format!("wadm_install_abs_{}", uuid::Uuid::new_v4()));
        std::fs::create_dir_all(&temp_dir).unwrap();

        let archive_bytes = build_raw_tar_gz("/etc/evil.conf", b"malicious payload");

        let job_db = Arc::new(JobDatabase::new_in_memory().unwrap());
        let (tx, _rx) = broadcast::channel(16);

        let res = safe_extract(
            &archive_bytes,
            &temp_dir,
            "evil-abs",
            &job_db,
            &tx,
            "job-2b",
        );
        assert!(res.is_err());
        let err_str = res.unwrap_err().to_string();
        assert!(err_str.contains("Path traversal attack detected"));

        let _ = std::fs::remove_dir_all(&temp_dir);
    }

    #[test]
    fn test_safe_extract_rejects_id_mismatch() {
        let temp_dir =
            std::env::temp_dir().join(format!("wadm_install_mismatch_{}", uuid::Uuid::new_v4()));
        std::fs::create_dir_all(&temp_dir).unwrap();

        let manifest_json = r#"{
            "id": "actual-id",
            "name": "Actual",
            "version": "1.0.0",
            "description": "Test",
            "author": "Tester",
            "executable": "test-bin",
            "ui": {
                "tab_id": "test-tab",
                "title": "Test Tab",
                "icon": "FaCog",
                "entry_type": "declarative_schema"
            },
            "capabilities": []
        }"#;

        let (archive_bytes, _) = build_test_archive(&[
            ("manifest.json", manifest_json.as_bytes(), 0o644),
            ("test-bin", b"test", 0o755),
        ]);

        let job_db = Arc::new(JobDatabase::new_in_memory().unwrap());
        let (tx, _rx) = broadcast::channel(16);

        let res = safe_extract(
            &archive_bytes,
            &temp_dir,
            "expected-id",
            &job_db,
            &tx,
            "job-3",
        );
        assert!(res.is_err());
        let err_str = res.unwrap_err().to_string();
        assert!(err_str.contains("does not match expected plugin id"));

        let _ = std::fs::remove_dir_all(&temp_dir);
    }

    #[tokio::test]
    async fn test_download_and_verify_file_url() {
        let temp_dir =
            std::env::temp_dir().join(format!("wadm_verify_test_{}", uuid::Uuid::new_v4()));
        std::fs::create_dir_all(&temp_dir).unwrap();

        let (archive_bytes, sha256) = build_test_archive(&[("manifest.json", b"{}", 0o644)]);

        let archive_path = temp_dir.join("test.tar.gz");
        std::fs::write(&archive_path, &archive_bytes).unwrap();

        let file_url = format!("file://{}", archive_path.display());
        let job_db = Arc::new(JobDatabase::new_in_memory().unwrap());
        let (tx, _rx) = broadcast::channel(16);

        // Success case
        let result = download_and_verify(&file_url, &sha256, &job_db, &tx, "job-4").await;
        assert!(result.is_ok());
        assert_eq!(result.unwrap(), archive_bytes);

        // Mismatch case
        let bad_sha = "0000000000000000000000000000000000000000000000000000000000000000";
        let fail_result = download_and_verify(&file_url, bad_sha, &job_db, &tx, "job-5").await;
        assert!(fail_result.is_err());

        let _ = std::fs::remove_dir_all(&temp_dir);
    }
}
