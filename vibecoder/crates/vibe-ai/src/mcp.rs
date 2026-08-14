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
use std::io::{BufRead, BufReader, Read, Write};
use std::process::{Child, ChildStderr, ChildStdin, ChildStdout, Command, Stdio};
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
    /// The server's stderr, kept until something goes wrong.
    ///
    /// It used to be `Stdio::null()`, which threw away the only place a broken
    /// server says what happened. A Python reference server crashing on an
    /// import error reached the user as `invalid MCP response: EOF while
    /// parsing a value at line 1 column 0` — accurate about the empty stdout,
    /// silent about the traceback sitting in the pipe.
    stderr: Option<ChildStderr>,
}

/// How much of a failing server's stderr to keep. A server that crashes in a
/// loop can print without limit; the tail is where the cause is anyway.
const STDERR_TAIL_BYTES: u64 = 8 * 1024;

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
            // Piped, not null: see `McpClient::stderr`.
            .stderr(Stdio::piped());

        for (k, v) in &cfg.env {
            cmd.env(k, v);
        }

        let mut child = cmd
            .spawn()
            .with_context(|| format!("Failed to spawn MCP server '{}'", cfg.command))?;

        let stdin = child.stdin.take().context("no stdin on MCP server")?;
        let stdout = BufReader::new(child.stdout.take().context("no stdout on MCP server")?);
        let stderr = child.stderr.take();

        let mut client = Self {
            server_name: cfg.name.clone(),
            _child: child,
            stdin,
            stdout,
            stderr,
        };
        if let Err(e) = client.initialize() {
            // Say why, not just that. A server that dies during the handshake
            // has already explained itself on stderr; without this the caller
            // gets a parse error about the empty stdout and has to reproduce
            // the whole thing by hand to find out it was a version mismatch.
            let detail = client.stderr_tail();
            return Err(if detail.is_empty() {
                e
            } else {
                e.context(format!("server '{}' wrote: {detail}", cfg.name))
            });
        }
        Ok(client)
    }

    /// Kill the server and return the tail of what it printed to stderr.
    ///
    /// Killing first is what makes this terminate: the pipe EOFs when the
    /// process is gone, so the read cannot block on a server that is still
    /// running and quiet.
    pub fn stderr_tail(&mut self) -> String {
        let _ = self._child.kill();
        let Some(mut err) = self.stderr.take() else {
            return String::new();
        };
        let mut buf = Vec::new();
        let _ = Read::take(&mut err, STDERR_TAIL_BYTES).read_to_end(&mut buf);
        let text = String::from_utf8_lossy(&buf);
        // The last few lines carry the cause; earlier ones are usually the
        // package manager narrating a download.
        let tail: Vec<&str> = text
            .lines()
            .filter(|l| !l.trim().is_empty())
            .rev()
            .take(6)
            .collect();
        tail.into_iter().rev().collect::<Vec<_>>().join("\n")
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
            anyhow::bail!(
                "MCP error {} from '{}': {}",
                e.code,
                self.server_name,
                e.message
            );
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
        let mut prompt = String::from(
            "\n\n## MCP Tools\n\nAdditional tools available via connected MCP servers:\n\n",
        );
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
        assert_eq!(
            back.env.get("GITHUB_TOKEN").map(|s| s.as_str()),
            Some("secret")
        );
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
        assert!(
            prompt.contains("mcp__github__list_repos"),
            "prompt should contain mcp__<server>__<tool> format"
        );
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
            env: [("TOKEN".to_string(), "abc".to_string())]
                .into_iter()
                .collect(),
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

    // ── McpTool field validation ─────────────────────────────────────────

    #[test]
    fn mcp_tool_with_complex_input_schema() {
        let tool = McpTool {
            name: "create_issue".to_string(),
            description: "Create a GitHub issue".to_string(),
            server: "github".to_string(),
            input_schema: serde_json::json!({
                "type": "object",
                "required": ["title"],
                "properties": {
                    "title": { "type": "string" },
                    "body": { "type": "string" },
                    "labels": { "type": "array", "items": { "type": "string" } }
                }
            }),
        };
        assert!(tool.input_schema["required"].is_array());
        assert_eq!(tool.input_schema["properties"]["title"]["type"], "string");
    }

    #[test]
    fn mcp_tool_empty_fields() {
        let tool = make_tool("", "", "");
        let json = serde_json::to_string(&tool).unwrap();
        let back: McpTool = serde_json::from_str(&json).unwrap();
        assert_eq!(back.name, "");
        assert_eq!(back.server, "");
        assert_eq!(back.description, "");
    }

    #[test]
    fn mcp_tool_clone() {
        let tool = make_tool("github", "list_repos", "desc");
        let cloned = tool.clone();
        assert_eq!(cloned.name, tool.name);
        assert_eq!(cloned.server, tool.server);
    }

    // ── McpServerConfig edge cases ──────────────────────────────────────

    #[test]
    fn server_config_with_multiple_env_vars() {
        let cfg = McpServerConfig {
            name: "db".to_string(),
            command: "db-server".to_string(),
            args: vec![],
            env: [
                ("DB_HOST".to_string(), "localhost".to_string()),
                ("DB_PORT".to_string(), "5432".to_string()),
                ("DB_USER".to_string(), "admin".to_string()),
            ]
            .into_iter()
            .collect(),
        };
        assert_eq!(cfg.env.len(), 3);
        assert_eq!(cfg.env["DB_PORT"], "5432");
    }

    #[test]
    fn server_config_with_inline_args_in_command() {
        // The connect() method splits command on whitespace.
        // Test the config itself stores the full command string.
        let cfg = McpServerConfig {
            name: "test".to_string(),
            command: "npx @mcp/server --port 3000".to_string(),
            args: vec!["--verbose".to_string()],
            ..Default::default()
        };
        assert!(cfg.command.contains("npx"));
        assert!(cfg.command.contains("--port"));
        assert_eq!(cfg.args.len(), 1);
    }

    #[test]
    fn server_config_debug_format() {
        let cfg = McpServerConfig::default();
        let debug = format!("{:?}", cfg);
        assert!(debug.contains("McpServerConfig"));
    }

    // ── tools_prompt edge cases ─────────────────────────────────────────

    #[test]
    fn tools_prompt_special_chars_in_names() {
        let tools = vec![make_tool(
            "my-server",
            "list_all-items",
            "List items with dashes",
        )];
        let prompt = McpClient::tools_prompt(&tools);
        assert!(prompt.contains("mcp__my-server__list_all-items"));
    }

    #[test]
    fn tools_prompt_many_tools() {
        let tools: Vec<McpTool> = (0..20)
            .map(|i| {
                make_tool(
                    "server",
                    &format!("tool_{}", i),
                    &format!("Tool number {}", i),
                )
            })
            .collect();
        let prompt = McpClient::tools_prompt(&tools);
        assert!(prompt.contains("mcp__server__tool_0"));
        assert!(prompt.contains("mcp__server__tool_19"));
    }

    // ── next_id concurrency safety ──────────────────────────────────────

    #[test]
    fn next_id_many_calls_unique() {
        let ids: Vec<u64> = (0..100).map(|_| next_id()).collect();
        let unique: std::collections::HashSet<u64> = ids.iter().copied().collect();
        assert_eq!(unique.len(), 100, "All IDs should be unique");
    }

    // ── JSON-RPC response deserialization ────────────────────────────────

    #[test]
    fn json_rpc_response_with_result() {
        let json = r#"{"id": 1, "result": {"tools": []}, "error": null}"#;
        let resp: JsonRpcResponse = serde_json::from_str(json).unwrap();
        assert!(resp.result.is_some());
        assert!(resp.error.is_none());
    }

    #[test]
    fn json_rpc_response_with_error() {
        let json =
            r#"{"id": 1, "result": null, "error": {"code": -32600, "message": "Invalid Request"}}"#;
        let resp: JsonRpcResponse = serde_json::from_str(json).unwrap();
        assert!(resp.error.is_some());
        let err = resp.error.unwrap();
        assert_eq!(err.code, -32600);
        assert_eq!(err.message, "Invalid Request");
    }

    #[test]
    fn json_rpc_response_missing_optional_fields() {
        let json = r#"{"id": null}"#;
        let resp: JsonRpcResponse = serde_json::from_str(json).unwrap();
        assert!(resp.result.is_none());
        assert!(resp.error.is_none());
    }

    /// A server that dies during the handshake must say why.
    ///
    /// Before stderr was piped, this exact shape — a process that prints its
    /// error and exits without writing to stdout — surfaced as "invalid MCP
    /// response: EOF while parsing a value at line 1 column 0", which describes
    /// the empty stdout and not the cause. Three catalog connectors failed this
    /// way and the reason had to be reproduced by hand.
    #[test]
    #[cfg(unix)]
    fn a_server_that_crashes_reports_what_it_printed() {
        let cfg = McpServerConfig {
            name: "crasher".into(),
            command: "/bin/sh".into(),
            args: vec![
                "-c".into(),
                "echo 'ImportError: cannot import name McpError' >&2; exit 1".into(),
            ],
            env: Default::default(),
        };
        let Err(err) = McpClient::connect(&cfg) else {
            panic!("this server cannot handshake");
        };
        let text = format!("{err:#}");
        assert!(
            text.contains("cannot import name McpError"),
            "the cause was dropped: {text}"
        );
    }

    /// A server with nothing to say must not turn into a fabricated one.
    #[test]
    #[cfg(unix)]
    fn a_silent_crash_reports_the_transport_error_alone() {
        let cfg = McpServerConfig {
            name: "quiet".into(),
            command: "/bin/sh".into(),
            args: vec!["-c".into(), "exit 1".into()],
            env: Default::default(),
        };
        let Err(err) = McpClient::connect(&cfg) else {
            panic!("this server cannot handshake");
        };
        let text = format!("{err:#}");
        assert!(
            !text.contains("wrote:"),
            "invented a stderr message: {text}"
        );
    }
}
