//! Agent Client Protocol (ACP) support for VibeCody.
//!
//! Implements the emerging standard for agent-editor communication,
//! providing capability negotiation, tool registration, and message
//! exchange between ACP-compatible agents and editors.

use std::collections::HashSet;
use std::fmt;

/// Protocol version following semver.
#[derive(Debug, Clone, PartialEq)]
pub struct AcpVersion {
    pub major: u32,
    pub minor: u32,
    pub patch: u32,
}

impl AcpVersion {
    pub fn new(major: u32, minor: u32, patch: u32) -> Self {
        Self { major, minor, patch }
    }

    pub fn is_compatible_with(&self, other: &AcpVersion) -> bool {
        self.major == other.major
    }
}

impl fmt::Display for AcpVersion {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}.{}.{}", self.major, self.minor, self.patch)
    }
}

/// Capabilities an ACP endpoint can advertise.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum AcpCapability {
    ToolExecution,
    FileEdit,
    CodeCompletion,
    Diagnostics,
    Search,
    Chat,
    AgentSpawn,
    ContextSharing,
}

impl AcpCapability {
    fn as_str(&self) -> &'static str {
        match self {
            Self::ToolExecution => "tool_execution",
            Self::FileEdit => "file_edit",
            Self::CodeCompletion => "code_completion",
            Self::Diagnostics => "diagnostics",
            Self::Search => "search",
            Self::Chat => "chat",
            Self::AgentSpawn => "agent_spawn",
            Self::ContextSharing => "context_sharing",
        }
    }

    fn from_str(s: &str) -> Option<Self> {
        match s {
            "tool_execution" => Some(Self::ToolExecution),
            "file_edit" => Some(Self::FileEdit),
            "code_completion" => Some(Self::CodeCompletion),
            "diagnostics" => Some(Self::Diagnostics),
            "search" => Some(Self::Search),
            "chat" => Some(Self::Chat),
            "agent_spawn" => Some(Self::AgentSpawn),
            "context_sharing" => Some(Self::ContextSharing),
            _ => None,
        }
    }
}

/// Message types in the ACP protocol.
#[derive(Debug, Clone, PartialEq)]
pub enum AcpMessageType {
    CapabilityRequest,
    CapabilityResponse,
    ToolCall,
    ToolResult,
    Notification,
    Error,
    Heartbeat,
    Disconnect,
}

impl AcpMessageType {
    fn as_str(&self) -> &'static str {
        match self {
            Self::CapabilityRequest => "capability_request",
            Self::CapabilityResponse => "capability_response",
            Self::ToolCall => "tool_call",
            Self::ToolResult => "tool_result",
            Self::Notification => "notification",
            Self::Error => "error",
            Self::Heartbeat => "heartbeat",
            Self::Disconnect => "disconnect",
        }
    }

    fn from_str(s: &str) -> Option<Self> {
        match s {
            "capability_request" => Some(Self::CapabilityRequest),
            "capability_response" => Some(Self::CapabilityResponse),
            "tool_call" => Some(Self::ToolCall),
            "tool_result" => Some(Self::ToolResult),
            "notification" => Some(Self::Notification),
            "error" => Some(Self::Error),
            "heartbeat" => Some(Self::Heartbeat),
            "disconnect" => Some(Self::Disconnect),
            _ => None,
        }
    }
}

/// A message exchanged over the ACP protocol.
#[derive(Debug, Clone, PartialEq)]
pub struct AcpMessage {
    pub id: String,
    pub message_type: AcpMessageType,
    pub payload: String,
    pub timestamp: u64,
}

impl AcpMessage {
    pub fn new(id: &str, message_type: AcpMessageType, payload: &str, timestamp: u64) -> Self {
        Self {
            id: id.to_string(),
            message_type,
            payload: payload.to_string(),
            timestamp,
        }
    }

    pub fn to_json(&self) -> String {
        let escaped_payload = self.payload.replace('\\', "\\\\").replace('"', "\\\"");
        format!(
            "{{\"id\":\"{}\",\"type\":\"{}\",\"payload\":\"{}\",\"timestamp\":{}}}",
            self.id,
            self.message_type.as_str(),
            escaped_payload,
            self.timestamp
        )
    }

