use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum PluginEntryType {
    DeclarativeSchema,
    Iframe,
    CustomView,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct PluginUiConfig {
    pub tab_id: String,
    pub title: String,
    pub icon: String,
    pub entry_type: PluginEntryType,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct PluginManifest {
    pub id: String,
    pub name: String,
    pub version: String,
    pub author: String,
    pub description: String,
    pub executable: String,
    pub ui: PluginUiConfig,
    #[serde(default)]
    pub capabilities: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum PluginState {
    Installed,
    Running,
    Stopped,
    Crashed,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PluginRuntimeInfo {
    pub manifest: PluginManifest,
    pub state: PluginState,
    pub pid: Option<u32>,
    pub socket_path: Option<String>,
    pub started_at: Option<String>,
    pub error: Option<String>,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_manifest_deserialization() {
        let json_data = r#"{
            "id": "wadm-nginx-manager",
            "name": "Nginx Manager",
            "version": "1.0.0",
            "author": "WADM Community",
            "description": "Visual vhost & reverse proxy editor",
            "executable": "bin/nginx-plugin",
            "ui": {
                "tab_id": "nginx",
                "title": "Nginx",
                "icon": "server",
                "entry_type": "declarative_schema"
            },
            "capabilities": ["manage_services", "read_network"]
        }"#;

        let manifest: PluginManifest = serde_json::from_str(json_data).unwrap();
        assert_eq!(manifest.id, "wadm-nginx-manager");
        assert_eq!(manifest.ui.entry_type, PluginEntryType::DeclarativeSchema);
        assert_eq!(manifest.capabilities.len(), 2);
    }
}
