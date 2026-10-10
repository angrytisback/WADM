use chrono::Utc;
use std::sync::Arc;
use tokio::time::{sleep, Duration};

use super::db::SslDatabase;
use crate::api::jobs::{JobManager, JobTaskPayload};

pub fn start_renewal_daemon(ssl_db: Arc<SslDatabase>, job_manager: Arc<JobManager>) {
    tokio::spawn(async move {
        // Initial grace period on startup
        sleep(Duration::from_secs(60)).await;

        loop {
            if let Ok(config) = ssl_db.get_config() {
                if config.enabled && config.mode == "lets_encrypt" && config.auto_renew {
                    if let Some(expires_at) = config.expires_at {
                        let now = Utc::now().timestamp();
                        let remaining_seconds = expires_at - now;
                        let thirty_days_sec = 30 * 86400;

                        if remaining_seconds < thirty_days_sec {
                            let days_left = remaining_seconds / 86400;
                            log::info!(
                                "SSL certificate for domain {:?} expires in {} days. Triggering auto-renewal...",
                                config.domain,
                                days_left
                            );

                            if let (Some(domain), Some(email)) =
                                (config.domain.clone(), config.email.clone())
                            {
                                match job_manager
                                    .enqueue_job(
                                        "acme_certificate_renew",
                                        JobTaskPayload::AcmeIssueCertificate { domain, email },
                                    )
                                    .await
                                {
                                    Ok(job_id) => {
                                        log::info!("Queued ACME auto-renewal job: {}", job_id);
                                    }
                                    Err(e) => {
                                        log::error!("Failed to enqueue ACME renewal job: {}", e);
                                    }
                                }
                            } else {
                                log::warn!("Cannot auto-renew ACME cert: domain or email missing in config");
                            }
                        }
                    }
                }
            }

            // Check once every 24 hours
            sleep(Duration::from_secs(24 * 3600)).await;
        }
    });
}
