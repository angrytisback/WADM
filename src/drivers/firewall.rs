use super::error::AppError;
use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use std::process::Command as StdCommand;
use tokio::process::Command as TokioCommand;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct FirewallStatus {
    pub active: bool,
    pub rules: Vec<String>,
    pub installed: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FirewallRuleRequest {
    pub rule: String,
}

pub fn is_safe_ufw_rule(rule: &str) -> bool {
    let r = rule.trim();
    if r.is_empty() || r.len() > 120 {
        return false;
    }
    if !r.chars().all(|c| {
        c.is_ascii_alphanumeric() || c == ' ' || c == '.' || c == '/' || c == ':' || c == '-'
    }) {
        return false;
    }
    let parts: Vec<&str> = r.split_whitespace().collect();
    if parts.is_empty() {
        return false;
    }
    let first = parts[0].to_lowercase();
    let valid_actions = [
        "allow", "deny", "reject", "limit", "delete", "insert", "route",
    ];
    let is_number = first.chars().all(|c| c.is_ascii_digit());
    valid_actions.contains(&first.as_str()) || is_number
}

#[async_trait]
pub trait FirewallDriver: Send + Sync {
    fn backend_name(&self) -> &'static str;
    async fn get_status(&self) -> Result<FirewallStatus, AppError>;
    async fn set_status(&self, enable: bool) -> Result<(), AppError>;
    async fn add_rule(&self, rule: &FirewallRuleRequest) -> Result<(), AppError>;
    async fn delete_rule(&self, rule_id_or_index: &str) -> Result<(), AppError>;
    fn is_installed(&self) -> bool;
}

// ============================================================================
// UFW Driver
// ============================================================================

pub struct UfwDriver;

#[async_trait]
impl FirewallDriver for UfwDriver {
    fn backend_name(&self) -> &'static str {
        "ufw"
    }

    fn is_installed(&self) -> bool {
        StdCommand::new("which")
            .arg("ufw")
            .output()
            .map(|o| o.status.success())
            .unwrap_or(false)
    }

    async fn get_status(&self) -> Result<FirewallStatus, AppError> {
        if !self.is_installed() {
            return Ok(FirewallStatus {
                active: false,
                rules: vec![],
                installed: false,
            });
        }

        let output = TokioCommand::new("sudo")
            .args(["-n", "ufw", "status", "numbered"])
            .output()
            .await
            .map_err(|e| AppError::ExecutionFailed(format!("Failed to execute ufw: {}", e)))?;

        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr).trim().to_string();
            let err_msg = if stderr.is_empty() {
                let stdout = String::from_utf8_lossy(&output.stdout).trim().to_string();
                if stdout.is_empty() {
                    format!(
                        "ufw command failed with exit code {:?}",
                        output.status.code()
                    )
                } else {
                    stdout
                }
            } else {
                stderr
            };
            return Err(AppError::ExecutionFailed(format!(
                "Firewall check error: {}",
                err_msg
            )));
        }

        let stdout = String::from_utf8_lossy(&output.stdout);
        let active = stdout.contains("Status: active");
        let rules: Vec<String> = stdout
            .lines()
            .filter(|line| line.contains('['))
            .map(|line| line.to_string())
            .collect();

        Ok(FirewallStatus {
            active,
            rules,
            installed: true,
        })
    }

    async fn set_status(&self, enable: bool) -> Result<(), AppError> {
        let mut cmd = TokioCommand::new("sudo");
        if enable {
            cmd.args(["-n", "ufw", "--force", "enable"]);
        } else {
            cmd.args(["-n", "ufw", "disable"]);
        }

        let output = cmd
            .output()
            .await
            .map_err(|e| AppError::ExecutionFailed(e.to_string()))?;

        if output.status.success() {
            Ok(())
        } else {
            Err(AppError::ExecutionFailed(
                String::from_utf8_lossy(&output.stderr).to_string(),
            ))
        }
    }

    async fn add_rule(&self, req: &FirewallRuleRequest) -> Result<(), AppError> {
        let rule = req.rule.trim();
        if !is_safe_ufw_rule(rule) {
            return Err(AppError::InvalidInput(
                "Invalid firewall rule format".to_string(),
            ));
        }

        let rule_parts: Vec<&str> = rule.split_whitespace().collect();
        let mut args = vec!["-n", "ufw"];
        args.extend(rule_parts);

        let output = TokioCommand::new("sudo")
            .args(&args)
            .output()
            .await
            .map_err(|e| AppError::ExecutionFailed(e.to_string()))?;

        if output.status.success() {
            Ok(())
        } else {
            Err(AppError::ExecutionFailed(
                String::from_utf8_lossy(&output.stderr).to_string(),
            ))
        }
    }

    async fn delete_rule(&self, rule_id_or_index: &str) -> Result<(), AppError> {
        let rule = rule_id_or_index.trim();
        if !is_safe_ufw_rule(rule) {
            return Err(AppError::InvalidInput(
                "Invalid firewall rule format".to_string(),
            ));
        }

        let mut args = vec!["-n", "ufw", "--force", "delete"];
        let rule_parts: Vec<&str> = rule.split_whitespace().collect();
        let effective_parts = if rule_parts.first().map(|s| *s == "delete").unwrap_or(false) {
            &rule_parts[1..]
        } else {
            &rule_parts[..]
        };
        args.extend_from_slice(effective_parts);

        let output = TokioCommand::new("sudo")
            .args(&args)
            .output()
            .await
            .map_err(|e| AppError::ExecutionFailed(e.to_string()))?;

        if output.status.success() {
            Ok(())
        } else {
            Err(AppError::ExecutionFailed(
                String::from_utf8_lossy(&output.stderr).to_string(),
            ))
        }
    }
}

