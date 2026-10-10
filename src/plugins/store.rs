use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::RwLock;
use std::time::{Duration, Instant};

use crate::drivers::error::AppError;

pub const DEFAULT_CATALOG_URL: &str =
    "https://raw.githubusercontent.com/angrytisback/wadm-plugins-index/main/catalog.json";
pub const CATALOG_CACHE_TTL: Duration = Duration::from_secs(3600); // 1 hour

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct PluginDownloadAsset {
    pub url: String,
    pub sha256: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct PluginStoreCatalogItem {
    pub id: String,
    pub name: String,
    pub description: String,
    pub version: String,
    pub author: String,
    pub category: String, // "Networking", "Web Servers", "Security", "Databases"
    pub icon: String,     // React-icons key
    #[serde(default)]
    pub homepage: Option<String>,
    #[serde(default)]
    pub repository: String,
    #[serde(default)]
    pub capabilities: Vec<String>,
    #[serde(default)]
    pub downloads: HashMap<String, PluginDownloadAsset>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PluginStoreItemView {
    #[serde(flatten)]
    pub item: PluginStoreCatalogItem,
    pub is_installed: bool,
    pub installed_version: Option<String>,
    pub has_update: bool,
}

pub struct PluginStore {
    cache: RwLock<Option<(Instant, Vec<PluginStoreCatalogItem>)>>,
    catalog_url: String,
}

impl Default for PluginStore {
    fn default() -> Self {
        Self::new()
    }
}

impl PluginStore {
    pub fn new() -> Self {
        let catalog_url = std::env::var("WADM_PLUGIN_CATALOG_URL")
            .unwrap_or_else(|_| DEFAULT_CATALOG_URL.to_string());
        Self {
            cache: RwLock::new(None),
            catalog_url,
        }
    }

    #[allow(dead_code)]
    pub fn with_catalog_url(url: String) -> Self {
        Self {
            cache: RwLock::new(None),
            catalog_url: url,
        }
    }

    pub async fn get_catalog(
        &self,
        force_refresh: bool,
    ) -> Result<Vec<PluginStoreCatalogItem>, AppError> {
        if !force_refresh {
            let cache_guard = self.cache.read().map_err(|_| {
                AppError::ExecutionFailed("Plugin store cache lock poisoned".to_string())
            })?;
            if let Some((cached_at, ref items)) = *cache_guard {
                if cached_at.elapsed() < CATALOG_CACHE_TTL {
                    return Ok(items.clone());
                }
            }
        }

        // Fetch fresh catalog
        let fetched = match self.fetch_remote_catalog().await {
            Ok(items) => items,
            Err(e) => {
                log::warn!(
                    "Failed to fetch remote plugin catalog from '{}': {}. Falling back to default catalog.",
                    self.catalog_url,
                    e
                );
                // If remote fetch fails, check if we have an expired cache
                let cache_guard = self.cache.read().map_err(|_| {
                    AppError::ExecutionFailed("Plugin store cache lock poisoned".to_string())
                })?;
                if let Some((_, ref items)) = *cache_guard {
                    items.clone()
                } else {
                    Self::default_curated_catalog()
                }
            }
        };

        // Update cache
        if let Ok(mut write_guard) = self.cache.write() {
            *write_guard = Some((Instant::now(), fetched.clone()));
        }

        Ok(fetched)
    }

    pub async fn get_item(&self, id: &str) -> Result<Option<PluginStoreCatalogItem>, AppError> {
        let catalog = self.get_catalog(false).await?;
        Ok(catalog.into_iter().find(|item| item.id == id))
    }

    async fn fetch_remote_catalog(&self) -> Result<Vec<PluginStoreCatalogItem>, AppError> {
        let client = reqwest::Client::builder()
            .timeout(Duration::from_secs(10))
            .build()
            .map_err(|e| {
                AppError::ExecutionFailed(format!("Failed to build HTTP client: {}", e))
            })?;

        let response = client
            .get(&self.catalog_url)
            .header("User-Agent", "WADM-Plugin-Store/1.0")
            .send()
            .await
            .map_err(|e| AppError::ExecutionFailed(format!("HTTP request error: {}", e)))?;

        if !response.status().is_success() {
            return Err(AppError::ExecutionFailed(format!(
                "HTTP error {} fetching catalog",
                response.status()
            )));
        }

        let items: Vec<PluginStoreCatalogItem> = response
            .json()
            .await
            .map_err(|e| AppError::ExecutionFailed(format!("Invalid catalog JSON: {}", e)))?;

        Ok(items)
    }

    pub fn default_curated_catalog() -> Vec<PluginStoreCatalogItem> {
        vec![
            PluginStoreCatalogItem {
                id: "wadm-wireguard".to_string(),
                name: "WireGuard VPN Manager".to_string(),
                description: "Modern, high-performance VPN tunnel server and peer manager with QR codes."
                    .to_string(),
                version: "1.2.0".to_string(),
                author: "WADM Core Team".to_string(),
                category: "Networking".to_string(),
                icon: "FaShieldAlt".to_string(),
                homepage: Some("https://wadm.io/plugins/wireguard".to_string()),
                repository: "https://github.com/angrytisback/wadm-wireguard".to_string(),
                capabilities: vec!["network_admin".to_string(), "system_firewall".to_string()],
                downloads: {
                    let mut m = HashMap::new();
                    m.insert(
                        "x86_64-unknown-linux-gnu".to_string(),
                        PluginDownloadAsset {
                            url: "https://github.com/angrytisback/wadm-plugins/releases/download/v1.2.0/wadm-wireguard-x86_64.tar.gz".to_string(),
                            sha256: "e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855".to_string(),
                        },
                    );
                    m.insert(
                        "aarch64-unknown-linux-gnu".to_string(),
                        PluginDownloadAsset {
                            url: "https://github.com/angrytisback/wadm-plugins/releases/download/v1.2.0/wadm-wireguard-aarch64.tar.gz".to_string(),
                            sha256: "e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855".to_string(),
                        },
                    );
                    m
                },
            },
            PluginStoreCatalogItem {
                id: "wadm-nginx".to_string(),
                name: "Nginx Web & SSL Orchestrator".to_string(),
                description: "Visual VirtualHost editor, reverse proxy orchestrator, and upstream health checker."
                    .to_string(),
                version: "2.1.0".to_string(),
                author: "WADM Community".to_string(),
                category: "Web Servers".to_string(),
                icon: "FaServer".to_string(),
                homepage: Some("https://wadm.io/plugins/nginx".to_string()),
                repository: "https://github.com/angrytisback/wadm-nginx".to_string(),
                capabilities: vec!["web_server_manage".to_string(), "file_system".to_string()],
                downloads: {
                    let mut m = HashMap::new();
                    m.insert(
                        "x86_64-unknown-linux-gnu".to_string(),
                        PluginDownloadAsset {
                            url: "https://github.com/angrytisback/wadm-plugins/releases/download/v2.1.0/wadm-nginx-x86_64.tar.gz".to_string(),
                            sha256: "e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855".to_string(),
                        },
                    );
                    m
                },
            },
            PluginStoreCatalogItem {
                id: "wadm-fail2ban".to_string(),
                name: "Fail2ban Intrusion Defense".to_string(),
                description: "Live brute-force monitoring, jail statistics, and one-click unban management."
                    .to_string(),
                version: "1.0.4".to_string(),
                author: "WADM Security SIG".to_string(),
                category: "Security".to_string(),
                icon: "FaLock".to_string(),
                homepage: None,
                repository: "https://github.com/angrytisback/wadm-fail2ban".to_string(),
                capabilities: vec!["security_audit".to_string(), "firewall_rules".to_string()],
                downloads: {
                    let mut m = HashMap::new();
                    m.insert(
                        "x86_64-unknown-linux-gnu".to_string(),
                        PluginDownloadAsset {
                            url: "https://github.com/angrytisback/wadm-plugins/releases/download/v1.0.4/wadm-fail2ban-x86_64.tar.gz".to_string(),
                            sha256: "e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855".to_string(),
                        },
                    );
                    m
                },
            },
            PluginStoreCatalogItem {
                id: "wadm-postgres-studio".to_string(),
                name: "PostgreSQL Studio".to_string(),
                description: "Visual database navigator, table schemas, live query executor, and index insights."
                    .to_string(),
                version: "1.3.1".to_string(),
                author: "DataOps Labs".to_string(),
                category: "Databases".to_string(),
                icon: "FaDatabase".to_string(),
                homepage: None,
                repository: "https://github.com/angrytisback/wadm-postgres-studio".to_string(),
                capabilities: vec!["database_read".to_string(), "database_write".to_string()],
                downloads: {
                    let mut m = HashMap::new();
                    m.insert(
                        "x86_64-unknown-linux-gnu".to_string(),
                        PluginDownloadAsset {
                            url: "https://github.com/angrytisback/wadm-plugins/releases/download/v1.3.1/wadm-postgres-x86_64.tar.gz".to_string(),
                            sha256: "e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855".to_string(),
                        },
                    );
                    m
                },
            },
        ]
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_catalog_serialization_roundtrip() {
        let items = PluginStore::default_curated_catalog();
        let json = serde_json::to_string(&items).expect("Serialization failed");
        let parsed: Vec<PluginStoreCatalogItem> =
            serde_json::from_str(&json).expect("Deserialization failed");

        assert_eq!(items.len(), parsed.len());
        assert_eq!(items[0].id, parsed[0].id);
        assert_eq!(items[0].category, "Networking");
    }

    #[tokio::test]
    async fn test_plugin_store_fallback_and_cache() {
        // Construct with unreachable URL to force fallback
        let store =
            PluginStore::with_catalog_url("http://127.0.0.1:9/unreachable.json".to_string());

        let catalog = store
            .get_catalog(false)
            .await
            .expect("Catalog retrieval failed");
        assert!(!catalog.is_empty());
        assert!(catalog.iter().any(|i| i.id == "wadm-wireguard"));

        let item = store
            .get_item("wadm-wireguard")
            .await
            .expect("Item get failed");
        assert!(item.is_some());
        assert_eq!(item.unwrap().name, "WireGuard VPN Manager");
    }
}