    pub fn from_json(json: &str) -> Result<Self, String> {
        let id = extract_json_string(json, "id")?;
        let msg_type_str = extract_json_string(json, "type")?;
        let payload = extract_json_string(json, "payload")?;
        let timestamp = extract_json_u64(json, "timestamp")?;

        let message_type = AcpMessageType::from_str(&msg_type_str)
            .ok_or_else(|| format!("Unknown message type: {}", msg_type_str))?;

        Ok(Self {
            id,
            message_type,
            payload,
            timestamp,
        })
    }
}

/// A tool parameter definition.
#[derive(Debug, Clone, PartialEq)]
pub struct AcpParam {
    pub name: String,
    pub param_type: String,
    pub description: String,
    pub required: bool,
}

/// A tool definition registered with an ACP server.
#[derive(Debug, Clone, PartialEq)]
pub struct AcpToolDef {
    pub name: String,
    pub description: String,
    pub parameters: Vec<AcpParam>,
    pub returns: String,
}

impl AcpToolDef {
    pub fn new(name: &str, description: &str, returns: &str) -> Self {
        Self {
            name: name.to_string(),
            description: description.to_string(),
            parameters: Vec::new(),
            returns: returns.to_string(),
        }
    }

    pub fn add_param(&mut self, name: &str, param_type: &str, description: &str, required: bool) {
        self.parameters.push(AcpParam {
            name: name.to_string(),
            param_type: param_type.to_string(),
            description: description.to_string(),
            required,
        });
    }
}

/// Result of capability negotiation between client and server.
#[derive(Debug, Clone, PartialEq)]
pub struct AcpNegotiationResult {
    pub agreed_version: AcpVersion,
    pub shared_capabilities: Vec<AcpCapability>,
    pub status: NegotiationStatus,
}

/// Status of a negotiation attempt.
#[derive(Debug, Clone, PartialEq)]
pub enum NegotiationStatus {
    Success,
    PartialMatch,
    VersionMismatch,
    Failed,
}

/// An ACP error returned from operations.
#[derive(Debug, Clone, PartialEq)]
pub struct AcpError {
    pub code: u32,
    pub message: String,
    pub data: Option<String>,
}

impl fmt::Display for AcpError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "AcpError({}): {}", self.code, self.message)
    }
}

/// An ACP server that advertises capabilities and handles messages.
#[derive(Debug, Clone)]
pub struct AcpServer {
    pub version: AcpVersion,
    pub capabilities: Vec<AcpCapability>,
    pub tools: Vec<AcpToolDef>,
    pub server_name: String,
    pub protocol_id: String,
}

impl AcpServer {
    pub fn new(name: &str, version: AcpVersion) -> Self {
        Self {
            server_name: name.to_string(),
            protocol_id: format!("acp-{}", version),
            version,
            capabilities: Vec::new(),
            tools: Vec::new(),
        }
    }

    pub fn register_capability(&mut self, cap: AcpCapability) {
        if !self.capabilities.contains(&cap) {
            self.capabilities.push(cap);
        }
    }

    pub fn register_tool(&mut self, tool: AcpToolDef) {
        self.tools.push(tool);
    }

