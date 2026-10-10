use chrono::{Duration, Utc};
use std::fs::File;
use std::io::Write;
use std::path::PathBuf;

use super::get_cert_paths;

pub struct GeneratedCert {
    pub cert_path: PathBuf,
    pub key_path: PathBuf,
    pub issuer: String,
    pub expires_at: i64,
}

pub fn generate_self_signed(domain: Option<&str>) -> Result<GeneratedCert, String> {
    let (cert_path, key_path) = get_cert_paths();
    generate_self_signed_to_paths(domain, cert_path, key_path)
}

pub fn generate_self_signed_to_paths(
    domain: Option<&str>,
    cert_path: PathBuf,
    key_path: PathBuf,
) -> Result<GeneratedCert, String> {
    let mut sans = vec![
        "localhost".to_string(),
        "127.0.0.1".to_string(),
        "0.0.0.0".to_string(),
    ];

    if let Some(d) = domain {
        let trimmed = d.trim();
        if !trimmed.is_empty() && !sans.iter().any(|s| s == trimmed) {
            sans.push(trimmed.to_string());
        }
    }

    let certified_key = rcgen::generate_simple_self_signed(sans)
        .map_err(|e| format!("Failed to generate self-signed certificate: {}", e))?;

    let cert_pem = certified_key.cert.pem();
    let key_pem = certified_key.signing_key.serialize_pem();

    if let Some(parent) = cert_path.parent() {
        let _ = std::fs::create_dir_all(parent);
    }
    if let Some(parent) = key_path.parent() {
        let _ = std::fs::create_dir_all(parent);
    }

    {
        let mut f_cert = File::create(&cert_path).map_err(|e| {
            format!(
                "Failed to create certificate file at {:?}: {}",
                cert_path, e
            )
        })?;
        f_cert
            .write_all(cert_pem.as_bytes())
            .map_err(|e| format!("Failed to write certificate: {}", e))?;
    }

    {
        let mut f_key = File::create(&key_path)
            .map_err(|e| format!("Failed to create private key file at {:?}: {}", key_path, e))?;
        f_key
            .write_all(key_pem.as_bytes())
            .map_err(|e| format!("Failed to write private key: {}", e))?;
    }

    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        let _ = std::fs::set_permissions(&key_path, std::fs::Permissions::from_mode(0o600));
        let _ = std::fs::set_permissions(&cert_path, std::fs::Permissions::from_mode(0o644));
    }

    // Default rcgen validity is ~4096 days (~11 years)
    let expires_at = (Utc::now() + Duration::days(3650)).timestamp();

    Ok(GeneratedCert {
        cert_path,
        key_path,
        issuer: "WADM Self-Signed CA".to_string(),
        expires_at,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_generate_self_signed() {
        let dir = std::env::temp_dir().join(format!("wadm_test_gen_{}", uuid::Uuid::new_v4()));
        let cert_path = dir.join("wadm.crt");
        let key_path = dir.join("wadm.key");
        let res = generate_self_signed_to_paths(Some("wadm.local"), cert_path, key_path).unwrap();
        assert!(res.cert_path.exists());
        assert!(res.key_path.exists());
        assert_eq!(res.issuer, "WADM Self-Signed CA");
        assert!(res.expires_at > Utc::now().timestamp());

        let cert_content = std::fs::read_to_string(&res.cert_path).unwrap();
        assert!(cert_content.contains("BEGIN CERTIFICATE"));

        let key_content = std::fs::read_to_string(&res.key_path).unwrap();
        assert!(key_content.contains("BEGIN PRIVATE KEY"));

        let _ = std::fs::remove_dir_all(dir);
    }
}
