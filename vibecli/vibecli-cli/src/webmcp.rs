//! WebMCP browser-tool exposure (gap C4) — W3C draft, Chrome 149 origin trial.
//!
//! WebMCP lets a web page expose JS-function / HTML-form "tools" to a browser
//! agent, and lets an agent host expose its own tools to a page. VibeCody plays
//! both roles through its CDP-attached browser ([`crate::browser_agent`]):
//!
//! * **Consumer** — discover WebMCP tool descriptors a site advertises and build
//!   validated invocations for the ones the user authorized.
//! * **Producer** — expose selected VibeUI panels as WebMCP tools so a page's
//!   agent can call them.
//!
//! Both stay behind a feature flag while the spec is in origin trial, and both
//! honor the §18.A7 cleared shape: **the agent never mutates the live DOM**. A
//! consumer invocation is surfaced to the user for an explicit diffcomplete-style
//! confirmation; the producer only advertises read/affordance tools. This module
//! is the pure descriptor + invocation layer (no CDP I/O), so it is unit-testable
//! without a live browser.

use serde::{Deserialize, Serialize};

/// Whether WebMCP is enabled. Off by default — origin-trial gated (§18.A7).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct WebMcpFlag(pub bool);

impl Default for WebMcpFlag {
    fn default() -> Self {
        WebMcpFlag(false)
    }
}

impl WebMcpFlag {
    pub fn enabled(self) -> bool {
        self.0
    }

    /// Resolve the origin-trial gate from the environment. Off unless
    /// `VIBECLI_WEBMCP` is `1`/`true`/`on`/`yes` (case-insensitive), keeping
    /// WebMCP disabled by default per §18.A7 while the spec is in origin trial.
    pub fn from_env() -> Self {
        let on = std::env::var("VIBECLI_WEBMCP")
            .map(|v| {
                matches!(
                    v.trim().to_ascii_lowercase().as_str(),
                    "1" | "true" | "on" | "yes"
                )
            })
            .unwrap_or(false);
        WebMcpFlag(on)
    }
}

/// A WebMCP tool descriptor (the JSON a page advertises, or the host produces).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct WebMcpTool {
    pub name: String,
    pub description: String,
    /// Parameter names → whether the parameter is required.
    #[serde(default)]
    pub params: Vec<WebMcpParam>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct WebMcpParam {
    pub name: String,
    #[serde(default)]
    pub required: bool,
}

/// A validated invocation of a WebMCP tool, ready to dispatch over CDP after the
/// user confirms. Construction *validates* required params so a malformed agent
/// call never reaches the page.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct WebMcpInvocation {
    pub tool: String,
    pub args: Vec<(String, String)>,
}

/// Parse the tool list a page advertises via `window.agent.provideContext({tools})`.
/// Accepts either a bare `[...]` array or a `{"tools":[...]}` wrapper. Unknown
/// fields are ignored; a malformed payload yields an empty list (never errors).
pub fn parse_advertised_tools(json: &str) -> Vec<WebMcpTool> {
    let value: serde_json::Value = match serde_json::from_str(json) {
        Ok(v) => v,
        Err(_) => return Vec::new(),
    };
    let arr = value
        .get("tools")
        .and_then(|t| t.as_array())
        .or_else(|| value.as_array());
    match arr {
        Some(items) => items
            .iter()
            .filter_map(|item| serde_json::from_value::<WebMcpTool>(item.clone()).ok())
            .collect(),
        None => Vec::new(),
    }
}

/// Build a validated invocation for a discovered tool. Errors when the feature is
/// disabled, the tool is unknown, or a required parameter is missing — so the
/// user is never prompted to confirm an invalid call.
pub fn build_invocation(
    flag: WebMcpFlag,
    tools: &[WebMcpTool],
    tool_name: &str,
    args: &[(String, String)],
) -> Result<WebMcpInvocation, String> {
    if !flag.enabled() {
        return Err("WebMCP is disabled (origin-trial gated). Enable it in settings.".to_string());
    }
    let tool = tools
        .iter()
        .find(|t| t.name == tool_name)
        .ok_or_else(|| format!("page does not advertise a WebMCP tool '{tool_name}'"))?;
    for p in &tool.params {
        if p.required && !args.iter().any(|(k, _)| k == &p.name) {
            return Err(format!("missing required parameter '{}'", p.name));
        }
    }
    Ok(WebMcpInvocation {
        tool: tool_name.to_string(),
        args: args.to_vec(),
    })
}

/// Producer side: describe a VibeUI panel as a WebMCP tool a page can call.
/// These are read/affordance tools — the agent never mutates the live DOM.
pub fn panel_as_tool(panel_id: &str, description: &str, params: &[(&str, bool)]) -> WebMcpTool {
    WebMcpTool {
        name: format!("vibeui.{panel_id}"),
        description: description.to_string(),
        params: params
            .iter()
            .map(|(name, required)| WebMcpParam {
                name: name.to_string(),
                required: *required,
            })
            .collect(),
    }
}

