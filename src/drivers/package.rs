use super::error::AppError;
use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use std::process::Command as StdCommand;
use tokio::process::Command as TokioCommand;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct PackageUpdate {
    pub name: String,
    pub version: String,
    pub status: String,
}

pub fn is_valid_package_name(name: &str) -> bool {
    !name.is_empty()
        && !name.starts_with('-')
        && name
            .chars()
            .all(|c| c.is_ascii_alphanumeric() || c == '_' || c == '-' || c == '.')
}

#[async_trait]
pub trait PackageManagerDriver: Send + Sync {
    fn name(&self) -> &'static str;
    async fn check_updates(&self) -> Result<Vec<PackageUpdate>, AppError>;
    async fn list_installed(&self) -> Result<Vec<PackageUpdate>, AppError>;
    fn build_upgrade_command(&self) -> (String, Vec<String>);
    fn build_install_command(&self, pkg: &str) -> (String, Vec<String>);
    fn build_remove_command(&self, pkg: &str, purge: bool) -> (String, Vec<String>);
    async fn remove_dry_run(&self, pkg: &str) -> Result<String, AppError>;
    fn is_available(&self) -> bool;
}

// ============================================================================
// Apt Driver (Debian / Ubuntu)
// ============================================================================

pub struct AptDriver;

#[async_trait]
impl PackageManagerDriver for AptDriver {
    fn name(&self) -> &'static str {
        "apt"
    }

    fn is_available(&self) -> bool {
        StdCommand::new("which")
            .arg("apt-get")
            .output()
            .map(|o| o.status.success())
            .unwrap_or(false)
    }

    async fn check_updates(&self) -> Result<Vec<PackageUpdate>, AppError> {
        let output = TokioCommand::new("sudo")
            .args(["-n", "apt", "list", "--upgradable"])
            .output()
            .await
            .map_err(|e| AppError::ExecutionFailed(e.to_string()))?;

        if !output.status.success() {
            return Err(AppError::ExecutionFailed(
                String::from_utf8_lossy(&output.stderr).to_string(),
            ));
        }

        let stdout = String::from_utf8_lossy(&output.stdout);
        let packages = stdout
            .lines()
            .skip(1)
            .filter(|l| !l.is_empty())
            .filter_map(|line| {
                let parts: Vec<&str> = line.split_whitespace().collect();
                if parts.len() >= 2 {
                    let name = parts[0].split('/').next().unwrap_or(parts[0]).to_string();
                    let version = parts[1].to_string();
                    Some(PackageUpdate {
                        name,
                        version,
                        status: "upgradable".to_string(),
                    })
                } else {
                    None
                }
            })
            .collect();

        Ok(packages)
    }

    async fn list_installed(&self) -> Result<Vec<PackageUpdate>, AppError> {
        let output = TokioCommand::new("dpkg-query")
            .arg("-W")
            .arg("-f=${binary:Package} ${Version}\n")
            .output()
            .await
            .map_err(|e| AppError::ExecutionFailed(e.to_string()))?;

        if !output.status.success() {
            return Err(AppError::ExecutionFailed(
                String::from_utf8_lossy(&output.stderr).to_string(),
            ));
        }

        let stdout = String::from_utf8_lossy(&output.stdout);
        let packages = stdout
            .lines()
            .filter_map(|line| {
                let parts: Vec<&str> = line.split_whitespace().collect();
                if parts.len() >= 2 {
                    Some(PackageUpdate {
                        name: parts[0].to_string(),
                        version: parts[1].to_string(),
                        status: "installed".to_string(),
                    })
                } else {
                    None
                }
            })
            .collect();

        Ok(packages)
    }

    fn build_upgrade_command(&self) -> (String, Vec<String>) {
        (
            "sudo".to_string(),
            vec![
                "-n".to_string(),
                "apt-get".to_string(),
                "upgrade".to_string(),
                "-y".to_string(),
            ],
        )
    }

    fn build_install_command(&self, pkg: &str) -> (String, Vec<String>) {
        (
            "sudo".to_string(),
            vec![
                "-n".to_string(),
                "apt-get".to_string(),
                "install".to_string(),
                "-y".to_string(),
                pkg.to_string(),
            ],
        )
    }

    fn build_remove_command(&self, pkg: &str, purge: bool) -> (String, Vec<String>) {
        let action = if purge { "purge" } else { "remove" };
        (
            "sudo".to_string(),
            vec![
                "-n".to_string(),
                "apt-get".to_string(),
                action.to_string(),
                "-y".to_string(),
                pkg.to_string(),
            ],
        )
    }

    async fn remove_dry_run(&self, pkg: &str) -> Result<String, AppError> {
        if !is_valid_package_name(pkg) {
            return Err(AppError::InvalidInput("Invalid package name".to_string()));
        }
        let output = TokioCommand::new("sudo")
            .args(["-n", "apt-get", "remove", "-s", pkg])
            .output()
            .await
            .map_err(|e| AppError::ExecutionFailed(e.to_string()))?;

        Ok(String::from_utf8_lossy(&output.stdout).to_string())
    }
}

