/*!
 * app_server.rs — Unified JSON-RPC 2.0 server dispatcher.
 *
 * Powers CLI, VS Code extension, and VibeUI over the same wire protocol.
 */

use serde::{Deserialize, Serialize};
use std::collections::HashMap;

// ---------------------------------------------------------------------------
// Error codes
// ---------------------------------------------------------------------------

pub struct RpcErrorCode;

impl RpcErrorCode {
    pub const PARSE_ERROR: i32 = -32700;
    pub const INVALID_REQUEST: i32 = -32600;
    pub const METHOD_NOT_FOUND: i32 = -32601;
    pub const INVALID_PARAMS: i32 = -32602;
    pub const INTERNAL_ERROR: i32 = -32603;
}

// ---------------------------------------------------------------------------
// RpcId
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(untagged)]
pub enum RpcId {
    Number(i64),
    Str(String),
    Null,
}

// ---------------------------------------------------------------------------
// RpcRequest / RpcResponse / RpcError
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RpcRequest {
    pub jsonrpc: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub id: Option<RpcId>,
    pub method: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub params: Option<serde_json::Value>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RpcResponse {
    pub jsonrpc: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub id: Option<RpcId>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub result: Option<serde_json::Value>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub error: Option<RpcError>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RpcError {
    pub code: i32,
    pub message: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub data: Option<serde_json::Value>,
}

// ---------------------------------------------------------------------------
// RpcResponse helpers
// ---------------------------------------------------------------------------

impl RpcResponse {
    pub fn ok(id: Option<RpcId>, result: serde_json::Value) -> Self {
        Self {
            jsonrpc: "2.0".into(),
            id,
            result: Some(result),
            error: None,
        }
    }

    pub fn error(id: Option<RpcId>, error: RpcError) -> Self {
        Self {
            jsonrpc: "2.0".into(),
            id,
            result: None,
            error: Some(error),
        }
    }
}

// ---------------------------------------------------------------------------
// RpcError helpers
// ---------------------------------------------------------------------------

impl RpcError {
    pub fn method_not_found(method: &str) -> Self {
        Self {
            code: RpcErrorCode::METHOD_NOT_FOUND,
            message: format!("Method not found: {method}"),
            data: None,
        }
    }

    pub fn invalid_params(msg: &str) -> Self {
        Self {
            code: RpcErrorCode::INVALID_PARAMS,
            message: msg.to_string(),
            data: None,
        }
    }

    pub fn internal(msg: &str) -> Self {
        Self {
            code: RpcErrorCode::INTERNAL_ERROR,
            message: msg.to_string(),
            data: None,
        }
    }

    pub fn parse_error(msg: &str) -> Self {
        Self {
            code: RpcErrorCode::PARSE_ERROR,
            message: msg.to_string(),
            data: None,
        }
    }
}

// ---------------------------------------------------------------------------
// Handler type alias
// ---------------------------------------------------------------------------

pub type HandlerFn = Box<dyn Fn(Option<serde_json::Value>) -> serde_json::Value + Send + Sync>;

// ---------------------------------------------------------------------------
// AppServer
// ---------------------------------------------------------------------------

pub struct AppServer {
    pub handlers: HashMap<String, HandlerFn>,
}

impl AppServer {
    pub fn new() -> Self {
        Self {
            handlers: HashMap::new(),
        }
    }

    pub fn register(&mut self, method: &str, handler: HandlerFn) {
        self.handlers.insert(method.to_string(), handler);
    }

    pub fn dispatch(&self, request: &RpcRequest) -> RpcResponse {
        match self.handlers.get(&request.method) {
            Some(handler) => {
                let result = handler(request.params.clone());
                RpcResponse::ok(request.id.clone(), result)
            }
            None => RpcResponse::error(
                request.id.clone(),
                RpcError::method_not_found(&request.method),
            ),
        }
    }

    pub fn parse_request(json: &str) -> Result<RpcRequest, RpcError> {
        serde_json::from_str::<RpcRequest>(json).map_err(|e| RpcError::parse_error(&e.to_string()))
    }

    pub fn handle_raw(&self, json: &str) -> String {
        let response = match Self::parse_request(json) {
            Ok(req) => self.dispatch(&req),
            Err(err) => RpcResponse::error(None, err),
        };
        serde_json::to_string(&response).unwrap_or_else(|e| {
            format!(
                r#"{{"jsonrpc":"2.0","error":{{"code":{},"message":"{}"}}}}"#,
                RpcErrorCode::INTERNAL_ERROR,
                e
            )
        })
    }
}

impl Default for AppServer {
    fn default() -> Self {
        Self::new()
    }
}

// ---------------------------------------------------------------------------
// Unit tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    fn make_server() -> AppServer {
        let mut server = AppServer::new();
        server.register(
            "echo",
            Box::new(|params| params.unwrap_or(json!("no params"))),
        );
        server
    }

    #[test]
    fn test_register_and_dispatch_known_method() {
        let server = make_server();
        let req = RpcRequest {
            jsonrpc: "2.0".into(),
            id: Some(RpcId::Number(1)),
            method: "echo".into(),
            params: Some(json!("hello")),
        };
        let resp = server.dispatch(&req);
        assert!(resp.error.is_none());
        assert_eq!(resp.result, Some(json!("hello")));
    }

    #[test]
    fn test_dispatch_unknown_method_returns_error() {
        let server = make_server();
        let req = RpcRequest {
            jsonrpc: "2.0".into(),
            id: Some(RpcId::Number(2)),
            method: "nope".into(),
            params: None,
        };
        let resp = server.dispatch(&req);
        assert!(resp.result.is_none());
        let err = resp.error.unwrap();
        assert_eq!(err.code, RpcErrorCode::METHOD_NOT_FOUND);
    }

    #[test]
    fn test_parse_valid_request() {
        let json = r#"{"jsonrpc":"2.0","id":1,"method":"ping","params":null}"#;
        let req = AppServer::parse_request(json).unwrap();
        assert_eq!(req.method, "ping");
        assert_eq!(req.id, Some(RpcId::Number(1)));
    }

    #[test]
    fn test_parse_invalid_json_returns_parse_error() {
        let json = "not json at all{{{";
        let err = AppServer::parse_request(json).unwrap_err();
        assert_eq!(err.code, RpcErrorCode::PARSE_ERROR);
    }

    #[test]
    fn test_response_ok_has_result_no_error() {
        let resp = RpcResponse::ok(Some(RpcId::Number(1)), json!(42));
        assert!(resp.result.is_some());
        assert!(resp.error.is_none());
        assert_eq!(resp.jsonrpc, "2.0");
    }

    #[test]
    fn test_response_error_has_error_no_result() {
        let resp = RpcResponse::error(None, RpcError::internal("oops"));
        assert!(resp.result.is_none());
        assert!(resp.error.is_some());
    }

    #[test]
    fn test_handle_raw_returns_json_string() {
        let server = make_server();
        let raw = r#"{"jsonrpc":"2.0","id":1,"method":"echo","params":"world"}"#;
        let out = server.handle_raw(raw);
        let v: serde_json::Value = serde_json::from_str(&out).unwrap();
        assert_eq!(v["result"], json!("world"));
    }

    #[test]
    fn test_notification_has_no_id() {
        // A notification has no id; dispatch should return a response with no id.
        let server = make_server();
        let req = RpcRequest {
            jsonrpc: "2.0".into(),
            id: None,
            method: "echo".into(),
            params: Some(json!("notify")),
        };
        let resp = server.dispatch(&req);
        assert!(resp.id.is_none());
    }
}
