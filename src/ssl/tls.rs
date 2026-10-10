use rustls::pki_types::{CertificateDer, PrivateKeyDer};
use std::fs::File;
use std::io::BufReader;
use std::path::Path;

pub fn load_certs(cert_path: &Path) -> Result<Vec<CertificateDer<'static>>, String> {
    let file = File::open(cert_path)
        .map_err(|e| format!("Failed to open certificate file at {:?}: {}", cert_path, e))?;
    let mut reader = BufReader::new(file);

    let certs: Vec<CertificateDer<'static>> = rustls_pemfile::certs(&mut reader)
        .collect::<Result<Vec<_>, _>>()
        .map_err(|e| format!("Failed to parse certificates at {:?}: {}", cert_path, e))?;

    if certs.is_empty() {
        return Err(format!("No certificates found in {:?}", cert_path));
    }

    Ok(certs)
}

pub fn load_private_key(key_path: &Path) -> Result<PrivateKeyDer<'static>, String> {
    let file = File::open(key_path)
        .map_err(|e| format!("Failed to open private key file at {:?}: {}", key_path, e))?;
    let mut reader = BufReader::new(file);

    let key_opt = rustls_pemfile::private_key(&mut reader)
        .map_err(|e| format!("Failed to parse private key at {:?}: {}", key_path, e))?;

    key_opt.ok_or_else(|| format!("No private key found in {:?}", key_path))
}

pub fn build_server_config(
    cert_path: &Path,
    key_path: &Path,
) -> Result<rustls::ServerConfig, String> {
    let _ = rustls::crypto::ring::default_provider().install_default();
    let cert_chain = load_certs(cert_path)?;
    let key = load_private_key(key_path)?;

    rustls::ServerConfig::builder()
        .with_no_client_auth()
        .with_single_cert(cert_chain, key)
        .map_err(|e| format!("Invalid TLS certificate/key pair: {}", e))
}

pub fn validate_and_parse_pem(
    cert_pem: &str,
    key_pem: &str,
) -> Result<(Vec<CertificateDer<'static>>, PrivateKeyDer<'static>), String> {
    let _ = rustls::crypto::ring::default_provider().install_default();
    let mut cert_reader = BufReader::new(cert_pem.as_bytes());
    let certs: Vec<CertificateDer<'static>> = rustls_pemfile::certs(&mut cert_reader)
        .collect::<Result<Vec<_>, _>>()
        .map_err(|e| format!("Invalid certificate PEM: {}", e))?;

    if certs.is_empty() {
        return Err("No certificates found in provided PEM data".to_string());
    }

    let mut key_reader = BufReader::new(key_pem.as_bytes());
    let key = rustls_pemfile::private_key(&mut key_reader)
        .map_err(|e| format!("Invalid private key PEM: {}", e))?
        .ok_or_else(|| "No valid private key found in provided PEM data".to_string())?;

    // Verify key pairs with certificate by attempting to build ServerConfig
    rustls::ServerConfig::builder()
        .with_no_client_auth()
        .with_single_cert(certs.clone(), key.clone_key())
        .map_err(|e| format!("Certificate does not match private key: {}", e))?;

    Ok((certs, key))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ssl::generator::generate_self_signed_to_paths;

    #[test]
    fn test_build_server_config_from_generated_cert() {
        let dir = std::env::temp_dir().join(format!("wadm_test_tls_{}", uuid::Uuid::new_v4()));
        let cert_path = dir.join("tls_test.crt");
        let key_path = dir.join("tls_test.key");
        let generated =
            generate_self_signed_to_paths(Some("tls-test.local"), cert_path, key_path).unwrap();
        let config_res = build_server_config(&generated.cert_path, &generated.key_path);
        assert!(config_res.is_ok());

        let cert_content = std::fs::read_to_string(&generated.cert_path).unwrap();
        let key_content = std::fs::read_to_string(&generated.key_path).unwrap();

        let val_res = validate_and_parse_pem(&cert_content, &key_content);
        assert!(val_res.is_ok());

        let _ = std::fs::remove_dir_all(dir);
    }
}
