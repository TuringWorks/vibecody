//! Model Context Protocol (MCP) client — JSON-RPC 2.0 over stdio.
//!
//! Spawns an MCP server process, performs the initialize handshake,
//! lists available tools, and executes tool calls.
//!
//! # Usage
//! ```no_run
//! use vibe_ai::mcp::{McpClient, McpServerConfig};
//!
//! let cfg = McpServerConfig {
//!     name: "github".to_string(),
//!     command: "npx @modelcontextprotocol/server-github".to_string(),
//!     args: vec![],
//!     env: Default::default(),
//! };
//! let mut client = McpClient::connect(&cfg)?;
//! let tools = client.list_tools()?;
//! let output = client.call_tool("list_repos", serde_json::json!({}))?;
//! # Ok::<(), anyhow::Error>(())
//! ```

use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use std::collections::HashMap;
use std::io::{BufRead, BufReader, Write};
use std::process::{Child, ChildStdin, ChildStdout, Command, Stdio};
use std::sync::atomic::{AtomicU64, Ordering};

static REQUEST_ID: AtomicU64 = AtomicU64::new(1);

fn next_id() -> u64 {
    REQUEST_ID.fetch_add(1, Ordering::SeqCst)
}

// ── JSON-RPC types ────────────────────────────────────────────────────────────

#[derive(Serialize)]
struct JsonRpcRequest {
    jsonrpc: &'static str,
    id: u64,
    method: String,
    params: Value,
}

#[derive(Deserialize)]
struct JsonRpcResponse {
    #[allow(dead_code)]
    id: Option<Value>,
    result: Option<Value>,
    error: Option<RpcError>,
}

#[derive(Deserialize)]
struct RpcError {
    code: i64,
    message: String,
}

// ── Public types ──────────────────────────────────────────────────────────────

/// A tool exposed by an MCP server.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct McpTool {
    /// Tool name (unique within the server).
    pub name: String,
    /// Human-readable description shown to the LLM.
    pub description: String,
    /// Name of the MCP server that owns this tool.
    pub server: String,
    /// JSON Schema for the tool's input arguments.
    pub input_schema: Value,
}

/// Configuration for one MCP server (one `[[mcp_servers]]` TOML entry).
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct McpServerConfig {
    /// Logical name (e.g. `"github"`, `"postgres"`).
    pub name: String,
    /// Shell command to launch the server (e.g. `"npx @modelcontextprotocol/server-github"`).
    pub command: String,
    /// Extra arguments appended after the command.
    #[serde(default)]
    pub args: Vec<String>,
    /// Optional environment variables injected into the server process.
    #[serde(default)]
    pub env: HashMap<String, String>,
}

// ── McpClient ─────────────────────────────────────────────────────────────────

/// A running MCP server process with a JSON-RPC 2.0 stdio transport.
pub struct McpClient {
    server_name: String,
    _child: Child,
    stdin: ChildStdin,
    stdout: BufReader<ChildStdout>,
}

impl McpClient {
    /// Spawn the MCP server described by `cfg` and perform the initialize
    /// handshake.  Returns `Err` if the process cannot be started or the
    /// handshake fails.
    pub fn connect(cfg: &McpServerConfig) -> Result<Self> {
        // Split `command` into program + inline args.
        let mut parts = cfg.command.split_whitespace();
        let prog = parts.next().context("MCP command is empty")?;
        let inline_args: Vec<&str> = parts.collect();

        let mut cmd = Command::new(prog);
        cmd.args(&inline_args)
            .args(&cfg.args)
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::null());

        for (k, v) in &cfg.env {
            cmd.env(k, v);
        }

        let mut child = cmd
            .spawn()
            .with_context(|| format!("Failed to spawn MCP server '{}'", cfg.command))?;

        let stdin = child.stdin.take().context("no stdin on MCP server")?;
        let stdout = BufReader::new(child.stdout.take().context("no stdout on MCP server")?);

