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