    pub fn handle_message(&self, msg: &AcpMessage) -> AcpMessage {
        match msg.message_type {
            AcpMessageType::CapabilityRequest => {
                let caps: Vec<&str> = self.capabilities.iter().map(|c| c.as_str()).collect();
                let payload = format!("{{\"capabilities\":[{}]}}",
                    caps.iter().map(|c| format!("\"{}\"", c)).collect::<Vec<_>>().join(","));
                AcpMessage::new(
                    &msg.id,
                    AcpMessageType::CapabilityResponse,
                    &payload,
                    msg.timestamp + 1,
                )
            }
            AcpMessageType::ToolCall => {
                let tool_name = extract_json_string(&msg.payload, "tool").unwrap_or_default();
                if self.tools.iter().any(|t| t.name == tool_name) {
                    AcpMessage::new(
                        &msg.id,
                        AcpMessageType::ToolResult,
                        &format!("{{\"tool\":\"{}\",\"status\":\"ok\"}}", tool_name),
                        msg.timestamp + 1,
                    )
                } else {
                    AcpMessage::new(
                        &msg.id,
                        AcpMessageType::Error,
                        &format!("{{\"code\":404,\"message\":\"Tool not found: {}\"}}", tool_name),
                        msg.timestamp + 1,
                    )
                }
            }
            AcpMessageType::Heartbeat => {
                AcpMessage::new(&msg.id, AcpMessageType::Heartbeat, "pong", msg.timestamp + 1)
            }
            AcpMessageType::Disconnect => {
                AcpMessage::new(&msg.id, AcpMessageType::Disconnect, "acknowledged", msg.timestamp + 1)
            }
            _ => {
                AcpMessage::new(
                    &msg.id,
                    AcpMessageType::Error,
                    "{\"code\":400,\"message\":\"Unhandled message type\"}",
                    msg.timestamp + 1,
                )
            }
        }
    }

    pub fn negotiate(
        &self,
        client_caps: &[AcpCapability],
        client_version: &AcpVersion,
    ) -> AcpNegotiationResult {
        if !self.version.is_compatible_with(client_version) {
            return AcpNegotiationResult {
                agreed_version: self.version.clone(),
                shared_capabilities: Vec::new(),
                status: NegotiationStatus::VersionMismatch,
            };
        }

        let server_caps: HashSet<&AcpCapability> = self.capabilities.iter().collect();
        let shared: Vec<AcpCapability> = client_caps
            .iter()
            .filter(|c| server_caps.contains(c))
            .cloned()
            .collect();

        if shared.is_empty() {
            AcpNegotiationResult {
                agreed_version: self.version.clone(),
                shared_capabilities: shared,
                status: NegotiationStatus::Failed,
            }
        } else if shared.len() < client_caps.len() {
            AcpNegotiationResult {
                agreed_version: AcpVersion::new(
                    self.version.major,
                    std::cmp::min(self.version.minor, client_version.minor),
                    0,
                ),
                shared_capabilities: shared,
                status: NegotiationStatus::PartialMatch,
            }
        } else {
            AcpNegotiationResult {
                agreed_version: AcpVersion::new(
                    self.version.major,
                    std::cmp::min(self.version.minor, client_version.minor),
                    0,
                ),
                shared_capabilities: shared,
                status: NegotiationStatus::Success,
            }
        }
    }

    pub fn to_manifest_json(&self) -> String {
        let caps: Vec<String> = self.capabilities.iter().map(|c| format!("\"{}\"", c.as_str())).collect();
        let tools: Vec<String> = self.tools.iter().map(|t| {
            let params: Vec<String> = t.parameters.iter().map(|p| {
                format!(
                    "{{\"name\":\"{}\",\"type\":\"{}\",\"description\":\"{}\",\"required\":{}}}",
                    p.name, p.param_type, p.description, p.required
                )
            }).collect();
            format!(
                "{{\"name\":\"{}\",\"description\":\"{}\",\"parameters\":[{}],\"returns\":\"{}\"}}",
                t.name, t.description, params.join(","), t.returns
            )
        }).collect();

        format!(
            "{{\"server_name\":\"{}\",\"protocol_id\":\"{}\",\"version\":\"{}\",\"capabilities\":[{}],\"tools\":[{}]}}",
            self.server_name,
            self.protocol_id,
            self.version,
            caps.join(","),
            tools.join(","),
        )
    }