        let mut client = Self {
            server_name: cfg.name.clone(),
            _child: child,
            stdin,
            stdout,
        };
        client.initialize()?;
        Ok(client)
    }

    // ── Internal JSON-RPC helpers ─────────────────────────────────────────

    fn send(&mut self, method: &str, params: Value) -> Result<Value> {
        let req = JsonRpcRequest {
            jsonrpc: "2.0",
            id: next_id(),
            method: method.to_string(),
            params,
        };
        let line = serde_json::to_string(&req)?;
        writeln!(self.stdin, "{}", line)?;
        self.stdin.flush()?;

        let mut resp_line = String::new();
        self.stdout
            .read_line(&mut resp_line)
            .context("MCP server closed unexpectedly")?;

        let resp: JsonRpcResponse = serde_json::from_str(resp_line.trim())
            .with_context(|| format!("invalid MCP response: {}", resp_line.trim()))?;

        if let Some(e) = resp.error {
            anyhow::bail!("MCP error {} from '{}': {}", e.code, self.server_name, e.message);
        }
        Ok(resp.result.unwrap_or(Value::Null))
    }

    fn notify(&mut self, method: &str, params: Value) -> Result<()> {
        let notif = json!({ "jsonrpc": "2.0", "method": method, "params": params });
        writeln!(self.stdin, "{}", serde_json::to_string(&notif)?)?;
        self.stdin.flush()?;
        Ok(())
    }

    fn initialize(&mut self) -> Result<()> {
        self.send(
            "initialize",
            json!({
                "protocolVersion": "2024-11-05",
                "capabilities": {},
                "clientInfo": { "name": "vibecli", "version": env!("CARGO_PKG_VERSION") }
            }),
        )?;
        self.notify("notifications/initialized", json!({}))?;
        Ok(())
    }

    // ── Public API ────────────────────────────────────────────────────────

    /// Fetch the list of tools available on this server.
    pub fn list_tools(&mut self) -> Result<Vec<McpTool>> {
        let result = self.send("tools/list", json!({}))?;
        let arr = result["tools"].as_array().cloned().unwrap_or_default();
        Ok(arr
            .into_iter()
            .map(|t| McpTool {
                name: t["name"].as_str().unwrap_or("").to_string(),
                description: t["description"].as_str().unwrap_or("").to_string(),
                server: self.server_name.clone(),
                input_schema: t.get("inputSchema").cloned().unwrap_or(Value::Null),
            })
            .collect())
    }

    /// Call a tool and return its text output.
    pub fn call_tool(&mut self, name: &str, arguments: Value) -> Result<String> {
        let result = self.send(
            "tools/call",
            json!({ "name": name, "arguments": arguments }),
        )?;
        let content = result["content"].as_array().cloned().unwrap_or_default();
        let mut out = String::new();
        for item in content {
            if item["type"].as_str() == Some("text") {
                if !out.is_empty() {
                    out.push('\n');
                }
                out.push_str(item["text"].as_str().unwrap_or(""));
            }
        }
        Ok(out)
    }

    /// Build a human-readable TOOL_SYSTEM_PROMPT fragment listing MCP tools.
    pub fn tools_prompt(tools: &[McpTool]) -> String {
        if tools.is_empty() {
            return String::new();
        }
        let mut prompt = String::from("\n\n## MCP Tools\n\nAdditional tools available via connected MCP servers:\n\n");
        for tool in tools {
            prompt.push_str(&format!(
                "### mcp/{}/{}\n{}\n\nCall with:\n```\n<tool_call name=\"mcp__{}__{}\">\n<arguments>{{\"key\": \"value\"}}</arguments>\n</tool_call>\n```\n\n",
                tool.server,
                tool.name,
                tool.description,
                tool.server,
                tool.name,
            ));
        }
        prompt
    }
}

