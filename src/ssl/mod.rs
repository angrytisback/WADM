pub mod acme;
pub mod db;
pub mod generator;
pub mod renewal;
pub mod tls;

#[allow(unused_imports)]
pub use db::{CertMetaUpdate, SslConfig, SslDatabase};

use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::Arc;
use tokio::sync::RwLock;

pub type AcmeChallengeStore = Arc<RwLock<HashMap<String, String>>>;

pub fn create_challenge_store() -> AcmeChallengeStore {
    Arc::new(RwLock::new(HashMap::new()))
}

pub fn get_certs_dir() -> PathBuf {
    let dir = if let Ok(custom_dir) = std::env::var("WADM_DATA_DIR") {
        PathBuf::from(custom_dir).join("certs")
    } else {
        let system_dir = PathBuf::from("/var/lib/wadm/certs");
        if system_dir.exists() || std::fs::create_dir_all(&system_dir).is_ok() {
            system_dir
        } else {
            let local_dir = PathBuf::from("./data/certs");
            let _ = std::fs::create_dir_all(&local_dir);
            local_dir
        }
    };

    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        let _ = std::fs::set_permissions(&dir, std::fs::Permissions::from_mode(0o700));
    }

    dir
}

pub fn get_cert_paths() -> (PathBuf, PathBuf) {
    let certs_dir = get_certs_dir();
    (
        certs_dir.join("fullchain.pem"),
        certs_dir.join("privkey.pem"),
    )
}
