//! MCP Apps payload — `application/vnd.mcp.app+json` parser + validator.
//!
//! Phase 53 P0 (A1 from v13 fitgap, SEP-1865 / experimental-ext-skills
//! May 4 2026). Payload shape mirrors the formal extension proposal:
//!
//! ```json
//! {
//!   "type":      "mcp.app",
//!   "version":   "0.1",
//!   "title":     "Issue triage dashboard",
//!   "component": "react@18",
//!   "props":     { "issues": [...] },
//!   "actions":   [{ "id": "assign", "label": "Assign to me" }],
//!   "csp": {
//!     "allowHttp":   ["api.github.com"],
//!     "allowScript": ["self"]
//!   }
//! }
//! ```
//!
//! This module is the BACKEND parser/validator that gates payloads
//! before they reach the renderer. The React renderer is a frontend
//! follow-up tracked separately — keeping the two halves independent
//! lets either one ship first without blocking the other.
//!
//! Red commit: types + signatures + 6 BDD scenarios. Impl bodies
//! `todo!()` so tests panic at runtime — TDD red. Green commit fills
//! in the bodies.

use anyhow::Result;
use serde::{Deserialize, Serialize};

pub const MIME_TYPE: &str = "application/vnd.mcp.app+json";
pub const APP_TYPE: &str = "mcp.app";

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct McpAppPayload {
    /// Always `"mcp.app"`.
    #[serde(rename = "type")]
    pub kind: String,
    pub version: String,
    pub title: String,
    /// Component descriptor — `"react@18"`, `"react@19"`, etc.
    pub component: String,
    #[serde(default)]
    pub props: serde_json::Value,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub actions: Vec<McpAppAction>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub csp: Option<McpAppCsp>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct McpAppAction {
    pub id: String,
    pub label: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct McpAppCsp {
    #[serde(default, rename = "allowHttp", skip_serializing_if = "Vec::is_empty")]
    pub allow_http: Vec<String>,
    #[serde(default, rename = "allowScript", skip_serializing_if = "Vec::is_empty")]
    pub allow_script: Vec<String>,
}

/// Parse a raw byte string carrying `application/vnd.mcp.app+json`.
/// Returns the typed payload or an error explaining why it was rejected.
pub fn parse(_bytes: &[u8]) -> Result<McpAppPayload> {
    todo!("A1: serde_json::from_slice into McpAppPayload, then validate()");
}

/// Validate a parsed payload. Catches missing required fields, wrong
/// `type`, and CSP shapes that are obviously malformed (e.g.
/// `allowScript` containing `"*"` which would defeat the gate).
pub fn validate(_payload: &McpAppPayload) -> Result<()> {
    todo!("A1: enforce kind == APP_TYPE, non-empty title/component, action ids unique, CSP wildcards rejected");
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    fn fixture_payload() -> McpAppPayload {
        McpAppPayload {
            kind: APP_TYPE.into(),
            version: "0.1".into(),
            title: "Issue triage".into(),
            component: "react@18".into(),
            props: json!({"issues": []}),
            actions: vec![McpAppAction {
                id: "assign".into(),
                label: "Assign to me".into(),
                description: None,
            }],
            csp: Some(McpAppCsp {
                allow_http: vec!["api.github.com".into()],
                allow_script: vec!["self".into()],
            }),
        }
    }

    #[test]
    fn parse_and_validate_round_trip_canonical_payload() {
        let bytes = serde_json::to_vec(&fixture_payload()).unwrap();
        let parsed = parse(&bytes).unwrap();
        assert_eq!(parsed, fixture_payload());
        validate(&parsed).unwrap();
    }

    #[test]
    fn parse_rejects_wrong_type_field() {
        let mut p = fixture_payload();
        p.kind = "mcp.notapp".into();
        let bytes = serde_json::to_vec(&p).unwrap();
        let err = parse(&bytes).unwrap_err();
        assert!(err.to_string().contains("mcp.app"), "got {err}");
    }

    #[test]
    fn validate_rejects_empty_title() {
        let mut p = fixture_payload();
        p.title = "".into();
        let err = validate(&p).unwrap_err();
        assert!(err.to_string().to_lowercase().contains("title"));
    }

    #[test]
    fn validate_rejects_duplicate_action_ids() {
        let mut p = fixture_payload();
        p.actions = vec![
            McpAppAction { id: "go".into(), label: "Go".into(), description: None },
            McpAppAction { id: "go".into(), label: "Again".into(), description: None },
        ];
        let err = validate(&p).unwrap_err();
        assert!(err.to_string().to_lowercase().contains("duplicate"));
    }

    #[test]
    fn validate_rejects_csp_wildcard_script_source() {
        let mut p = fixture_payload();
        p.csp = Some(McpAppCsp {
            allow_http: vec![],
            allow_script: vec!["*".into()],
        });
        let err = validate(&p).unwrap_err();
        assert!(err.to_string().to_lowercase().contains("wildcard"));
    }

    #[test]
    fn parse_rejects_invalid_json() {
        let err = parse(b"{not json").unwrap_err();
        assert!(
            err.to_string().to_lowercase().contains("json")
                || err.to_string().to_lowercase().contains("parse"),
            "got {err}"
        );
    }
}