    pub fn from_manifest_json(json: &str) -> Result<Self, String> {
        let server_name = extract_json_string(json, "server_name")?;
        let protocol_id = extract_json_string(json, "protocol_id")?;
        let version_str = extract_json_string(json, "version")?;

        let version_parts: Vec<u32> = version_str
            .split('.')
            .map(|s| s.parse::<u32>().unwrap_or(0))
            .collect();
        if version_parts.len() != 3 {
            return Err("Invalid version format".to_string());
        }
        let version = AcpVersion::new(version_parts[0], version_parts[1], version_parts[2]);

        let caps_section = extract_json_array(json, "capabilities")?;
        let capabilities: Vec<AcpCapability> = caps_section
            .iter()
            .filter_map(|s| AcpCapability::from_str(s))
            .collect();

        let mut server = Self {
            server_name,
            protocol_id,
            version,
            capabilities,
            tools: Vec::new(),
        };

        // Parse tools array
        if let Some(tools_start) = json.find("\"tools\":[") {
            let tools_offset = tools_start + 9;
            let rest = &json[tools_offset..];
            if let Some(tools_end) = find_matching_bracket(rest) {
                let tools_content = &rest[..tools_end];
                for tool_json in split_json_objects(tools_content) {
                    if let Ok(name) = extract_json_string(&tool_json, "name") {
                        let desc = extract_json_string(&tool_json, "description").unwrap_or_default();
                        let returns = extract_json_string(&tool_json, "returns").unwrap_or_default();
                        let mut tool = AcpToolDef::new(&name, &desc, &returns);

                        if let Some(params_start) = tool_json.find("\"parameters\":[") {
                            let params_offset = params_start + 14;
                            let params_rest = &tool_json[params_offset..];
                            if let Some(params_end) = find_matching_bracket(params_rest) {
                                let params_content = &params_rest[..params_end];
                                for param_json in split_json_objects(params_content) {
                                    let pname = extract_json_string(&param_json, "name").unwrap_or_default();
                                    let ptype = extract_json_string(&param_json, "type").unwrap_or_default();
                                    let pdesc = extract_json_string(&param_json, "description").unwrap_or_default();
                                    let preq = param_json.contains("\"required\":true");
                                    tool.add_param(&pname, &ptype, &pdesc, preq);
                                }
                            }
                        }

                        server.tools.push(tool);
                    }
                }
            }
        }

        Ok(server)
    }
}

/// An ACP client that connects to servers.
#[derive(Debug, Clone)]
pub struct AcpClient {
    pub server_url: String,
    pub capabilities: Vec<AcpCapability>,
    pub connected: bool,
    pub protocol_version: AcpVersion,
}

impl AcpClient {
    pub fn new(url: &str, version: AcpVersion) -> Self {
        Self {
            server_url: url.to_string(),
            capabilities: Vec::new(),
            connected: false,
            protocol_version: version,
        }
    }

    pub fn connect(&mut self, server_manifest: &str) -> Result<AcpNegotiationResult, String> {
        let server = AcpServer::from_manifest_json(server_manifest)?;
        let result = server.negotiate(&self.capabilities, &self.protocol_version);

        match result.status {
            NegotiationStatus::Success | NegotiationStatus::PartialMatch => {
                self.connected = true;
                Ok(result)
            }
            NegotiationStatus::VersionMismatch => {
                Err(format!(
                    "Version mismatch: client={}, server={}",
                    self.protocol_version, server.version
                ))
            }
            NegotiationStatus::Failed => {
                Err("Negotiation failed: no shared capabilities".to_string())
            }
        }
    }

    pub fn call_tool(&self, name: &str, args_json: &str) -> Result<AcpMessage, AcpError> {
        if !self.connected {
            return Err(AcpError {
                code: 1,
                message: "Not connected to server".to_string(),
                data: None,
            });
        }

        if name.is_empty() {
            return Err(AcpError {
                code: 2,
                message: "Tool name cannot be empty".to_string(),
                data: None,
            });
        }

        let payload = format!("{{\"tool\":\"{}\",\"args\":{}}}", name, args_json);
        Ok(AcpMessage::new(
            &format!("call-{}", name),
            AcpMessageType::ToolCall,
            &payload,
            current_timestamp(),
        ))
    }

    pub fn disconnect(&mut self) -> AcpMessage {
        self.connected = false;
        AcpMessage::new(
            "disconnect",
            AcpMessageType::Disconnect,
            "client_disconnect",
            current_timestamp(),
        )
    }

    pub fn is_connected(&self) -> bool {
        self.connected
    }

    pub fn supports_capability(&self, cap: &AcpCapability) -> bool {
        self.capabilities.contains(cap)
    }
}

// --- Helper functions for minimal JSON parsing (no serde dependency) ---