// ============================================================================
// Nftables Driver (Stub / Future Plugin Support)
// ============================================================================

#[allow(dead_code)]
pub struct NftablesDriver;

#[async_trait]
impl FirewallDriver for NftablesDriver {
    fn backend_name(&self) -> &'static str {
        "nftables"
    }

    fn is_installed(&self) -> bool {
        StdCommand::new("which")
            .arg("nft")
            .output()
            .map(|o| o.status.success())
            .unwrap_or(false)
    }

    async fn get_status(&self) -> Result<FirewallStatus, AppError> {
        let installed = self.is_installed();
        Ok(FirewallStatus {
            active: installed,
            rules: vec![],
            installed,
        })
    }

    async fn set_status(&self, _enable: bool) -> Result<(), AppError> {
        Err(AppError::Unsupported(
            "Nftables driver is experimental/stub in this version".to_string(),
        ))
    }

    async fn add_rule(&self, _rule: &FirewallRuleRequest) -> Result<(), AppError> {
        Err(AppError::Unsupported(
            "Nftables driver is experimental/stub in this version".to_string(),
        ))
    }

    async fn delete_rule(&self, _rule_id_or_index: &str) -> Result<(), AppError> {
        Err(AppError::Unsupported(
            "Nftables driver is experimental/stub in this version".to_string(),
        ))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_valid_ufw_rules() {
        assert!(is_safe_ufw_rule("allow 22/tcp"));
        assert!(is_safe_ufw_rule("deny 80"));
        assert!(is_safe_ufw_rule("allow from 192.168.1.0/24"));
        assert!(is_safe_ufw_rule("delete 1"));
        assert!(is_safe_ufw_rule("1"));
    }

    #[test]
    fn test_invalid_ufw_rules_injection() {
        assert!(!is_safe_ufw_rule(""));
        assert!(!is_safe_ufw_rule("allow 22; rm -rf /"));
        assert!(!is_safe_ufw_rule("allow `whoami`"));
        assert!(!is_safe_ufw_rule("allow $(cat /etc/passwd)"));
        assert!(!is_safe_ufw_rule("allow 22 && reboot"));
        assert!(!is_safe_ufw_rule("malicious_command 22"));
    }

    #[test]
    fn test_driver_backend_names() {
        let ufw = UfwDriver;
        assert_eq!(ufw.backend_name(), "ufw");

        let nft = NftablesDriver;
        assert_eq!(nft.backend_name(), "nftables");
    }
}
