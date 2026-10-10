use std::collections::HashMap;
use std::sync::Arc;

use super::database::{DatabaseDriver, MySqlDriver, PostgresDriver};
use super::firewall::{FirewallDriver, UfwDriver};
use super::package::{AptDriver, DnfDriver, PackageManagerDriver, PacmanDriver};

#[derive(Clone)]
pub struct DriverRegistry {
    pub package_manager: Arc<dyn PackageManagerDriver>,
    pub databases: HashMap<String, Arc<dyn DatabaseDriver>>,
    pub firewall: Arc<dyn FirewallDriver>,
}

impl DriverRegistry {
    #[allow(dead_code)]
    pub fn new(
        package_manager: Arc<dyn PackageManagerDriver>,
        databases: HashMap<String, Arc<dyn DatabaseDriver>>,
        firewall: Arc<dyn FirewallDriver>,
    ) -> Self {
        Self {
            package_manager,
            databases,
            firewall,
        }
    }

    pub fn detect_and_init() -> Self {
        log::info!("Detecting and initializing system drivers...");

        // 1. Detect default package manager
        let pm: Arc<dyn PackageManagerDriver> = if AptDriver.is_available() {
            log::info!("Detected package manager: APT (AptDriver)");
            Arc::new(AptDriver)
        } else if DnfDriver.is_available() {
            log::info!("Detected package manager: DNF (DnfDriver)");
            Arc::new(DnfDriver)
        } else if PacmanDriver.is_available() {
            log::info!("Detected package manager: PACMAN (PacmanDriver)");
            Arc::new(PacmanDriver)
        } else {
            log::warn!("No known package manager binary found, falling back to AptDriver default");
            Arc::new(AptDriver)
        };

        // 2. Register database drivers
        let mut databases: HashMap<String, Arc<dyn DatabaseDriver>> = HashMap::new();
        databases.insert("mysql".to_string(), Arc::new(MySqlDriver));
        databases.insert("postgres".to_string(), Arc::new(PostgresDriver));
        log::info!("Registered database drivers: mysql (MySqlDriver), postgres (PostgresDriver)");

        // 3. Register firewall driver
        let firewall: Arc<dyn FirewallDriver> = Arc::new(UfwDriver);
        log::info!(
            "Registered firewall driver: {} (UfwDriver)",
            firewall.backend_name()
        );

        Self {
            package_manager: pm,
            databases,
            firewall,
        }
    }

    pub fn get_database(&self, engine: &str) -> Option<Arc<dyn DatabaseDriver>> {
        self.databases.get(engine).cloned()
    }
}