fn extract_json_string(json: &str, key: &str) -> Result<String, String> {
    let search = format!("\"{}\":\"", key);
    if let Some(start) = json.find(&search) {
        let value_start = start + search.len();
        let rest = &json[value_start..];
        let mut end = 0;
        let bytes = rest.as_bytes();
        while end < bytes.len() {
            if bytes[end] == b'"' && (end == 0 || bytes[end - 1] != b'\\') {
                break;
            }
            end += 1;
        }
        if end < bytes.len() {
            return Ok(rest[..end].replace("\\\"", "\"").replace("\\\\", "\\"));
        }
    }
    Err(format!("Key '{}' not found", key))
}

fn extract_json_u64(json: &str, key: &str) -> Result<u64, String> {
    let search = format!("\"{}\":", key);
    if let Some(start) = json.find(&search) {
        let value_start = start + search.len();
        let rest = json[value_start..].trim_start();
        let end = rest.find(|c: char| !c.is_ascii_digit()).unwrap_or(rest.len());
        return rest[..end]
            .parse::<u64>()
            .map_err(|e| format!("Invalid u64 for '{}': {}", key, e));
    }
    Err(format!("Key '{}' not found", key))
}

fn extract_json_array(json: &str, key: &str) -> Result<Vec<String>, String> {
    let search = format!("\"{}\":[", key);
    if let Some(start) = json.find(&search) {
        let arr_start = start + search.len();
        let rest = &json[arr_start..];
        if let Some(arr_end) = rest.find(']') {
            let content = &rest[..arr_end];
            let items: Vec<String> = content
                .split(',')
                .map(|s| s.trim().trim_matches('"').to_string())
                .filter(|s| !s.is_empty())
                .collect();
            return Ok(items);
        }
    }
    Err(format!("Array '{}' not found", key))
}

fn find_matching_bracket(s: &str) -> Option<usize> {
    let mut depth = 0i32;
    for (i, c) in s.char_indices() {
        match c {
            '[' => depth += 1,
            '{' => depth += 1,
            ']' => {
                if depth == 0 {
                    return Some(i);
                }
                depth -= 1;
            }
            '}' => depth -= 1,
            _ => {}
        }
    }
    None
}

fn split_json_objects(s: &str) -> Vec<String> {
    let mut objects = Vec::new();
    let mut depth = 0i32;
    let mut start = None;

    for (i, c) in s.char_indices() {
        match c {
            '{' => {
                if depth == 0 {
                    start = Some(i);
                }
                depth += 1;
            }
            '}' => {
                depth -= 1;
                if depth == 0 {
                    if let Some(s_idx) = start {
                        objects.push(s[s_idx..=i].to_string());
                    }
                    start = None;
                }
            }
            _ => {}
        }
    }

    objects
}

fn current_timestamp() -> u64 {
    // Use a fixed value in non-test code for determinism in message creation;
    // real usage would use SystemTime.
    0
}

#[cfg(test)]
mod tests {
    use super::*;

    fn make_server() -> AcpServer {
        let mut server = AcpServer::new("test-server", AcpVersion::new(1, 0, 0));
        server.register_capability(AcpCapability::ToolExecution);
        server.register_capability(AcpCapability::FileEdit);
        server.register_capability(AcpCapability::Chat);
        let mut tool = AcpToolDef::new("read_file", "Reads a file", "string");
        tool.add_param("path", "string", "File path", true);
        server.register_tool(tool);
        server
    }

    fn make_client() -> AcpClient {
        let mut client = AcpClient::new("http://localhost:8080", AcpVersion::new(1, 0, 0));
        client.capabilities.push(AcpCapability::ToolExecution);
        client.capabilities.push(AcpCapability::Chat);
        client
    }

    #[test]
    fn test_version_display() {
        let v = AcpVersion::new(1, 2, 3);
        assert_eq!(v.to_string(), "1.2.3");
    }

    #[test]
    fn test_version_compatibility_same_major() {
        let v1 = AcpVersion::new(1, 0, 0);
        let v2 = AcpVersion::new(1, 5, 3);
        assert!(v1.is_compatible_with(&v2));
    }

