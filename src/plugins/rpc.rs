use crate::drivers::error::AppError;
use serde::{Deserialize, Serialize};
use std::path::Path;
use std::time::Duration;
use tokio::io::{AsyncBufReadExt, AsyncWriteExt, BufReader};
use tokio::net::UnixStream;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct RpcRequest {
    pub jsonrpc: String,
    pub id: Option<serde_json::Value>,
    pub method: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub params: Option<serde_json::Value>,
}

impl RpcRequest {
    pub fn new(
        id: Option<serde_json::Value>,
        method: impl Into<String>,
        params: Option<serde_json::Value>,
    ) -> Self {
        Self {
            jsonrpc: "2.0".to_string(),
            id,
            method: method.into(),
            params,
        }
    }
}

#[allow(dead_code)]
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct RpcError {
    pub code: i64,
    pub message: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub data: Option<serde_json::Value>,
}

#[allow(dead_code)]
impl RpcError {
    pub fn new(code: i64, message: impl Into<String>, data: Option<serde_json::Value>) -> Self {
        Self {
            code,
            message: message.into(),
            data,
        }
    }
}

#[allow(dead_code)]
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct RpcResponse {
    pub jsonrpc: String,
    pub id: Option<serde_json::Value>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub result: Option<serde_json::Value>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub error: Option<RpcError>,
}

#[allow(dead_code)]
impl RpcResponse {
    pub fn success(id: Option<serde_json::Value>, result: serde_json::Value) -> Self {
        Self {
            jsonrpc: "2.0".to_string(),
            id,
            result: Some(result),
            error: None,
        }
    }

    pub fn error(
        id: Option<serde_json::Value>,
        code: i64,
        message: impl Into<String>,
        data: Option<serde_json::Value>,
    ) -> Self {
        Self {
            jsonrpc: "2.0".to_string(),
            id,
            result: None,
            error: Some(RpcError::new(code, message, data)),
        }
    }
}

pub async fn call_uds(
    socket_path: &Path,
    request: &serde_json::Value,
    timeout: Duration,
) -> Result<serde_json::Value, AppError> {
    let operation = async {
        let stream = UnixStream::connect(socket_path).await.map_err(|e| {
            AppError::ExecutionFailed(format!(
                "Failed to connect to socket at '{}': {}",
                socket_path.display(),
                e
            ))
        })?;

        let (reader, mut writer) = stream.into_split();
        let mut buf_reader = BufReader::new(reader);

        let mut payload = serde_json::to_string(request).map_err(|e| {
            AppError::InvalidInput(format!("Failed to serialize JSON-RPC request: {}", e))
        })?;
        payload.push('\n');

        writer.write_all(payload.as_bytes()).await.map_err(|e| {
            AppError::ExecutionFailed(format!("Failed to write to plugin socket: {}", e))
        })?;
        writer.flush().await.map_err(|e| {
            AppError::ExecutionFailed(format!("Failed to flush plugin socket: {}", e))
        })?;

        let mut line = String::new();
        let bytes_read = buf_reader.read_line(&mut line).await.map_err(|e| {
            AppError::ExecutionFailed(format!("Failed to read response from plugin socket: {}", e))
        })?;

        if bytes_read == 0 {
            return Err(AppError::ExecutionFailed(
                "Plugin closed connection without sending a response".to_string(),
            ));
        }

        let resp: serde_json::Value = serde_json::from_str(line.trim()).map_err(|e| {
            AppError::ExecutionFailed(format!(
                "Invalid JSON-RPC response from plugin ('{}'): {}",
                line.trim(),
                e
            ))
        })?;

        Ok(resp)
    };

    tokio::time::timeout(timeout, operation)
        .await
        .map_err(|_| {
            AppError::ExecutionFailed(format!("Plugin RPC call timed out after {:?}", timeout))
        })?
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;
    use tokio::net::UnixListener;

    #[test]
    fn test_rpc_models_serialization() {
        let req = RpcRequest::new(Some(json!(1)), "test.method", Some(json!({"param": "val"})));
        let serialized = serde_json::to_string(&req).unwrap();
        assert!(serialized.contains("\"jsonrpc\":\"2.0\""));
        assert!(serialized.contains("\"method\":\"test.method\""));

        let deserialized: RpcRequest = serde_json::from_str(&serialized).unwrap();
        assert_eq!(req, deserialized);

        let resp = RpcResponse::success(Some(json!(1)), json!({"status": "ok"}));
        let serialized_resp = serde_json::to_string(&resp).unwrap();
        assert!(serialized_resp.contains("\"result\":{\"status\":\"ok\"}"));

        let err_resp = RpcResponse::error(Some(json!(2)), -32600, "Invalid Request", None);
        let serialized_err = serde_json::to_string(&err_resp).unwrap();
        assert!(serialized_err.contains("\"code\":-32600"));
    }

    #[tokio::test]
    async fn test_call_uds_roundtrip() {
        let temp_dir = std::env::temp_dir();
        let sock_path = temp_dir.join(format!("test_uds_{}.sock", uuid::Uuid::new_v4()));
        let _ = std::fs::remove_file(&sock_path);

        let listener = UnixListener::bind(&sock_path).unwrap();

        // Spawn mock server
        tokio::spawn(async move {
            if let Ok((stream, _)) = listener.accept().await {
                let (reader, mut writer) = stream.into_split();
                let mut buf_reader = BufReader::new(reader);
                let mut line = String::new();
                if buf_reader.read_line(&mut line).await.is_ok() {
                    let req: RpcRequest = serde_json::from_str(line.trim()).unwrap();
                    let resp = RpcResponse::success(req.id, json!({"echo": req.method}));
                    let mut resp_str = serde_json::to_string(&resp).unwrap();
                    resp_str.push('\n');
                    let _ = writer.write_all(resp_str.as_bytes()).await;
                    let _ = writer.flush().await;
                }
            }
        });

        let req = json!({
            "jsonrpc": "2.0",
            "id": 100,
            "method": "ping",
            "params": {}
        });

        let res = call_uds(&sock_path, &req, Duration::from_secs(2)).await;
        assert!(res.is_ok());
        let val = res.unwrap();
        assert_eq!(val["result"]["echo"], "ping");

        let _ = std::fs::remove_file(&sock_path);
    }
}