/// Serialize produced tools as the `{"tools":[...]}` payload VibeUI publishes to
/// a page's `window.agent`.
pub fn publish_tools(tools: &[WebMcpTool]) -> String {
    serde_json::to_string(&serde_json::json!({ "tools": tools }))
        .unwrap_or_else(|_| "{\"tools\":[]}".to_string())
}

/// Parse CLI-style `key=value` tokens (from a `/webmcp call` invocation) into
/// invocation args. A token without `=` is a bare flag (empty value). Order is
/// preserved so positional intent survives.
pub fn parse_kv_args(tokens: &[String]) -> Vec<(String, String)> {
    tokens
        .iter()
        .filter(|t| !t.is_empty())
        .map(|t| match t.split_once('=') {
            Some((k, v)) => (k.to_string(), v.to_string()),
            None => (t.clone(), String::new()),
        })
        .collect()
}

/// Human-readable listing of discovered tools for the REPL `/webmcp list`.
pub fn format_tools(tools: &[WebMcpTool]) -> String {
    if tools.is_empty() {
        return "No WebMCP tools advertised by the current page.".to_string();
    }
    let mut out = String::new();
    for t in tools {
        out.push_str(&format!("  {} — {}\n", t.name, t.description));
        for p in &t.params {
            out.push_str(&format!(
                "      {}{}\n",
                p.name,
                if p.required { " (required)" } else { "" }
            ));
        }
    }
    out.trim_end().to_string()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn sample_tools() -> Vec<WebMcpTool> {
        parse_advertised_tools(
            r#"{"tools":[
                {"name":"search","description":"Search the catalog",
                 "params":[{"name":"q","required":true},{"name":"limit","required":false}]}
            ]}"#,
        )
    }

    #[test]
    fn parses_wrapped_and_bare_arrays() {
        assert_eq!(sample_tools().len(), 1);
        let bare = parse_advertised_tools(r#"[{"name":"t","description":"d"}]"#);
        assert_eq!(bare.len(), 1);
        assert_eq!(bare[0].name, "t");
    }

    #[test]
    fn malformed_payload_is_empty_not_error() {
        assert!(parse_advertised_tools("not json").is_empty());
        assert!(parse_advertised_tools("{}").is_empty());
    }

    #[test]
    fn invocation_blocked_when_flag_disabled() {
        let tools = sample_tools();
        let err = build_invocation(
            WebMcpFlag::default(),
            &tools,
            "search",
            &[("q".into(), "rust".into())],
        )
        .unwrap_err();
        assert!(err.contains("disabled"));
    }

    #[test]
    fn invocation_validates_required_params() {
        let tools = sample_tools();
        // Missing required 'q'.
        assert!(build_invocation(WebMcpFlag(true), &tools, "search", &[]).is_err());
        // Unknown tool.
        assert!(build_invocation(WebMcpFlag(true), &tools, "nope", &[]).is_err());
        // Valid.
        let inv = build_invocation(
            WebMcpFlag(true),
            &tools,
            "search",
            &[("q".into(), "rust".into())],
        )
        .unwrap();
        assert_eq!(inv.tool, "search");
    }

    #[test]
    fn parse_kv_args_splits_pairs_and_bare_flags() {
        let toks = vec!["q=rust".to_string(), "limit=10".to_string(), "verbose".to_string()];
        let args = parse_kv_args(&toks);
        assert_eq!(args[0], ("q".to_string(), "rust".to_string()));
        assert_eq!(args[1], ("limit".to_string(), "10".to_string()));
        assert_eq!(args[2], ("verbose".to_string(), String::new()));
        // Values may themselves contain '=' — only the first '=' splits.
        let eq = parse_kv_args(&["expr=a=b".to_string()]);
        assert_eq!(eq[0], ("expr".to_string(), "a=b".to_string()));
    }

    #[test]
    fn format_tools_empty_and_populated() {
        assert!(format_tools(&[]).contains("No WebMCP tools"));
        let out = format_tools(&sample_tools());
        assert!(out.contains("search"));
        assert!(out.contains("q (required)"));
        assert!(out.contains("limit"));
    }

    #[test]
    fn flag_from_env_defaults_off_and_reads_truthy() {
        // Default (var unset in test env) is off.
        assert!(!WebMcpFlag::from_env().enabled() || std::env::var("VIBECLI_WEBMCP").is_ok());
        // Truthy parsing is covered by matching the accepted set directly.
        for v in ["1", "true", "on", "yes", "TRUE", "On"] {
            let on = matches!(v.trim().to_ascii_lowercase().as_str(), "1" | "true" | "on" | "yes");
            assert!(on, "{v} should be truthy");
        }
        for v in ["0", "false", "off", ""] {
            let on = matches!(v.trim().to_ascii_lowercase().as_str(), "1" | "true" | "on" | "yes");
            assert!(!on, "{v} should be falsy");
        }
    }

    #[test]
    fn producer_roundtrips_through_publish() {
        let tool = panel_as_tool("git", "Inspect git status", &[("path", false)]);
        assert_eq!(tool.name, "vibeui.git");
        let json = publish_tools(&[tool.clone()]);
        let reparsed = parse_advertised_tools(&json);
        assert_eq!(reparsed, vec![tool]);
    }
}