    #[test]
    fn test_version_incompatible_different_major() {
        let v1 = AcpVersion::new(1, 0, 0);
        let v2 = AcpVersion::new(2, 0, 0);
        assert!(!v1.is_compatible_with(&v2));
    }

    #[test]
    fn test_capability_as_str_roundtrip() {
        let caps = vec![
            AcpCapability::ToolExecution,
            AcpCapability::FileEdit,
            AcpCapability::CodeCompletion,
            AcpCapability::Diagnostics,
            AcpCapability::Search,
            AcpCapability::Chat,
            AcpCapability::AgentSpawn,
            AcpCapability::ContextSharing,
        ];
        for cap in &caps {
            let s = cap.as_str();
            let parsed = AcpCapability::from_str(s).expect("Should parse back");
            assert_eq!(&parsed, cap);
        }
    }

    #[test]
    fn test_server_register_capability_dedup() {
        let mut server = AcpServer::new("s", AcpVersion::new(1, 0, 0));
        server.register_capability(AcpCapability::Chat);
        server.register_capability(AcpCapability::Chat);
        assert_eq!(server.capabilities.len(), 1);
    }

    #[test]
    fn test_server_register_tool() {
        let mut server = AcpServer::new("s", AcpVersion::new(1, 0, 0));
        assert_eq!(server.tools.len(), 0);
        server.register_tool(AcpToolDef::new("t", "desc", "void"));
        assert_eq!(server.tools.len(), 1);
        assert_eq!(server.tools[0].name, "t");
    }

    #[test]
    fn test_negotiate_success() {
        let server = make_server();
        let client_caps = vec![AcpCapability::ToolExecution, AcpCapability::Chat];
        let result = server.negotiate(&client_caps, &AcpVersion::new(1, 0, 0));
        assert_eq!(result.status, NegotiationStatus::Success);
        assert_eq!(result.shared_capabilities.len(), 2);
    }

    #[test]
    fn test_negotiate_partial_match() {
        let server = make_server();
        let client_caps = vec![AcpCapability::ToolExecution, AcpCapability::Diagnostics];
        let result = server.negotiate(&client_caps, &AcpVersion::new(1, 0, 0));
        assert_eq!(result.status, NegotiationStatus::PartialMatch);
        assert_eq!(result.shared_capabilities.len(), 1);
        assert_eq!(result.shared_capabilities[0], AcpCapability::ToolExecution);
    }

    #[test]
    fn test_negotiate_version_mismatch() {
        let server = make_server();
        let client_caps = vec![AcpCapability::ToolExecution];
        let result = server.negotiate(&client_caps, &AcpVersion::new(2, 0, 0));
        assert_eq!(result.status, NegotiationStatus::VersionMismatch);
        assert!(result.shared_capabilities.is_empty());
    }

    #[test]
    fn test_negotiate_failed_no_shared() {
        let server = make_server();
        let client_caps = vec![AcpCapability::Diagnostics, AcpCapability::AgentSpawn];
        let result = server.negotiate(&client_caps, &AcpVersion::new(1, 0, 0));
        assert_eq!(result.status, NegotiationStatus::Failed);
    }

    #[test]
    fn test_negotiate_agreed_version_uses_min_minor() {
        let mut server = AcpServer::new("s", AcpVersion::new(1, 5, 0));
        server.register_capability(AcpCapability::Chat);
        let result = server.negotiate(&[AcpCapability::Chat], &AcpVersion::new(1, 3, 0));
        assert_eq!(result.status, NegotiationStatus::Success);
        assert_eq!(result.agreed_version, AcpVersion::new(1, 3, 0));
    }

    #[test]
    fn test_message_json_roundtrip() {
        let msg = AcpMessage::new("msg-1", AcpMessageType::Heartbeat, "ping", 1000);
        let json = msg.to_json();
        let parsed = AcpMessage::from_json(&json).expect("Should parse");
        assert_eq!(parsed.id, "msg-1");
        assert_eq!(parsed.message_type, AcpMessageType::Heartbeat);
        assert_eq!(parsed.payload, "ping");
        assert_eq!(parsed.timestamp, 1000);
    }

    #[test]
    fn test_message_from_json_unknown_type() {
        let json = r#"{"id":"1","type":"unknown_type","payload":"x","timestamp":0}"#;
        assert!(AcpMessage::from_json(json).is_err());
    }