// ── Tests ─────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    fn make_tool(server: &str, name: &str, desc: &str) -> McpTool {
        McpTool {
            name: name.to_string(),
            description: desc.to_string(),
            server: server.to_string(),
            input_schema: serde_json::Value::Null,
        }
    }

    // ── McpServerConfig ───────────────────────────────────────────────────────

    #[test]
    fn server_config_default_has_empty_args_and_env() {
        let cfg = McpServerConfig {
            name: "test".to_string(),
            command: "echo".to_string(),
            ..Default::default()
        };
        assert!(cfg.args.is_empty());
        assert!(cfg.env.is_empty());
    }

    #[test]
    fn server_config_roundtrips_json() {
        let cfg = McpServerConfig {
            name: "github".to_string(),
            command: "npx @modelcontextprotocol/server-github".to_string(),
            args: vec!["--token".to_string(), "abc".to_string()],
            env: [("GITHUB_TOKEN".to_string(), "secret".to_string())]
                .into_iter()
                .collect(),
        };
        let json = serde_json::to_string(&cfg).unwrap();
        let back: McpServerConfig = serde_json::from_str(&json).unwrap();
        assert_eq!(back.name, cfg.name);
        assert_eq!(back.command, cfg.command);
        assert_eq!(back.args, cfg.args);
        assert_eq!(back.env.get("GITHUB_TOKEN").map(|s| s.as_str()), Some("secret"));
    }

    // ── McpTool ───────────────────────────────────────────────────────────────

    #[test]
    fn mcp_tool_serializes_fields() {
        let tool = make_tool("github", "list_repos", "Lists repositories");
        let json = serde_json::to_string(&tool).unwrap();
        assert!(json.contains("\"name\":\"list_repos\""));
        assert!(json.contains("\"server\":\"github\""));
        assert!(json.contains("\"description\":\"Lists repositories\""));
    }

    // ── tools_prompt ──────────────────────────────────────────────────────────

    #[test]
    fn tools_prompt_empty_returns_empty_string() {
        assert_eq!(McpClient::tools_prompt(&[]), "");
    }

    #[test]
    fn tools_prompt_contains_mcp_tool_call_format() {
        let tools = vec![make_tool("github", "list_repos", "Lists repositories")];
        let prompt = McpClient::tools_prompt(&tools);
        assert!(prompt.contains("mcp__github__list_repos"),
            "prompt should contain mcp__<server>__<tool> format");
        assert!(prompt.contains("Lists repositories"));
    }

    #[test]
    fn tools_prompt_contains_all_tools() {
        let tools = vec![
            make_tool("github", "list_repos", "List repos"),
            make_tool("postgres", "query", "Run SQL"),
        ];
        let prompt = McpClient::tools_prompt(&tools);
        assert!(prompt.contains("mcp__github__list_repos"));
        assert!(prompt.contains("mcp__postgres__query"));
    }

    #[test]
    fn tools_prompt_has_mcp_tools_header() {
        let tools = vec![make_tool("s", "t", "d")];
        let prompt = McpClient::tools_prompt(&tools);
        assert!(prompt.contains("## MCP Tools"));
    }

    // ── next_id monotonically increasing ─────────────────────────────────

    #[test]
    fn next_id_monotonically_increasing() {
        let id1 = next_id();
        let id2 = next_id();
        let id3 = next_id();
        assert!(id2 > id1);
        assert!(id3 > id2);
    }

    // ── McpServerConfig serde with defaults ──────────────────────────────

    #[test]
    fn server_config_missing_optional_fields_uses_defaults() {
        let json = r#"{"name": "test", "command": "echo hello"}"#;
        let cfg: McpServerConfig = serde_json::from_str(json).unwrap();
        assert_eq!(cfg.name, "test");
        assert_eq!(cfg.command, "echo hello");
        assert!(cfg.args.is_empty());
        assert!(cfg.env.is_empty());
    }

    #[test]
    fn server_config_toml_roundtrip() {
        let cfg = McpServerConfig {
            name: "github".to_string(),
            command: "npx @mcp/server-github".to_string(),
            args: vec!["--verbose".to_string()],
            env: [("TOKEN".to_string(), "abc".to_string())].into_iter().collect(),
        };
        let toml_str = toml::to_string(&cfg).unwrap();
        let back: McpServerConfig = toml::from_str(&toml_str).unwrap();
        assert_eq!(back.name, "github");
        assert_eq!(back.args, vec!["--verbose"]);
        assert_eq!(back.env.get("TOKEN").map(|s| s.as_str()), Some("abc"));
    }

    // ── McpTool roundtrip ────────────────────────────────────────────────

    #[test]
    fn mcp_tool_serde_roundtrip() {
        let tool = McpTool {
            name: "create_pr".to_string(),
            description: "Creates a pull request".to_string(),
            server: "github".to_string(),
            input_schema: serde_json::json!({
                "type": "object",
                "properties": {
                    "title": { "type": "string" },
                    "body": { "type": "string" }
                }
            }),
        };
        let json = serde_json::to_string(&tool).unwrap();
        let back: McpTool = serde_json::from_str(&json).unwrap();
        assert_eq!(back.name, "create_pr");
        assert_eq!(back.description, "Creates a pull request");
        assert_eq!(back.server, "github");
        assert!(back.input_schema.is_object());
    }

    // ── tools_prompt formatting ──────────────────────────────────────────

    #[test]
    fn tools_prompt_includes_server_slash_tool() {
        let tools = vec![make_tool("github", "list_issues", "List issues")];
        let prompt = McpClient::tools_prompt(&tools);
        assert!(prompt.contains("mcp/github/list_issues"));
    }

    #[test]
    fn tools_prompt_includes_tool_call_xml_format() {
        let tools = vec![make_tool("db", "query", "Run query")];
        let prompt = McpClient::tools_prompt(&tools);
        assert!(prompt.contains("<tool_call name=\"mcp__db__query\">"));
        assert!(prompt.contains("<arguments>"));
    }
}
