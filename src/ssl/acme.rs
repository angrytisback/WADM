use chrono::{Duration, Utc};
use instant_acme::{
    Account, AccountCredentials, ChallengeType, Identifier, LetsEncrypt, NewAccount, NewOrder,
    RetryPolicy,
};
use rcgen::{CertificateParams, KeyPair};
use std::fs::File;
use std::io::Write;
use std::path::PathBuf;

use super::{get_cert_paths, get_certs_dir, AcmeChallengeStore};

pub struct AcmeIssueResult {
    pub cert_path: PathBuf,
    pub key_path: PathBuf,
    pub domain: String,
    pub expires_at: i64,
}

pub async fn issue_letsencrypt_certificate<F>(
    domain: &str,
    email: &str,
    challenge_store: &AcmeChallengeStore,
    progress_cb: F,
) -> Result<AcmeIssueResult, String>
where
    F: Fn(i32, &str) + Send + Sync,
{
    let domain = domain.trim();
    if domain.is_empty() {
        return Err("Domain name cannot be empty".to_string());
    }

    let email = email.trim();
    if email.is_empty() {
        return Err("Email address cannot be empty".to_string());
    }

    progress_cb(10, "Connecting to Let's Encrypt directory...");

    let directory_url = if std::env::var("WADM_ACME_STAGING").unwrap_or_default() == "1" {
        LetsEncrypt::Staging.url().to_string()
    } else {
        LetsEncrypt::Production.url().to_string()
    };

    let certs_dir = get_certs_dir();
    let account_file = certs_dir.join("account.json");

    let account = if account_file.exists() {
        progress_cb(15, "Loading existing ACME account credentials...");
        match std::fs::read_to_string(&account_file) {
            Ok(content) => match serde_json::from_str::<AccountCredentials>(&content) {
                Ok(creds) => {
                    match Account::builder()
                        .map_err(|e| format!("Failed to create AccountBuilder: {}", e))?
                        .from_credentials(creds)
                        .await
                    {
                        Ok(acc) => Some(acc),
                        Err(e) => {
                            log::warn!("Failed to resume ACME account: {}. Creating new one.", e);
                            None
                        }
                    }
                }
                Err(_) => None,
            },
            Err(_) => None,
        }
    } else {
        None
    };

    let account = match account {
        Some(acc) => acc,
        None => {
            progress_cb(20, "Registering new ACME account with Let's Encrypt...");
            let mailto = format!("mailto:{}", email);
            let (new_account, credentials) = Account::builder()
                .map_err(|e| format!("Failed to create AccountBuilder: {}", e))?
                .create(
                    &NewAccount {
                        contact: &[&mailto],
                        terms_of_service_agreed: true,
                        only_return_existing: false,
                    },
                    directory_url,
                    None,
                )
                .await
                .map_err(|e| format!("Failed to register ACME account: {}", e))?;

            if let Ok(creds_json) = serde_json::to_string(&credentials) {
                let _ = std::fs::write(&account_file, creds_json);
                #[cfg(unix)]
                {
                    use std::os::unix::fs::PermissionsExt;
                    let _ = std::fs::set_permissions(
                        &account_file,
                        std::fs::Permissions::from_mode(0o600),
                    );
                }
            }
            new_account
        }
    };

    progress_cb(35, &format!("Creating ACME order for domain: {}", domain));
    let identifier = Identifier::Dns(domain.to_string());
    let new_order = NewOrder::new(std::slice::from_ref(&identifier));
    let mut order = account
        .new_order(&new_order)
        .await
        .map_err(|e| format!("Failed to create ACME order: {}", e))?;

    progress_cb(50, "Setting up HTTP-01 challenge authorizations...");
    let mut tokens_to_clean = Vec::new();

    {
        let mut authorizations = order.authorizations();
        while let Some(auth_res) = authorizations.next().await {
            let mut auth = auth_res.map_err(|e| format!("Failed to fetch authorization: {}", e))?;

            if let Some(mut chal) = auth.challenge(ChallengeType::Http01) {
                let token = chal.token.clone();
                let key_auth = chal.key_authorization().as_str().to_string();

                tokens_to_clean.push(token.clone());
                challenge_store
                    .write()
                    .await
                    .insert(token.clone(), key_auth);

                progress_cb(
                    60,
                    &format!(
                        "HTTP-01 challenge active on /.well-known/acme-challenge/{}",
                        token
                    ),
                );

                chal.set_ready()
                    .await
                    .map_err(|e| format!("Failed to signal challenge readiness: {}", e))?;
            }
        }
    }

    progress_cb(70, "Awaiting challenge verification by Let's Encrypt...");
    let _order_status = order
        .poll_ready(&RetryPolicy::default())
        .await
        .map_err(|e| format!("ACME challenge validation failed: {}", e))?;

    // Cleanup challenge store
    {
        let mut store = challenge_store.write().await;
        for t in &tokens_to_clean {
            store.remove(t);
        }
    }

    progress_cb(
        80,
        "Validation successful! Generating CSR and private key...",
    );
    let key_pair = KeyPair::generate()
        .map_err(|e| format!("Failed to generate cryptographic key pair: {}", e))?;

    let params = CertificateParams::new(vec![domain.to_string()])
        .map_err(|e| format!("Failed to configure CertificateParams: {}", e))?;

    let csr = params
        .serialize_request(&key_pair)
        .map_err(|e| format!("Failed to serialize CSR: {}", e))?;

    progress_cb(85, "Submitting CSR to finalize certificate order...");
    order
        .finalize_csr(csr.der())
        .await
        .map_err(|e| format!("Failed to finalize CSR with ACME server: {}", e))?;

    progress_cb(90, "Downloading issued certificate chain...");
    let cert_chain_pem = order
        .poll_certificate(&RetryPolicy::default())
        .await
        .map_err(|e| format!("Failed to retrieve certificate chain: {}", e))?;

    let key_pem = key_pair.serialize_pem();

    let (cert_path, key_path) = get_cert_paths();

    {
        let mut f_cert = File::create(&cert_path)
            .map_err(|e| format!("Failed to write cert file at {:?}: {}", cert_path, e))?;
        f_cert
            .write_all(cert_chain_pem.as_bytes())
            .map_err(|e| format!("Failed to save certificate chain: {}", e))?;
    }

    {
        let mut f_key = File::create(&key_path)
            .map_err(|e| format!("Failed to write key file at {:?}: {}", key_path, e))?;
        f_key
            .write_all(key_pem.as_bytes())
            .map_err(|e| format!("Failed to save private key: {}", e))?;
    }

    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        let _ = std::fs::set_permissions(&key_path, std::fs::Permissions::from_mode(0o600));
        let _ = std::fs::set_permissions(&cert_path, std::fs::Permissions::from_mode(0o644));
    }

    // Let's Encrypt certificates are valid for 90 days
    let expires_at = (Utc::now() + Duration::days(90)).timestamp();

    progress_cb(
        100,
        &format!(
            "Certificate for '{}' issued successfully! (Valid until: {})",
            domain,
            Utc::now() + Duration::days(90)
        ),
    );

    Ok(AcmeIssueResult {
        cert_path,
        key_path,
        domain: domain.to_string(),
        expires_at,
    })
}
