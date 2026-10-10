use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct NodeSpecs {
    pub cpu_cores: usize,
    pub total_memory: u64,
    pub os_name: String,
    pub os_version: String,
    pub kernel_version: String,
    pub architecture: String,
}

impl Default for NodeSpecs {
    fn default() -> Self {
        Self {
            cpu_cores: 1,
            total_memory: 0,
            os_name: "Linux".to_string(),
            os_version: "Unknown".to_string(),
            kernel_version: "Unknown".to_string(),
            architecture: std::env::consts::ARCH.to_string(),
        }
    }
}

impl NodeSpecs {
    pub fn to_json_string(&self) -> String {
        serde_json::to_string(self).unwrap_or_else(|_| "{}".to_string())
    }

    #[allow(dead_code)]
    pub fn from_json_str(s: &str) -> Self {
        serde_json::from_str(s).unwrap_or_default()
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(tag = "type", content = "payload")]
pub enum ClusterMessage {
    Handshake {
        #[serde(default)]
        node_id: Option<String>,
        name: String,
        hostname: String,
        version: String,
        specs: NodeSpecs,
        token: String,
    },
    HandshakeAck {
        node_id: String,
        auth_token: String,
        message: String,
    },
    Ping {
        #[serde(default)]
        specs: Option<NodeSpecs>,
        #[serde(default)]
        version: Option<String>,
    },
    Pong {
        timestamp: i64,
    },
    RpcRequest {
        id: String,
        method: String,
        params: serde_json::Value,
    },
    RpcResponse {
        id: String,
        result: Option<serde_json::Value>,
        error: Option<String>,
    },
    Disconnect {
        reason: String,
    },
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_cluster_message_serialization_roundtrip() {
        let specs = NodeSpecs {
            cpu_cores: 8,
            total_memory: 16 * 1024 * 1024 * 1024,
            os_name: "Ubuntu".to_string(),
            os_version: "24.04".to_string(),
            kernel_version: "6.8.0-generic".to_string(),
            architecture: "x86_64".to_string(),
        };

        let handshake = ClusterMessage::Handshake {
            node_id: None,
            name: "worker-fra-01".to_string(),
            hostname: "fra-srv-01".to_string(),
            version: "0.96.0".to_string(),
            specs: specs.clone(),
            token: "join-secret-xyz".to_string(),
        };

        let json = serde_json::to_string(&handshake).expect("Serialization failed");
        let deserialized: ClusterMessage =
            serde_json::from_str(&json).expect("Deserialization failed");

        assert_eq!(handshake, deserialized);
    }

    #[test]
    fn test_rpc_request_response_roundtrip() {
        let req = ClusterMessage::RpcRequest {
            id: "req-12345".to_string(),
            method: "get_stats".to_string(),
            params: serde_json::json!({ "node_id": "node-1" }),
        };

        let json = serde_json::to_string(&req).expect("Serialization failed");
        let deser_req: ClusterMessage =
            serde_json::from_str(&json).expect("Deserialization failed");
        assert_eq!(req, deser_req);

        let resp = ClusterMessage::RpcResponse {
            id: "req-12345".to_string(),
            result: Some(serde_json::json!({ "cpu_usage": 12.5 })),
            error: None,
        };

        let resp_json = serde_json::to_string(&resp).expect("Serialization failed");
        let deser_resp: ClusterMessage =
            serde_json::from_str(&resp_json).expect("Deserialization failed");
        assert_eq!(resp, deser_resp);
    }

    #[test]
    fn test_ping_pong_roundtrip() {
        let ping = ClusterMessage::Ping {
            specs: None,
            version: Some("0.96.0".to_string()),
        };
        let json = serde_json::to_string(&ping).unwrap();
        let deser: ClusterMessage = serde_json::from_str(&json).unwrap();
        assert_eq!(ping, deser);

        let pong = ClusterMessage::Pong {
            timestamp: 1712345678,
        };
        let json_pong = serde_json::to_string(&pong).unwrap();
        let deser_pong: ClusterMessage = serde_json::from_str(&json_pong).unwrap();
        assert_eq!(pong, deser_pong);
    }
}