    #[test]
    fn test_handle_message_capability_request() {
        let server = make_server();
        let msg = AcpMessage::new("r1", AcpMessageType::CapabilityRequest, "", 100);
        let resp = server.handle_message(&msg);
        assert_eq!(resp.message_type, AcpMessageType::CapabilityResponse);
        assert!(resp.payload.contains("tool_execution"));
        assert!(resp.payload.contains("file_edit"));
    }

    #[test]
    fn test_handle_message_tool_call_found() {
        let server = make_server();
        let msg = AcpMessage::new("t1", AcpMessageType::ToolCall, r#"{"tool":"read_file"}"#, 200);
        let resp = server.handle_message(&msg);
        assert_eq!(resp.message_type, AcpMessageType::ToolResult);
        assert!(resp.payload.contains("ok"));
    }

    #[test]
    fn test_handle_message_tool_call_not_found() {
        let server = make_server();
        let msg = AcpMessage::new("t2", AcpMessageType::ToolCall, r#"{"tool":"nonexistent"}"#, 200);
        let resp = server.handle_message(&msg);
        assert_eq!(resp.message_type, AcpMessageType::Error);
        assert!(resp.payload.contains("404"));
    }

    #[test]
    fn test_handle_message_heartbeat() {
        let server = make_server();
        let msg = AcpMessage::new("h1", AcpMessageType::Heartbeat, "ping", 300);
        let resp = server.handle_message(&msg);
        assert_eq!(resp.message_type, AcpMessageType::Heartbeat);
        assert_eq!(resp.payload, "pong");
    }

    #[test]
    fn test_handle_message_disconnect() {
        let server = make_server();
        let msg = AcpMessage::new("d1", AcpMessageType::Disconnect, "", 400);
        let resp = server.handle_message(&msg);
        assert_eq!(resp.message_type, AcpMessageType::Disconnect);
    }

    #[test]
    fn test_handle_message_unhandled_type() {
        let server = make_server();
        let msg = AcpMessage::new("n1", AcpMessageType::Notification, "info", 500);
        let resp = server.handle_message(&msg);
        assert_eq!(resp.message_type, AcpMessageType::Error);
        assert!(resp.payload.contains("400"));
    }

    #[test]
    fn test_manifest_roundtrip() {
        let server = make_server();
        let json = server.to_manifest_json();
        let parsed = AcpServer::from_manifest_json(&json).expect("Should parse manifest");
        assert_eq!(parsed.server_name, "test-server");
        assert_eq!(parsed.version, AcpVersion::new(1, 0, 0));
        assert_eq!(parsed.capabilities.len(), 3);
        assert_eq!(parsed.tools.len(), 1);
        assert_eq!(parsed.tools[0].name, "read_file");
        assert_eq!(parsed.tools[0].parameters.len(), 1);
        assert_eq!(parsed.tools[0].parameters[0].name, "path");
        assert!(parsed.tools[0].parameters[0].required);
    }

    #[test]
    fn test_manifest_invalid_json() {
        let result = AcpServer::from_manifest_json("not json");
        assert!(result.is_err());
    }

    #[test]
    fn test_client_connect_success() {
        let server = make_server();
        let manifest = server.to_manifest_json();
        let mut client = make_client();
        let result = client.connect(&manifest).expect("Should connect");
        assert!(client.is_connected());
        assert_eq!(result.status, NegotiationStatus::Success);
    }

    #[test]
    fn test_client_connect_version_mismatch() {
        let server = make_server();
        let manifest = server.to_manifest_json();
        let mut client = AcpClient::new("http://localhost", AcpVersion::new(2, 0, 0));
        client.capabilities.push(AcpCapability::Chat);
        let result = client.connect(&manifest);
        assert!(result.is_err());
        assert!(!client.is_connected());
    }

    #[test]
    fn test_client_connect_no_shared_caps() {
        let server = make_server();
        let manifest = server.to_manifest_json();
        let mut client = AcpClient::new("http://localhost", AcpVersion::new(1, 0, 0));
        client.capabilities.push(AcpCapability::AgentSpawn);
        let result = client.connect(&manifest);
        assert!(result.is_err());
        assert!(!client.is_connected());
    }

    #[test]
    fn test_client_call_tool_not_connected() {
        let client = AcpClient::new("http://localhost", AcpVersion::new(1, 0, 0));
        let result = client.call_tool("test", "{}");
        assert!(result.is_err());
        assert_eq!(result.unwrap_err().code, 1);
    }

    #[test]
    fn test_client_call_tool_empty_name() {
        let server = make_server();
        let manifest = server.to_manifest_json();
        let mut client = make_client();
        client.connect(&manifest).expect("Should connect");
        let result = client.call_tool("", "{}");
        assert!(result.is_err());
        assert_eq!(result.unwrap_err().code, 2);
    }

    #[test]
    fn test_client_call_tool_success() {
        let server = make_server();
        let manifest = server.to_manifest_json();
        let mut client = make_client();
        client.connect(&manifest).expect("Should connect");
        let msg = client.call_tool("read_file", r#"{"path":"test.rs"}"#).expect("Should succeed");
        assert_eq!(msg.message_type, AcpMessageType::ToolCall);
        assert!(msg.payload.contains("read_file"));
    }

    #[test]
    fn test_client_disconnect() {
        let server = make_server();
        let manifest = server.to_manifest_json();
        let mut client = make_client();
        client.connect(&manifest).expect("Should connect");
        assert!(client.is_connected());
        let msg = client.disconnect();
        assert!(!client.is_connected());
        assert_eq!(msg.message_type, AcpMessageType::Disconnect);
    }

    #[test]
    fn test_client_supports_capability() {
        let client = make_client();
        assert!(client.supports_capability(&AcpCapability::ToolExecution));
        assert!(client.supports_capability(&AcpCapability::Chat));
        assert!(!client.supports_capability(&AcpCapability::Diagnostics));
    }

    #[test]
    fn test_acp_error_display() {
        let err = AcpError {
            code: 42,
            message: "Something went wrong".to_string(),
            data: Some("details".to_string()),
        };
        let display = format!("{}", err);
        assert!(display.contains("42"));
        assert!(display.contains("Something went wrong"));
    }

    #[test]
    fn test_tool_def_add_multiple_params() {
        let mut tool = AcpToolDef::new("search", "Search files", "results");
        tool.add_param("query", "string", "Search query", true);
        tool.add_param("limit", "number", "Max results", false);
        tool.add_param("regex", "bool", "Use regex", false);
        assert_eq!(tool.parameters.len(), 3);
        assert!(tool.parameters[0].required);
        assert!(!tool.parameters[1].required);
    }

    #[test]
    fn test_message_type_roundtrip() {
        let types = vec![
            AcpMessageType::CapabilityRequest,
            AcpMessageType::CapabilityResponse,
            AcpMessageType::ToolCall,
            AcpMessageType::ToolResult,
            AcpMessageType::Notification,
            AcpMessageType::Error,
            AcpMessageType::Heartbeat,
            AcpMessageType::Disconnect,
        ];
        for t in &types {
            let s = t.as_str();
            let parsed = AcpMessageType::from_str(s).expect("Should parse");
            assert_eq!(&parsed, t);
        }
    }

    #[test]
    fn test_server_protocol_id_format() {
        let server = AcpServer::new("my-server", AcpVersion::new(1, 2, 3));
        assert_eq!(server.protocol_id, "acp-1.2.3");
    }

    #[test]
    fn test_manifest_with_multiple_tools() {
        let mut server = AcpServer::new("multi", AcpVersion::new(1, 0, 0));
        server.register_capability(AcpCapability::ToolExecution);
        server.register_tool(AcpToolDef::new("tool_a", "First tool", "string"));
        server.register_tool(AcpToolDef::new("tool_b", "Second tool", "number"));

        let json = server.to_manifest_json();
        let parsed = AcpServer::from_manifest_json(&json).expect("Should parse");
        assert_eq!(parsed.tools.len(), 2);
        assert_eq!(parsed.tools[0].name, "tool_a");
        assert_eq!(parsed.tools[1].name, "tool_b");
    }
}