// ============================================================================
// Dnf Driver (Fedora / RHEL / CentOS)
// ============================================================================

pub struct DnfDriver;

#[async_trait]
impl PackageManagerDriver for DnfDriver {
    fn name(&self) -> &'static str {
        "dnf"
    }

    fn is_available(&self) -> bool {
        StdCommand::new("which")
            .arg("dnf")
            .output()
            .map(|o| o.status.success())
            .unwrap_or(false)
    }

    async fn check_updates(&self) -> Result<Vec<PackageUpdate>, AppError> {
        let output = TokioCommand::new("sudo")
            .args(["-n", "dnf", "check-update"])
            .output()
            .await
            .map_err(|e| AppError::ExecutionFailed(e.to_string()))?;

        let code = output.status.code();
        if code == Some(0) {
            return Ok(Vec::new());
        } else if code != Some(100) {
            return Err(AppError::ExecutionFailed(
                String::from_utf8_lossy(&output.stderr).to_string(),
            ));
        }

        let stdout = String::from_utf8_lossy(&output.stdout);
        let packages = stdout
            .lines()
            .filter(|l| {
                !l.is_empty()
                    && !l.starts_with("Last metadata expiration check")
                    && !l.starts_with("Obsoleting Packages")
            })
            .filter_map(|line| {
                let parts: Vec<&str> = line.split_whitespace().collect();
                if parts.len() >= 2 {
                    let name = parts[0].to_string();
                    let version = parts[1].to_string();
                    Some(PackageUpdate {
                        name,
                        version,
                        status: "upgradable".to_string(),
                    })
                } else {
                    None
                }
            })
            .collect();

        Ok(packages)
    }

    async fn list_installed(&self) -> Result<Vec<PackageUpdate>, AppError> {
        let output = TokioCommand::new("sudo")
            .args(["-n", "dnf", "list", "installed", "-q"])
            .output()
            .await
            .map_err(|e| AppError::ExecutionFailed(e.to_string()))?;

        if !output.status.success() {
            return Err(AppError::ExecutionFailed(
                String::from_utf8_lossy(&output.stderr).to_string(),
            ));
        }

        let stdout = String::from_utf8_lossy(&output.stdout);
        let packages = stdout
            .lines()
            .filter(|l| !l.starts_with("Installed Packages"))
            .filter_map(|line| {
                let parts: Vec<&str> = line.split_whitespace().collect();
                if parts.len() >= 2 {
                    let name = parts[0].split('.').next().unwrap_or(parts[0]).to_string();
                    let version = parts[1].to_string();
                    Some(PackageUpdate {
                        name,
                        version,
                        status: "installed".to_string(),
                    })
                } else {
                    None
                }
            })
            .collect();

        Ok(packages)
    }

    fn build_upgrade_command(&self) -> (String, Vec<String>) {
        (
            "sudo".to_string(),
            vec![
                "-n".to_string(),
                "dnf".to_string(),
                "upgrade".to_string(),
                "-y".to_string(),
            ],
        )
    }

    fn build_install_command(&self, pkg: &str) -> (String, Vec<String>) {
        (
            "sudo".to_string(),
            vec![
                "-n".to_string(),
                "dnf".to_string(),
                "install".to_string(),
                "-y".to_string(),
                pkg.to_string(),
            ],
        )
    }

    fn build_remove_command(&self, pkg: &str, _purge: bool) -> (String, Vec<String>) {
        (
            "sudo".to_string(),
            vec![
                "-n".to_string(),
                "dnf".to_string(),
                "remove".to_string(),
                "-y".to_string(),
                pkg.to_string(),
            ],
        )
    }

    async fn remove_dry_run(&self, pkg: &str) -> Result<String, AppError> {
        if !is_valid_package_name(pkg) {
            return Err(AppError::InvalidInput("Invalid package name".to_string()));
        }
        let output = TokioCommand::new("sudo")
            .args(["-n", "dnf", "remove", pkg, "--assumeno"])
            .output()
            .await
            .map_err(|e| AppError::ExecutionFailed(e.to_string()))?;

        Ok(String::from_utf8_lossy(&output.stdout).to_string())
    }
}

// ============================================================================
// Pacman Driver (Arch Linux / Manjaro)
// ============================================================================

pub struct PacmanDriver;

