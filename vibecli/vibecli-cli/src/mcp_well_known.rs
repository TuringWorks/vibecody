#![allow(dead_code)] // Staged wave6 / Phase 53 module — wired up in a later cycle
//! `/.well-known/mcp.json` — stateless capability advertisement for the
//! daemon's MCP surface.
//!
//! Phase 53 P0 (A3 from v13 fitgap, MCP 2026 roadmap). Lets a host
//! discover the daemon's tools / prompts / resources catalogue without
//! opening a live SSE connection — required for horizontal scale and
//! for HTTP-only inspectors.
//!
//! The shape is intentionally a strict subset of what `tools/list` +
//! `prompts/list` + `resources/list` over JSON-RPC return, packed into
//! one envelope:
//!
//! ```json
//! {
//!   "name":            "vibecli",
//!   "version":         "0.5.7",
//!   "protocolVersion": "2024-11-05",
//!   "transports":      ["stdio", "http", "sse", "streamable_http"],
//!   "tools":           [{"name": "...", "description": "..."}],
//!   "prompts":         [],
//!   "resources":       []
//! }
//! ```
//!
//! Pure function — no IO. Hosts call `GET /.well-known/mcp.json` and
//! the route handler emits whatever this returns; the daemon's
//! `mcp_server::tool_defs()` provides the source of truth that
//! `tools_from_mcp_defs` translates.

use serde::Serialize;
use serde_json::Value;

/// MCP transport advertised in the well-known descriptor. Mirrors
/// the names used in the 2025-11-25 spec.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum McpTransport {
    Stdio,
    Http,
    Sse,
    StreamableHttp,
}

#[derive(Debug, Clone, Serialize)]
pub struct ToolDescriptor {
    pub name: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
}

#[derive(Debug, Clone, Serialize)]
pub struct PromptDescriptor {
    pub name: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
}

#[derive(Debug, Clone, Serialize)]
pub struct ResourceDescriptor {
    pub uri: String,
    pub name: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
}

#[derive(Debug, Clone, Serialize)]
pub struct WellKnownMcp {
    pub name: String,
    pub version: String,
    #[serde(rename = "protocolVersion")]
    pub protocol_version: String,
    pub transports: Vec<McpTransport>,
    pub tools: Vec<ToolDescriptor>,
    pub prompts: Vec<PromptDescriptor>,
    pub resources: Vec<ResourceDescriptor>,
}

/// Build the well-known descriptor from the daemon's configured tool
/// list.
pub fn build_well_known(
    server_name: &str,
    server_version: &str,
    tools: Vec<ToolDescriptor>,
) -> WellKnownMcp {
    WellKnownMcp {
        name: server_name.to_string(),
        version: server_version.to_string(),
        // Matches the version mcp_server::dispatch advertises in
        // initialize. Bumped lockstep with the upstream MCP spec.
        protocol_version: "2024-11-05".to_string(),
        transports: vec![
            McpTransport::Stdio,
            McpTransport::Http,
            McpTransport::Sse,
            McpTransport::StreamableHttp,
        ],
        tools,
        prompts: Vec::new(),
        resources: Vec::new(),
    }
}

/// Translate the existing `mcp_server::tool_defs()` JSON shape (Vec<Value>)
/// into our `ToolDescriptor` list. The mcp_server module already owns
/// the tool definitions; this helper avoids duplicating them.
pub fn tools_from_mcp_defs(defs: &[Value]) -> Vec<ToolDescriptor> {
    defs.iter()
        .filter_map(|d| {
            let name = d.get("name")?.as_str()?.to_string();
            let description = d
                .get("description")
                .and_then(|v| v.as_str())
                .map(|s| s.to_string());
            Some(ToolDescriptor { name, description })
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    fn fixture_tools() -> Vec<ToolDescriptor> {
        vec![
            ToolDescriptor {
                name: "read_file".into(),
                description: Some("Read the full contents of a file.".into()),
            },
            ToolDescriptor {
                name: "bash".into(),
                description: Some("Execute a shell command.".into()),
            },
        ]
    }

    #[test]
    fn build_well_known_includes_server_identity_and_protocol_version() {
        let wk = build_well_known("vibecli", "0.5.7", fixture_tools());
        assert_eq!(wk.name, "vibecli");
        assert_eq!(wk.version, "0.5.7");
        // Matches the version mcp_server::dispatch advertises in initialize.
        assert_eq!(wk.protocol_version, "2024-11-05");
    }

    #[test]
    fn build_well_known_advertises_default_transports() {
        let wk = build_well_known("vibecli", "0.5.7", vec![]);
        assert!(wk.transports.contains(&McpTransport::Stdio));
        assert!(wk.transports.contains(&McpTransport::Http));
        assert!(wk.transports.contains(&McpTransport::StreamableHttp));
    }

    #[test]
    fn build_well_known_passes_through_tools() {
        let wk = build_well_known("vibecli", "0.5.7", fixture_tools());
        assert_eq!(wk.tools.len(), 2);
        assert_eq!(wk.tools[0].name, "read_file");
        assert_eq!(wk.tools[1].name, "bash");
    }

    #[test]
    fn tools_from_mcp_defs_extracts_name_and_description() {
        let defs = vec![
            json!({"name": "read_file", "description": "Read the full contents of a file."}),
            json!({"name": "no_desc"}),
            json!({"description": "missing name"}), // skipped
        ];
        let tools = tools_from_mcp_defs(&defs);
        assert_eq!(tools.len(), 2, "entries without name should be dropped");
        assert_eq!(tools[0].name, "read_file");
        assert_eq!(
            tools[0].description.as_deref(),
            Some("Read the full contents of a file.")
        );
        assert_eq!(tools[1].name, "no_desc");
        assert!(tools[1].description.is_none());
    }

    #[test]
    fn well_known_serialises_to_expected_shape() {
        let wk = build_well_known("vibecli", "0.5.7", fixture_tools());
        let v = serde_json::to_value(&wk).unwrap();
        assert_eq!(v["name"], "vibecli");
        assert_eq!(v["version"], "0.5.7");
        assert_eq!(v["protocolVersion"], "2024-11-05");
        assert!(v["transports"].is_array());
        assert!(v["tools"].is_array());
        assert!(v["prompts"].is_array());
        assert!(v["resources"].is_array());
    }
}