#[async_trait]
impl PackageManagerDriver for PacmanDriver {
    fn name(&self) -> &'static str {
        "pacman"
    }

    fn is_available(&self) -> bool {
        StdCommand::new("which")
            .arg("pacman")
            .output()
            .map(|o| o.status.success())
            .unwrap_or(false)
    }

    async fn check_updates(&self) -> Result<Vec<PackageUpdate>, AppError> {
        let output = TokioCommand::new("sudo")
            .args(["-n", "pacman", "-Qu"])
            .output()
            .await
            .map_err(|e| AppError::ExecutionFailed(e.to_string()))?;

        if !output.status.success() {
            if output.status.code() == Some(1) {
                return Ok(Vec::new());
            }
            return Err(AppError::ExecutionFailed(
                String::from_utf8_lossy(&output.stderr).to_string(),
            ));
        }

        let stdout = String::from_utf8_lossy(&output.stdout);
        let packages = stdout
            .lines()
            .filter(|l| !l.is_empty())
            .filter_map(|line| {
                let parts: Vec<&str> = line.split_whitespace().collect();
                if !parts.is_empty() {
                    let name = parts[0].to_string();
                    let version = if parts.len() >= 4 {
                        parts[3].to_string()
                    } else {
                        "latest".to_string()
                    };
                    Some(PackageUpdate {
                        name,
                        version,
                        status: "upgradable".to_string(),
                    })
                } else {
                    None
                }
            })
            .collect();

        Ok(packages)
    }

    async fn list_installed(&self) -> Result<Vec<PackageUpdate>, AppError> {
        let output = TokioCommand::new("sudo")
            .args(["-n", "pacman", "-Q"])
            .output()
            .await
            .map_err(|e| AppError::ExecutionFailed(e.to_string()))?;

        if !output.status.success() {
            return Err(AppError::ExecutionFailed(
                String::from_utf8_lossy(&output.stderr).to_string(),
            ));
        }

        let stdout = String::from_utf8_lossy(&output.stdout);
        let packages = stdout
            .lines()
            .filter_map(|line| {
                let parts: Vec<&str> = line.split_whitespace().collect();
                if parts.len() >= 2 {
                    Some(PackageUpdate {
                        name: parts[0].to_string(),
                        version: parts[1].to_string(),
                        status: "installed".to_string(),
                    })
                } else {
                    None
                }
            })
            .collect();

        Ok(packages)
    }

    fn build_upgrade_command(&self) -> (String, Vec<String>) {
        (
            "sudo".to_string(),
            vec![
                "-n".to_string(),
                "pacman".to_string(),
                "-Syu".to_string(),
                "--noconfirm".to_string(),
            ],
        )
    }

    fn build_install_command(&self, pkg: &str) -> (String, Vec<String>) {
        (
            "sudo".to_string(),
            vec![
                "-n".to_string(),
                "pacman".to_string(),
                "-S".to_string(),
                "--noconfirm".to_string(),
                pkg.to_string(),
            ],
        )
    }

    fn build_remove_command(&self, pkg: &str, _purge: bool) -> (String, Vec<String>) {
        (
            "sudo".to_string(),
            vec![
                "-n".to_string(),
                "pacman".to_string(),
                "-Rns".to_string(),
                "--noconfirm".to_string(),
                pkg.to_string(),
            ],
        )
    }

    async fn remove_dry_run(&self, pkg: &str) -> Result<String, AppError> {
        if !is_valid_package_name(pkg) {
            return Err(AppError::InvalidInput("Invalid package name".to_string()));
        }
        let output = TokioCommand::new("sudo")
            .args(["-n", "pacman", "-Rns", pkg, "-p"])
            .output()
            .await
            .map_err(|e| AppError::ExecutionFailed(e.to_string()))?;

        Ok(String::from_utf8_lossy(&output.stdout).to_string())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_valid_package_names() {
        assert!(is_valid_package_name("nginx"));
        assert!(is_valid_package_name("python3.11"));
        assert!(is_valid_package_name("libssl-dev"));
        assert!(is_valid_package_name("pkg_config"));

        assert!(!is_valid_package_name(""));
        assert!(!is_valid_package_name("-rf"));
        assert!(!is_valid_package_name("nginx; rm -rf /"));
        assert!(!is_valid_package_name("pkg`whoami`"));
    }

    #[test]
    fn test_driver_command_builders() {
        let apt = AptDriver;
        assert_eq!(apt.name(), "apt");
        let (cmd, args) = apt.build_upgrade_command();
        assert_eq!(cmd, "sudo");
        assert_eq!(args, vec!["-n", "apt-get", "upgrade", "-y"]);

        let (cmd, args) = apt.build_install_command("curl");
        assert_eq!(cmd, "sudo");
        assert_eq!(args, vec!["-n", "apt-get", "install", "-y", "curl"]);

        let (cmd, args) = apt.build_remove_command("curl", true);
        assert_eq!(cmd, "sudo");
        assert_eq!(args, vec!["-n", "apt-get", "purge", "-y", "curl"]);

        let pacman = PacmanDriver;
        assert_eq!(pacman.name(), "pacman");
        let (_, args) = pacman.build_upgrade_command();
        assert_eq!(args, vec!["-n", "pacman", "-Syu", "--noconfirm"]);

        let dnf = DnfDriver;
        assert_eq!(dnf.name(), "dnf");
        let (_, args) = dnf.build_upgrade_command();
        assert_eq!(args, vec!["-n", "dnf", "upgrade", "-y"]);
    }
}
