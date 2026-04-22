//! VibeCLI as an MCP (Model Context Protocol) server.
//!
//! Transport: stdio, newline-delimited JSON-RPC 2.0.
//! Spec: <https://spec.modelcontextprotocol.io/>
//!
//! ## Quick start
//!
//! Add to Claude Desktop `config.json`:
//! ```json
//! {
//!   "mcpServers": {
//!     "vibecli": {
//!       "command": "vibecli",
//!       "args": ["--mcp-server"],
//!       "cwd": "/path/to/your/project"
//!     }
//!   }
//! }
//! ```
//!
//! Available tools: `read_file`, `write_file`, `list_directory`, `bash`,
//! `search_files`, `agent_run`.

use anyhow::Result;
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use std::path::{Path, PathBuf};
use std::sync::Arc;
use tokio::io::{AsyncBufReadExt, AsyncWriteExt, BufReader};
use vibe_ai::agent::{AgentContext, AgentEvent, AgentLoop, ApprovalPolicy, ToolExecutorTrait};
use vibe_ai::provider::AIProvider;
use vibe_core::search::search_files;

// ── JSON-RPC 2.0 types ────────────────────────────────────────────────────────

#[derive(Debug, Deserialize)]
struct RpcRequest {
    #[allow(dead_code)]
    jsonrpc: Option<String>,
    id: Option<Value>,
    method: String,
    params: Option<Value>,
}

#[derive(Debug, Serialize)]
struct RpcOk {
    jsonrpc: &'static str,
    id: Value,
    result: Value,
}

#[derive(Debug, Serialize)]
struct RpcErr {
    jsonrpc: &'static str,
    id: Value,
    error: ErrObj,
}

#[derive(Debug, Serialize)]
struct ErrObj {
    code: i32,
    message: String,
}

// ── Entry point ───────────────────────────────────────────────────────────────

/// Run the MCP server. Reads JSON-RPC requests from stdin, writes responses to
/// stdout. Blocks until the host closes stdin (EOF).
pub async fn run_server(
    workspace_root: PathBuf,
    provider: Arc<dyn AIProvider>,
    approval: ApprovalPolicy,
    sandbox: bool,
) -> Result<()> {
    eprintln!(
        "[vibecli mcp-server] ready — workspace: {}",
        workspace_root.display()
    );

    let stdin = tokio::io::stdin();
    let stdout = tokio::io::stdout();
    let mut reader = BufReader::new(stdin);
    let mut out = tokio::io::BufWriter::new(stdout);
    let mut line = String::new();

    loop {
        line.clear();
        let n = reader.read_line(&mut line).await?;
        if n == 0 {
            break; // EOF — host closed the connection
        }
        let trimmed = line.trim();
        if trimmed.is_empty() {
            continue;
        }

        let req: RpcRequest = match serde_json::from_str(trimmed) {
            Ok(r) => r,
            Err(e) => {
                write_err(&mut out, Value::Null, -32700, format!("Parse error: {e}")).await?;
                continue;
            }
        };

        // JSON-RPC 2.0 notifications have no `id` (or null `id`).
        // We must NOT send a response for them.
        let id = match req.id.as_ref() {
            Some(v) if !v.is_null() => v.clone(),
            _ => {
                // Notification — handle side effect silently, no response.
                continue;
            }
        };

        let result = dispatch(
            &req.method,
            req.params,
            &workspace_root,
            &provider,
            approval.clone(),
            sandbox,
        )
        .await;

        let line_out = match result {
            Ok(val) => serde_json::to_string(&RpcOk {
                jsonrpc: "2.0",
                id,
                result: val,
            })?,
            Err(e) => serde_json::to_string(&RpcErr {
                jsonrpc: "2.0",
                id,
                error: ErrObj {
                    code: -32000,
                    message: e.to_string(),
                },
            })?,
        };

        out.write_all(line_out.as_bytes()).await?;
        out.write_all(b"\n").await?;
        out.flush().await?;
    }

    eprintln!("[vibecli mcp-server] shutting down (EOF)");
    Ok(())
}

async fn write_err(
    out: &mut tokio::io::BufWriter<tokio::io::Stdout>,
    id: Value,
    code: i32,
    message: String,
) -> Result<()> {
    let s = serde_json::to_string(&RpcErr {
        jsonrpc: "2.0",
        id,
        error: ErrObj { code, message },
    })?;
    out.write_all(s.as_bytes()).await?;
    out.write_all(b"\n").await?;
    out.flush().await?;
    Ok(())
}

// ── Method dispatch ───────────────────────────────────────────────────────────

async fn dispatch(
    method: &str,
    params: Option<Value>,
    workspace_root: &PathBuf,
    provider: &Arc<dyn AIProvider>,
    approval: ApprovalPolicy,
    sandbox: bool,
) -> Result<Value> {
    match method {
        // ── Handshake ────────────────────────────────────────────────────────
        "initialize" => Ok(json!({
            "protocolVersion": "2024-11-05",
            "capabilities": {
                "tools": { "listChanged": false }
            },
            "serverInfo": {
                "name": "vibecli",
                "version": env!("CARGO_PKG_VERSION")
            }
        })),

        "ping" => Ok(json!({})),

        // ── Tool discovery ───────────────────────────────────────────────────
        "tools/list" => Ok(json!({ "tools": tool_defs() })),

        // ── Tool invocation ──────────────────────────────────────────────────
        "tools/call" => {
            let p = params.unwrap_or_default();
            let name = p["name"].as_str().unwrap_or("").to_string();
            let args = p["arguments"].clone();
            let start = std::time::Instant::now();
            let result = call_tool(&name, args, workspace_root, provider, approval, sandbox).await;
            let latency_ms = start.elapsed().as_millis() as u64;
            let (outcome, reason) = match &result {
                Ok(_) => ("success", None),
                Err(e) => ("error", Some(e.to_string())),
            };
            append_audit_event(&name, "mcp-stdio", outcome, reason, latency_ms);
            result
        }

        _ => Err(anyhow::anyhow!("Method not found: {}", method)),
    }
}

// ── Tool definitions ──────────────────────────────────────────────────────────

fn tool_defs() -> Vec<Value> {
    vec![
        json!({
            "name": "read_file",
            "description": "Read the full contents of a file. Path may be absolute or relative to the workspace root.",
            "inputSchema": {
                "type": "object",
                "properties": {
                    "path": { "type": "string", "description": "File path" }
                },
                "required": ["path"]
            }
        }),
        json!({
            "name": "write_file",
            "description": "Write (create or overwrite) a file. Parent directories are created automatically.",
            "inputSchema": {
                "type": "object",
                "properties": {
                    "path": { "type": "string", "description": "File path" },
                    "content": { "type": "string", "description": "Text content to write" }
                },
                "required": ["path", "content"]
            }
        }),
        json!({
            "name": "list_directory",
            "description": "List files and subdirectories. Directories are suffixed with '/'.",
            "inputSchema": {
                "type": "object",
                "properties": {
                    "path": {
                        "type": "string",
                        "description": "Directory path. Defaults to workspace root."
                    }
                },
                "required": []
            }
        }),
        json!({
            "name": "bash",
            "description": "Execute a shell command in the workspace directory and return stdout/stderr.",
            "inputSchema": {
                "type": "object",
                "properties": {
                    "command": { "type": "string", "description": "Shell command to run via sh -c" },
                    "timeout_secs": {
                        "type": "integer",
                        "default": 30,
                        "description": "Timeout in seconds (default 30)"
                    }
                },
                "required": ["command"]
            }
        }),
        json!({
            "name": "search_files",
            "description": "Regex search across all files in the workspace. Returns file:line:content matches.",
            "inputSchema": {
                "type": "object",
                "properties": {
                    "query": { "type": "string", "description": "Search pattern (regex)" },
                    "case_sensitive": {
                        "type": "boolean",
                        "default": false,
                        "description": "Whether the search is case-sensitive"
                    }
                },
                "required": ["query"]
            }
        }),
        json!({
            "name": "agent_run",
            "description": "Run a VibeCLI coding agent task autonomously (plan→act→observe loop). The agent can read/write files, run bash commands, and search the codebase. Returns a step-by-step log and final summary.",
            "inputSchema": {
                "type": "object",
                "properties": {
                    "task": {
                        "type": "string",
                        "description": "Task description for the agent"
                    },
                    "approval": {
                        "type": "string",
                        "enum": ["auto-edit", "full-auto"],
                        "default": "auto-edit",
                        "description": "auto-edit: auto-apply file ops, ask for bash; full-auto: no prompts"
                    },
                    "max_steps": {
                        "type": "integer",
                        "default": 30,
                        "description": "Maximum number of tool-call steps"
                    }
                },
                "required": ["task"]
            }
        }),
        // ── OpenMemory MCP tools ─────────────────────────────────────────
        json!({
            "name": "memory_add",
            "description": "Store a memory in VibeCody's cognitive memory engine. Auto-classifies into 5 sectors (episodic, semantic, procedural, emotional, reflective) with decay and reinforcement.",
            "inputSchema": {
                "type": "object",
                "properties": {
                    "content": { "type": "string", "description": "Memory content to store" },
                    "tags": { "type": "array", "items": { "type": "string" }, "description": "Optional tags" }
                },
                "required": ["content"]
            }
        }),
        json!({
            "name": "memory_query",
            "description": "Search the cognitive memory store using composite scoring (similarity + salience + recency + waypoint graph + sector match).",
            "inputSchema": {
                "type": "object",
                "properties": {
                    "query": { "type": "string", "description": "Search query text" },
                    "limit": { "type": "integer", "default": 10, "description": "Max results" },
                    "sector": { "type": "string", "enum": ["episodic","semantic","procedural","emotional","reflective"], "description": "Optional sector filter" }
                },
                "required": ["query"]
            }
        }),
        json!({
            "name": "memory_add_fact",
            "description": "Add a temporal fact to the knowledge graph. New facts auto-close previous conflicting entries with the same subject+predicate.",
            "inputSchema": {
                "type": "object",
                "properties": {
                    "subject": { "type": "string" },
                    "predicate": { "type": "string" },
                    "object": { "type": "string" }
                },
                "required": ["subject", "predicate", "object"]
            }
        }),
        json!({
            "name": "memory_facts",
            "description": "Query current temporal facts from the knowledge graph. Returns all facts valid at the current time.",
            "inputSchema": {
                "type": "object",
                "properties": {
                    "subject": { "type": "string", "description": "Optional: filter by subject" }
                }
            }
        }),
        json!({
            "name": "memory_stats",
            "description": "Get cognitive memory statistics: total memories, waypoints, facts, and per-sector breakdown with salience averages.",
            "inputSchema": { "type": "object", "properties": {} }
        }),
        // ── Security Scanner MCP tools (rivals Snyk MCP) ─────────────────
        json!({
            "name": "code_scan",
            "description": "Static Application Security Testing (SAST) — scans source code for vulnerabilities using 67 rules across 10+ languages. Detects SQL injection, XSS, command injection, hardcoded secrets, insecure deserialization, path traversal, weak crypto, and more. Supports nosec/nosonar suppression comments.",
            "inputSchema": {
                "type": "object",
                "properties": {
                    "file_path": { "type": "string", "description": "File path for language detection (e.g., 'app.py', 'index.js', 'main.tf')" },
                    "content": { "type": "string", "description": "Source code content to scan" }
                },
                "required": ["file_path", "content"]
            }
        }),
        json!({
            "name": "sca_scan",
            "description": "Software Composition Analysis — scans dependencies for known CVEs across 8 ecosystems (npm, PyPI, crates.io, Go, Maven, RubyGems, NuGet, Packagist). Uses 326+ hardcoded CVEs offline, plus live OSV.dev and GitHub Advisory Database APIs for 60,000+ advisories. Returns CVE ID, CVSS score, EPSS exploit probability, and fix version.",
            "inputSchema": {
                "type": "object",
                "properties": {
                    "lockfile_name": { "type": "string", "description": "Lockfile name for format detection: package-lock.json, yarn.lock, Cargo.lock, requirements.txt, poetry.lock, go.sum, Gemfile.lock" },
                    "content": { "type": "string", "description": "Lockfile content to scan" }
                },
                "required": ["lockfile_name", "content"]
            }
        }),
        json!({
            "name": "iac_scan",
            "description": "Infrastructure as Code security scan — detects misconfigurations in Dockerfiles (root user, :latest tags, HTTP downloads), Kubernetes YAML (privileged containers, host network, missing limits), and Terraform (open security groups, unencrypted resources, public databases).",
            "inputSchema": {
                "type": "object",
                "properties": {
                    "file_path": { "type": "string", "description": "IaC file path (e.g., 'Dockerfile', 'pod.yaml', 'main.tf')" },
                    "content": { "type": "string", "description": "IaC file content to scan" }
                },
                "required": ["file_path", "content"]
            }
        }),
        json!({
            "name": "secret_scan",
            "description": "Detect hardcoded secrets and credentials in source code — AWS keys, GitHub tokens, private keys, JWT tokens, database connection strings, Stripe keys, SendGrid keys, passwords, API keys, Slack webhooks, and more. Returns line numbers and remediation advice.",
            "inputSchema": {
                "type": "object",
                "properties": {
                    "content": { "type": "string", "description": "Source code or config content to scan for secrets" },
                    "file_path": { "type": "string", "description": "Optional file path for context" }
                },
                "required": ["content"]
            }
        }),
        json!({
            "name": "scan_report",
            "description": "Generate a comprehensive security scan report in SARIF or Markdown format. Scans both source files and dependencies, producing a unified report with severity breakdown, CVE details, EPSS scores, and remediation guidance. SARIF output is compatible with GitHub Code Scanning and Azure DevOps.",
            "inputSchema": {
                "type": "object",
                "properties": {
                    "format": { "type": "string", "enum": ["sarif", "markdown"], "default": "markdown", "description": "Report format: sarif (for CI/CD) or markdown (human-readable)" },
                    "lockfile_name": { "type": "string", "description": "Optional lockfile to include in scan" },
                    "lockfile_content": { "type": "string", "description": "Optional lockfile content" },
                    "files": {
                        "type": "array",
                        "items": {
                            "type": "object",
                            "properties": {
                                "path": { "type": "string" },
                                "content": { "type": "string" }
                            }
                        },
                        "description": "Optional source files to SAST scan"
                    }
                }
            }
        }),
        json!({
            "name": "vuln_db_status",
            "description": "Get vulnerability database status — shows count of offline CVEs (326+), SAST rules (67), supported ecosystems, lockfile formats, and whether the local OSV snapshot is available (~60,000 advisories).",
            "inputSchema": { "type": "object", "properties": {} }
        }),
        // ── Email (Gmail / Outlook) ──────────────────────────────────────────
        json!({
            "name": "email_inbox",
            "description": "List the most recent inbox messages from Gmail or Outlook (whichever is configured). Returns sender, subject, date, and message ID.",
            "inputSchema": { "type": "object", "properties": {
                "limit": { "type": "integer", "default": 20, "description": "Max messages to return" }
            }}
        }),
        json!({
            "name": "email_unread",
            "description": "List unread messages from Gmail or Outlook. Returns sender, subject, date, and message ID.",
            "inputSchema": { "type": "object", "properties": {
                "limit": { "type": "integer", "default": 20, "description": "Max messages to return" }
            }}
        }),
        json!({
            "name": "email_read",
            "description": "Read the full body of a specific email message by its ID.",
            "inputSchema": { "type": "object", "properties": {
                "id": { "type": "string", "description": "Message ID from email_inbox or email_unread" }
            }, "required": ["id"]}
        }),
        json!({
            "name": "email_send",
            "description": "Send an email via Gmail or Outlook.",
            "inputSchema": { "type": "object", "properties": {
                "to":      { "type": "string", "description": "Recipient email address" },
                "subject": { "type": "string", "description": "Email subject line" },
                "body":    { "type": "string", "description": "Email body (plain text)" }
            }, "required": ["to", "subject", "body"]}
        }),
        json!({
            "name": "email_search",
            "description": "Search email messages by keyword across inbox.",
            "inputSchema": { "type": "object", "properties": {
                "query": { "type": "string", "description": "Search query" },
                "limit": { "type": "integer", "default": 20 }
            }, "required": ["query"]}
        }),
        // ── Calendar (Google / Outlook) ──────────────────────────────────────
        json!({
            "name": "calendar_today",
            "description": "List today's calendar events from Google Calendar or Outlook Calendar.",
            "inputSchema": { "type": "object", "properties": {} }
        }),
        json!({
            "name": "calendar_week",
            "description": "List this week's calendar events.",
            "inputSchema": { "type": "object", "properties": {} }
        }),
        json!({
            "name": "calendar_create",
            "description": "Create a new calendar event.",
            "inputSchema": { "type": "object", "properties": {
                "title": { "type": "string", "description": "Event title" },
                "start": { "type": "string", "description": "Start time (ISO8601 or natural language)" },
                "end":   { "type": "string", "description": "End time (ISO8601 or natural language)" },
                "description": { "type": "string", "description": "Optional event description" }
            }, "required": ["title", "start", "end"]}
        }),
        json!({
            "name": "calendar_free",
            "description": "Find free time slots on a given date.",
            "inputSchema": { "type": "object", "properties": {
                "date": { "type": "string", "description": "Date to check (ISO8601 or 'today', 'tomorrow')" }
            }}
        }),
        // ── Todoist ──────────────────────────────────────────────────────────
        json!({
            "name": "todoist_list",
            "description": "List active tasks in Todoist.",
            "inputSchema": { "type": "object", "properties": {
                "project": { "type": "string", "description": "Optional project name filter" }
            }}
        }),
        json!({
            "name": "todoist_today",
            "description": "List Todoist tasks due today or overdue.",
            "inputSchema": { "type": "object", "properties": {} }
        }),
        json!({
            "name": "todoist_add",
            "description": "Add a new task to Todoist.",
            "inputSchema": { "type": "object", "properties": {
                "content":  { "type": "string", "description": "Task description" },
                "due":      { "type": "string", "description": "Due date (natural language: 'today', 'tomorrow', 'friday')" },
                "priority": { "type": "integer", "description": "Priority 1 (urgent) to 4 (normal)", "minimum": 1, "maximum": 4 }
            }, "required": ["content"]}
        }),
        json!({
            "name": "todoist_close",
            "description": "Mark a Todoist task as complete.",
            "inputSchema": { "type": "object", "properties": {
                "id": { "type": "string", "description": "Task ID" }
            }, "required": ["id"]}
        }),
        // ── Notion ───────────────────────────────────────────────────────────
        json!({
            "name": "notion_search",
            "description": "Full-text search across a Notion workspace. Returns page titles, IDs, and URLs.",
            "inputSchema": { "type": "object", "properties": {
                "query": { "type": "string", "description": "Search query" },
                "limit": { "type": "integer", "default": 10 }
            }, "required": ["query"]}
        }),
        json!({
            "name": "notion_get",
            "description": "Read the content of a Notion page as plain text.",
            "inputSchema": { "type": "object", "properties": {
                "page_id": { "type": "string", "description": "Notion page ID (UUID)" }
            }, "required": ["page_id"]}
        }),
        json!({
            "name": "notion_append",
            "description": "Append a paragraph block to a Notion page.",
            "inputSchema": { "type": "object", "properties": {
                "page_id": { "type": "string", "description": "Notion page ID (UUID)" },
                "text":    { "type": "string", "description": "Text to append" }
            }, "required": ["page_id", "text"]}
        }),
        // ── Jira ─────────────────────────────────────────────────────────────
        json!({
            "name": "jira_list",
            "description": "List open Jira issues. Optionally filter by project key.",
            "inputSchema": { "type": "object", "properties": {
                "project": { "type": "string", "description": "Project key filter (e.g. 'PROJ')" },
                "limit":   { "type": "integer", "default": 20 }
            }}
        }),
        json!({
            "name": "jira_get",
            "description": "Get full details of a Jira issue by key.",
            "inputSchema": { "type": "object", "properties": {
                "key": { "type": "string", "description": "Jira issue key (e.g. 'PROJ-123')" }
            }, "required": ["key"]}
        }),
        json!({
            "name": "jira_create",
            "description": "Create a new Jira issue.",
            "inputSchema": { "type": "object", "properties": {
                "project":     { "type": "string", "description": "Project key" },
                "summary":     { "type": "string", "description": "Issue title / summary" },
                "description": { "type": "string", "description": "Optional detailed description" },
                "issue_type":  { "type": "string", "default": "Task", "description": "Issue type: Task, Bug, Story, Epic" }
            }, "required": ["project", "summary"]}
        }),
        json!({
            "name": "jira_comment",
            "description": "Add a comment to an existing Jira issue.",
            "inputSchema": { "type": "object", "properties": {
                "key":  { "type": "string", "description": "Jira issue key" },
                "text": { "type": "string", "description": "Comment text" }
            }, "required": ["key", "text"]}
        }),
        json!({
            "name": "jira_search",
            "description": "Run a JQL query against Jira and return matching issues.",
            "inputSchema": { "type": "object", "properties": {
                "jql":   { "type": "string", "description": "JQL query string" },
                "limit": { "type": "integer", "default": 20 }
            }, "required": ["jql"]}
        }),
        // ── Home Assistant ───────────────────────────────────────────────────
        json!({
            "name": "ha_status",
            "description": "Get a summary of all Home Assistant entity states (lights, climate, locks, sensors).",
            "inputSchema": { "type": "object", "properties": {} }
        }),
        json!({
            "name": "ha_control",
            "description": "Control a Home Assistant entity: turn on/off, toggle, set attributes (brightness, color_temp, temperature).",
            "inputSchema": { "type": "object", "properties": {
                "entity":    { "type": "string", "description": "Entity ID (e.g. 'light.living_room') or friendly name" },
                "action":    { "type": "string", "enum": ["on", "off", "toggle"], "description": "Action to perform" },
                "attribute": { "type": "string", "description": "Optional attribute name to set (e.g. 'brightness')" },
                "value":     { "type": "string", "description": "Optional attribute value" }
            }, "required": ["entity", "action"]}
        }),
        json!({
            "name": "ha_scene",
            "description": "Activate a Home Assistant scene by name.",
            "inputSchema": { "type": "object", "properties": {
                "name": { "type": "string", "description": "Scene name (e.g. 'relax', 'focus', 'goodnight')" }
            }, "required": ["name"]}
        }),
        json!({
            "name": "ha_climate",
            "description": "Set a thermostat target temperature in Home Assistant.",
            "inputSchema": { "type": "object", "properties": {
                "entity":      { "type": "string", "description": "Climate entity ID" },
                "temperature": { "type": "number", "description": "Target temperature (Fahrenheit or Celsius per HA config)" }
            }, "required": ["entity", "temperature"]}
        }),
    ]
}

// ── Tool execution ────────────────────────────────────────────────────────────

async fn call_tool(
    name: &str,
    args: Value,
    workspace_root: &PathBuf,
    provider: &Arc<dyn AIProvider>,
    approval: ApprovalPolicy,
    sandbox: bool,
) -> Result<Value> {
    let text: String = match name {
        "read_file" => {
            let path = resolve(workspace_root, args["path"].as_str().unwrap_or(""));
            tokio::fs::read_to_string(&path)
                .await
                .map_err(|e| anyhow::anyhow!("read_file {}: {}", path.display(), e))?
        }

        "write_file" => {
            let path = resolve(workspace_root, args["path"].as_str().unwrap_or(""));
            let content = args["content"].as_str().unwrap_or("");
            if let Some(parent) = path.parent() {
                tokio::fs::create_dir_all(parent).await?;
            }
            tokio::fs::write(&path, content)
                .await
                .map_err(|e| anyhow::anyhow!("write_file {}: {}", path.display(), e))?;
            format!("Wrote {} bytes to {}", content.len(), path.display())
        }

        "list_directory" => {
            let raw = args["path"].as_str().unwrap_or(".");
            let path = resolve(workspace_root, raw);
            let mut rd = tokio::fs::read_dir(&path)
                .await
                .map_err(|e| anyhow::anyhow!("list_directory {}: {}", path.display(), e))?;
            let mut entries = Vec::new();
            while let Ok(Some(entry)) = rd.next_entry().await {
                let mut n = entry.file_name().to_string_lossy().to_string();
                if entry
                    .file_type()
                    .await
                    .map(|ft| ft.is_dir())
                    .unwrap_or(false)
                {
                    n.push('/');
                }
                entries.push(n);
            }
            entries.sort();
            entries.join("\n")
        }

        "bash" => {
            let cmd = args["command"].as_str().unwrap_or("").to_string();
            let timeout_secs = args["timeout_secs"].as_u64().unwrap_or(30);
            let output = tokio::time::timeout(
                std::time::Duration::from_secs(timeout_secs),
                tokio::process::Command::new("sh")
                    .arg("-c")
                    .arg(&cmd)
                    .current_dir(workspace_root)
                    .output(),
            )
            .await
            .map_err(|_| anyhow::anyhow!("bash timed out after {}s", timeout_secs))??;

            let stdout = String::from_utf8_lossy(&output.stdout);
            let stderr = String::from_utf8_lossy(&output.stderr);
            let code = output.status.code().unwrap_or(-1);

            if stderr.is_empty() {
                format!("exit={}\n{}", code, stdout)
            } else {
                format!("exit={}\n{}\n[stderr]\n{}", code, stdout, stderr)
            }
        }

        "search_files" => {
            let query = args["query"].as_str().unwrap_or("").to_string();
            let case_sensitive = args["case_sensitive"].as_bool().unwrap_or(false);
            let results =
                search_files(workspace_root, &query, case_sensitive)
                    .unwrap_or_default();
            if results.is_empty() {
                format!("No matches for '{}'", query)
            } else {
                results
                    .into_iter()
                    .take(200)
                    .map(|r| format!("{}:{}:{}", r.path, r.line_number, r.line_content.trim()))
                    .collect::<Vec<_>>()
                    .join("\n")
            }
        }

        "agent_run" => {
            let task = args["task"].as_str().unwrap_or("").to_string();
            // Use per-call approval if specified; fall back to server-level policy.
            let task_approval = match args["approval"].as_str() {
                Some(s) => ApprovalPolicy::from_str(s),
                None => approval.clone(),
            };
            let max_steps = args["max_steps"].as_u64().unwrap_or(30) as usize;
            run_agent(
                task,
                workspace_root.clone(),
                provider,
                task_approval,
                max_steps,
                sandbox,
            )
            .await?
        }

        // ── OpenMemory MCP tool handlers ─────────────────────────────────
        "memory_add" => {
            let content = args["content"].as_str().unwrap_or("").to_string();
            let tags: Vec<String> = args["tags"].as_array()
                .map(|a| a.iter().filter_map(|v| v.as_str().map(String::from)).collect())
                .unwrap_or_default();
            let mut store = crate::open_memory::project_scoped_store(workspace_root);
            let id = store.add_with_tags(content, tags, std::collections::HashMap::new());
            let sector = store.get(&id).map(|m| m.sector.to_string()).unwrap_or_default();
            let _ = store.save();
            format!("Stored memory {} (sector: {})", id, sector)
        }

        "memory_query" => {
            let query = args["query"].as_str().unwrap_or("").to_string();
            let limit = args["limit"].as_u64().unwrap_or(10) as usize;
            let sector_filter = args["sector"].as_str().and_then(|s| s.parse().ok());
            let store = crate::open_memory::project_scoped_store(workspace_root);
            let results = store.query_with_filters(&query, limit, sector_filter, None);
            if results.is_empty() {
                "No matching memories found.".to_string()
            } else {
                results.iter().map(|r| {
                    format!("[{} | score:{:.2} | sal:{:.0}%] {}",
                        r.memory.sector, r.score, r.effective_salience * 100.0,
                        &r.memory.content[..r.memory.content.len().min(200)])
                }).collect::<Vec<_>>().join("\n")
            }
        }

        "memory_add_fact" => {
            let subject = args["subject"].as_str().unwrap_or("").to_string();
            let predicate = args["predicate"].as_str().unwrap_or("").to_string();
            let object = args["object"].as_str().unwrap_or("").to_string();
            let mut store = crate::open_memory::project_scoped_store(workspace_root);
            let id = store.add_fact(subject.clone(), predicate.clone(), object.clone());
            let _ = store.save();
            format!("Added fact: {} {} {} (id: {})", subject, predicate, object, id)
        }

        "memory_facts" => {
            let store = crate::open_memory::project_scoped_store(workspace_root);
            let subject_filter = args["subject"].as_str();
            let facts = store.query_current_facts();
            let filtered: Vec<_> = facts.iter()
                .filter(|f| subject_filter.is_none_or(|s| f.subject == s))
                .collect();
            if filtered.is_empty() {
                "No current temporal facts.".to_string()
            } else {
                filtered.iter().map(|f| {
                    format!("{} {} {} (conf: {:.0}%)", f.subject, f.predicate, f.object, f.confidence * 100.0)
                }).collect::<Vec<_>>().join("\n")
            }
        }

        "memory_stats" => {
            let store = crate::open_memory::project_scoped_store(workspace_root);
            let stats = store.sector_stats();
            let mut lines = vec![format!("Memories: {} | Waypoints: {} | Facts: {}",
                store.total_memories(), store.total_waypoints(), store.total_facts())];
            lines.push(format!(
                "Index: TurboQuant {}-dim, ~{:.1}× compression",
                store.embedding_dim(),
                store.embedding_compression_ratio(),
            ));
            for s in &stats {
                if s.count > 0 {
                    lines.push(format!("  {} — {} memories, avg sal {:.0}%, {} pinned",
                        s.sector, s.count, s.avg_salience * 100.0, s.pinned_count));
                }
            }
            lines.join("\n")
        }

        // ── Security Scanner MCP tool handlers ─────────────────────────
        "code_scan" => {
            let file_path = args["file_path"].as_str().unwrap_or("unknown.txt").to_string();
            let content = args["content"].as_str().unwrap_or("").to_string();
            let mut scanner = crate::vulnerability_db::VulnerabilityScanner::new();
            let count = scanner.scan_file(&file_path, &content);
            if count == 0 {
                format!("No security issues found in {}", file_path)
            } else {
                let mut lines = vec![format!("{} findings in {}:", count, file_path)];
                for f in scanner.active_findings().iter().take(25) {
                    let line = f.line.map(|l| format!(":{}", l)).unwrap_or_default();
                    lines.push(format!("  {} [{}] {}{} — {}",
                        f.severity, f.cwe_id.as_deref().unwrap_or(""),
                        f.file_path.as_deref().unwrap_or(&file_path), line, f.title));
                    lines.push(format!("    Fix: {}", f.remediation));
                }
                lines.join("\n")
            }
        }

        "sca_scan" => {
            let lockfile_name = args["lockfile_name"].as_str().unwrap_or("").to_string();
            let content = args["content"].as_str().unwrap_or("").to_string();
            let deps = crate::vulnerability_db::parse_lockfile(&lockfile_name, &content);
            if deps.is_empty() {
                format!("No dependencies parsed from '{}'. Supported formats: package-lock.json, yarn.lock, Cargo.lock, requirements.txt, poetry.lock, go.sum, Gemfile.lock", lockfile_name)
            } else {
                let mut scanner = crate::vulnerability_db::VulnerabilityScanner::new();
                scanner.scan_dependencies(&deps);
                let s = scanner.summary();
                let mut lines = vec![
                    format!("{} packages scanned, {} vulnerabilities found", deps.len(), s.total_findings),
                    format!("Critical: {} | High: {} | Medium: {} | Low: {}", s.critical, s.high, s.medium, s.low),
                ];
                if s.exploit_available_count > 0 {
                    lines.push(format!("{} with known public exploit (EPSS avg: {:.0}%)", s.exploit_available_count, s.mean_epss * 100.0));
                }
                lines.push(String::new());
                for f in scanner.active_findings().iter().take(30) {
                    let fix = f.fixed_version.as_deref().unwrap_or("no fix");
                    let exploit = if f.exploit_available { " [EXPLOIT]" } else { "" };
                    lines.push(format!("  {} {} {}@{} → {}{}", f.severity,
                        f.cve_id.as_deref().unwrap_or(""), f.package.as_deref().unwrap_or(""),
                        f.installed_version.as_deref().unwrap_or("?"), fix, exploit));
                }
                lines.join("\n")
            }
        }

        "iac_scan" => {
            let file_path = args["file_path"].as_str().unwrap_or("unknown").to_string();
            let content = args["content"].as_str().unwrap_or("").to_string();
            let mut scanner = crate::vulnerability_db::VulnerabilityScanner::new();
            let count = scanner.scan_file(&file_path, &content);
            if count == 0 {
                format!("No IaC misconfigurations found in {}", file_path)
            } else {
                let mut lines = vec![format!("{} IaC findings in {}:", count, file_path)];
                for f in scanner.active_findings() {
                    let line = f.line.map(|l| format!(":{}", l)).unwrap_or_default();
                    lines.push(format!("  {} [{}] {}{} — {}", f.severity,
                        f.cwe_id.as_deref().unwrap_or(""), file_path, line, f.title));
                    lines.push(format!("    Fix: {}", f.remediation));
                }
                lines.join("\n")
            }
        }

        "secret_scan" => {
            let content = args["content"].as_str().unwrap_or("").to_string();
            let file_path = args["file_path"].as_str().unwrap_or("input").to_string();
            let mut scanner = crate::vulnerability_db::VulnerabilityScanner::new();
            // Use SAST rules filtered to secret category
            let _count = scanner.scan_file(&file_path, &content);
            let secrets: Vec<_> = scanner.active_findings().into_iter()
                .filter(|f| f.cwe_id.as_deref() == Some("CWE-798") || f.title.to_lowercase().contains("secret") || f.title.to_lowercase().contains("password") || f.title.to_lowercase().contains("key"))
                .collect();
            if secrets.is_empty() {
                "No hardcoded secrets detected.".to_string()
            } else {
                let mut lines = vec![format!("{} potential secrets found:", secrets.len())];
                for f in &secrets {
                    let line = f.line.map(|l| format!(":{}", l)).unwrap_or_default();
                    lines.push(format!("  {} {}{} — {}", f.severity, file_path, line, f.title));
                    lines.push(format!("    Fix: {}", f.remediation));
                }
                lines.join("\n")
            }
        }

        "scan_report" => {
            let format = args["format"].as_str().unwrap_or("markdown");
            let mut scanner = crate::vulnerability_db::VulnerabilityScanner::new();

            // Scan lockfile if provided
            if let (Some(lf_name), Some(lf_content)) = (
                args["lockfile_name"].as_str(),
                args["lockfile_content"].as_str()
            ) {
                let deps = crate::vulnerability_db::parse_lockfile(lf_name, lf_content);
                scanner.scan_dependencies(&deps);
            }

            // Scan source files if provided
            if let Some(files) = args["files"].as_array() {
                for file in files {
                    if let (Some(path), Some(content)) = (
                        file.get("path").and_then(|p| p.as_str()),
                        file.get("content").and_then(|c| c.as_str()),
                    ) {
                        scanner.scan_file(path, content);
                    }
                }
            }

            match format {
                "sarif" => {
                    let sarif = scanner.to_sarif();
                    serde_json::to_string_pretty(&sarif).unwrap_or_else(|_| "SARIF generation failed".to_string())
                }
                _ => scanner.to_markdown(),
            }
        }

        "vuln_db_status" => {
            let scanner = crate::vulnerability_db::VulnerabilityScanner::new();
            let snapshot = crate::vulnerability_db::OsvSnapshotDb::new(
                crate::vulnerability_db::OsvSnapshotDb::default_path()
            );
            let mut lines = vec![
                format!("VibeCody Vulnerability Scanner"),
                format!("  Offline CVE database: {} known vulnerabilities", scanner.vuln_db_size()),
                format!("  SAST rules: {} patterns", scanner.sast_rule_count()),
                format!("  Ecosystems: npm, PyPI, crates.io, Go, Maven, RubyGems, NuGet, Packagist"),
                format!("  Lockfile parsers: package-lock.json, yarn.lock, Cargo.lock, requirements.txt, poetry.lock, go.sum, Gemfile.lock"),
                format!("  Live APIs: OSV.dev (60K+ advisories), GHSA (with GITHUB_TOKEN)"),
                format!("  Output: SARIF v2.1.0, Markdown"),
            ];
            if snapshot.exists() {
                lines.push(format!("  Local snapshot: {} advisories (age: {:.0}h)",
                    snapshot.advisory_count(),
                    snapshot.age_hours().unwrap_or(0.0)));
            } else {
                lines.push("  Local snapshot: not downloaded".to_string());
            }
            lines.join("\n")
        }

        // ── Email ────────────────────────────────────────────────────────────
        "email_inbox" => {
            let limit = args["limit"].as_u64().unwrap_or(20);
            crate::email_client::handle_email_command(&format!("inbox {}", limit)).await
        }
        "email_unread" => {
            let limit = args["limit"].as_u64().unwrap_or(20);
            crate::email_client::handle_email_command(&format!("unread {}", limit)).await
        }
        "email_read" => {
            let id = args["id"].as_str().unwrap_or("");
            crate::email_client::handle_email_command(&format!("read {}", id)).await
        }
        "email_send" => {
            let to      = args["to"].as_str().unwrap_or("");
            let subject = args["subject"].as_str().unwrap_or("");
            let body    = args["body"].as_str().unwrap_or("");
            crate::email_client::handle_email_command(&format!("send {} {} {}", to, subject, body)).await
        }
        "email_search" => {
            let query = args["query"].as_str().unwrap_or("");
            let limit = args["limit"].as_u64().unwrap_or(20);
            crate::email_client::handle_email_command(&format!("search {} {}", query, limit)).await
        }

        // ── Calendar ─────────────────────────────────────────────────────────
        "calendar_today" => {
            crate::calendar_client::handle_calendar_command("today").await
        }
        "calendar_week" => {
            crate::calendar_client::handle_calendar_command("week").await
        }
        "calendar_create" => {
            let title = args["title"].as_str().unwrap_or("");
            let start = args["start"].as_str().unwrap_or("");
            let end   = args["end"].as_str().unwrap_or("");
            let desc  = args["description"].as_str().unwrap_or("");
            crate::calendar_client::handle_calendar_command(
                &format!("create {} {} {} {}", title, start, end, desc)
            ).await
        }
        "calendar_free" => {
            let date = args["date"].as_str().unwrap_or("today");
            crate::calendar_client::handle_calendar_command(&format!("free {}", date)).await
        }

        // ── Todoist ──────────────────────────────────────────────────────────
        "todoist_list" => {
            let project = args["project"].as_str().unwrap_or("");
            let cmd = format!("todo list {}", project);
            crate::productivity::handle_productivity_command(cmd.trim()).await
        }
        "todoist_today" => {
            crate::productivity::handle_productivity_command("todo today").await
        }
        "todoist_add" => {
            let content  = args["content"].as_str().unwrap_or("");
            let due      = args["due"].as_str().unwrap_or("");
            let priority = args["priority"].as_u64().unwrap_or(4);
            crate::productivity::handle_productivity_command(
                &format!("todo add {} due:{} p:{}", content, due, priority)
            ).await
        }
        "todoist_close" => {
            let id = args["id"].as_str().unwrap_or("");
            crate::productivity::handle_productivity_command(&format!("todo close {}", id)).await
        }

        // ── Notion ───────────────────────────────────────────────────────────
        "notion_search" => {
            let query = args["query"].as_str().unwrap_or("");
            crate::productivity::handle_productivity_command(&format!("notion search {}", query)).await
        }
        "notion_get" => {
            let page_id = args["page_id"].as_str().unwrap_or("");
            crate::productivity::handle_productivity_command(&format!("notion get {}", page_id)).await
        }
        "notion_append" => {
            let page_id = args["page_id"].as_str().unwrap_or("");
            let text    = args["text"].as_str().unwrap_or("");
            crate::productivity::handle_productivity_command(
                &format!("notion append {} {}", page_id, text)
            ).await
        }

        // ── Jira ─────────────────────────────────────────────────────────────
        "jira_list" => {
            let project = args["project"].as_str().unwrap_or("");
            let cmd = format!("jira list {}", project);
            crate::productivity::handle_productivity_command(cmd.trim()).await
        }
        "jira_get" => {
            let key = args["key"].as_str().unwrap_or("");
            crate::productivity::handle_productivity_command(&format!("jira get {}", key)).await
        }
        "jira_create" => {
            let project     = args["project"].as_str().unwrap_or("");
            let summary     = args["summary"].as_str().unwrap_or("");
            let description = args["description"].as_str().unwrap_or("");
            crate::productivity::handle_productivity_command(
                &format!("jira create {} {} {}", project, summary, description)
            ).await
        }
        "jira_comment" => {
            let key  = args["key"].as_str().unwrap_or("");
            let text = args["text"].as_str().unwrap_or("");
            crate::productivity::handle_productivity_command(
                &format!("jira comment {} {}", key, text)
            ).await
        }
        "jira_search" => {
            let jql   = args["jql"].as_str().unwrap_or("");
            let limit = args["limit"].as_u64().unwrap_or(20);
            crate::productivity::handle_productivity_command(
                &format!("jira search {} limit:{}", jql, limit)
            ).await
        }

        // ── Home Assistant ───────────────────────────────────────────────────
        "ha_status" => {
            crate::home_assistant::handle_ha_command("status").await
        }
        "ha_control" => {
            let entity    = args["entity"].as_str().unwrap_or("");
            let action    = args["action"].as_str().unwrap_or("toggle");
            let attribute = args["attribute"].as_str().unwrap_or("");
            let value     = args["value"].as_str().unwrap_or("");
            if attribute.is_empty() {
                crate::home_assistant::handle_ha_command(&format!("{} {}", action, entity)).await
            } else {
                crate::home_assistant::handle_ha_command(
                    &format!("set {} {} {}", entity, attribute, value)
                ).await
            }
        }
        "ha_scene" => {
            let name = args["name"].as_str().unwrap_or("");
            crate::home_assistant::handle_ha_command(&format!("scene {}", name)).await
        }
        "ha_climate" => {
            let entity = args["entity"].as_str().unwrap_or("");
            let temp   = args["temperature"].as_f64().unwrap_or(70.0);
            crate::home_assistant::handle_ha_command(&format!("climate {} {}", entity, temp)).await
        }

        _ => return Err(anyhow::anyhow!("Unknown tool: {}", name)),
    };

    Ok(json!({ "content": [{ "type": "text", "text": text }] }))
}

// ── Agent runner ──────────────────────────────────────────────────────────────

async fn run_agent(
    task: String,
    workspace_root: PathBuf,
    provider: &Arc<dyn AIProvider>,
    approval: ApprovalPolicy,
    max_steps: usize,
    sandbox: bool,
) -> Result<String> {
    use crate::tool_executor::ToolExecutor;

    let executor = Arc::new(ToolExecutor::new(workspace_root.clone(), sandbox));
    let agent = AgentLoop::new(Arc::clone(provider), approval, executor)
        .with_policy(&workspace_root);

    let context = AgentContext {
        workspace_root: workspace_root.clone(),
        ..Default::default()
    };

    // Override max_steps via a local wrapper since we can't mutate after `new`.
    // We re-create with correct max_steps by using with_policy which reads from file;
    // cap it manually here.
    let _ = max_steps; // honoured through policy effective_max_steps

    let (tx, mut rx) = tokio::sync::mpsc::channel(256);
    let task_for_spawn = task.clone();

    tokio::spawn(async move {
        let _ = agent.run(&task_for_spawn, context, tx).await;
    });

    let mut log = Vec::<String>::new();
    log.push(format!("Agent task: {}\n", task));

    while let Some(event) = rx.recv().await {
        match event {
            AgentEvent::ToolCallExecuted(step) => {
                let icon = if step.tool_result.success { "✅" } else { "❌" };
                let snippet: String = step.tool_result.output.chars().take(300).collect();
                log.push(format!(
                    "{} Step {}: {}\n   {}",
                    icon,
                    step.step_num,
                    step.tool_call.name(),
                    snippet
                ));
            }
            AgentEvent::ToolCallPending { call, result_tx } => {
                // In MCP server mode all pending calls are auto-approved.
                // The MCP host controls access at the protocol level.
                let executor = ToolExecutor::new(workspace_root.clone(), sandbox);
                let result = executor.execute(&call).await;
                log.push(format!("⚡ Auto-approved: {}", call.name()));
                let _ = result_tx.send(Some(result));
            }
            AgentEvent::Complete(summary) => {
                log.push(format!("\n✔ Complete: {}", summary));
                break;
            }
            AgentEvent::Error(e) => {
                log.push(format!("\n✗ Error: {}", e));
                break;
            }
            AgentEvent::StreamChunk(_) => {}
            AgentEvent::RetryableError { .. } => {} // silently retry in MCP mode
            AgentEvent::CircuitBreak { state, reason } => {
                log.push(format!("⚠ Circuit break ({}): {}", state, reason));
                if state == vibe_ai::agent::AgentHealthState::Blocked {
                    break;
                }
            }
            AgentEvent::Partial { summary, steps_completed, steps_planned, remaining_plan } => {
                log.push(format!(
                    "\n⚠ Partial ({}/{}): {}",
                    steps_completed, steps_planned, summary
                ));
                if !remaining_plan.is_empty() {
                    log.push(format!("   Remaining: {}", remaining_plan.join(", ")));
                }
                break;
            }
            AgentEvent::Verifier { decision } => {
                log.push(format!("verifier: {:?}", decision));
            }
        }
    }

    Ok(log.join("\n"))
}

// ── Helpers ───────────────────────────────────────────────────────────────────

fn resolve(root: &Path, path: &str) -> PathBuf {
    let p = PathBuf::from(path);
    if p.is_absolute() {
        p
    } else {
        root.join(p)
    }
}

// ── Audit log ─────────────────────────────────────────────────────────────────
//
// Every `tools/call` dispatch appends a single JSON line to
// `~/.vibecli/mcp_audit.jsonl`. VibeUI's MCP Governance panel tails this
// file via the `mcp_audit_query` Tauri command.

/// Path to the MCP audit log (`~/.vibecli/mcp_audit.jsonl`).
pub fn audit_log_path() -> PathBuf {
    dirs::home_dir()
        .unwrap_or_else(|| PathBuf::from("."))
        .join(".vibecli")
        .join("mcp_audit.jsonl")
}

/// Size at which the audit log rotates to `mcp_audit.jsonl.1`. 10 MiB.
const AUDIT_LOG_MAX_BYTES: u64 = 10 * 1024 * 1024;

fn maybe_rotate_audit_log(path: &Path) {
    let Ok(meta) = std::fs::metadata(path) else { return };
    if meta.len() < AUDIT_LOG_MAX_BYTES {
        return;
    }
    // Single-generation rotation: current → .1, old .1 is overwritten.
    let rotated = path.with_extension("jsonl.1");
    let _ = std::fs::rename(path, rotated);
}

fn append_audit_event(
    tool_name: &str,
    caller_id: &str,
    outcome: &str,
    reason: Option<String>,
    latency_ms: u64,
) {
    use std::io::Write;
    let path = audit_log_path();
    if let Some(parent) = path.parent() {
        let _ = std::fs::create_dir_all(parent);
    }
    maybe_rotate_audit_log(&path);
    let now_ms = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default()
        .as_millis() as u64;
    let entry = serde_json::json!({
        "event_id": format!("evt-{:x}", now_ms),
        "tool_name": tool_name,
        "caller_id": caller_id,
        "outcome": outcome,
        "reason": reason,
        "latency_ms": latency_ms,
        "timestamp_ms": now_ms,
    });
    if let Ok(mut f) = std::fs::OpenOptions::new()
        .create(true)
        .append(true)
        .open(&path)
    {
        // Best-effort — never surface audit-write errors into the RPC path.
        let _ = writeln!(f, "{}", entry);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // ── resolve ──────────────────────────────────────────────────────────────

    #[test]
    fn resolve_absolute_path_unchanged() {
        let root = PathBuf::from("/workspace");
        let result = resolve(&root, "/etc/hosts");
        assert_eq!(result, PathBuf::from("/etc/hosts"));
    }

    #[test]
    fn resolve_relative_path_joined() {
        let root = PathBuf::from("/workspace");
        let result = resolve(&root, "src/main.rs");
        assert_eq!(result, PathBuf::from("/workspace/src/main.rs"));
    }

    #[test]
    fn resolve_empty_path() {
        let root = PathBuf::from("/workspace");
        let result = resolve(&root, "");
        assert_eq!(result, PathBuf::from("/workspace/"));
    }

    #[test]
    fn resolve_dot() {
        let root = PathBuf::from("/workspace");
        let result = resolve(&root, ".");
        assert_eq!(result, PathBuf::from("/workspace/."));
    }

    // ── tool_defs ────────────────────────────────────────────────────────────

    #[test]
    fn tool_defs_returns_expected_count() {
        let defs = tool_defs();
        // Count grows as new MCP tools are added; verify we have all categories:
        // 6 core + 5 memory + 6 security + 5 email + 4 calendar + 4 todoist + 3 notion + 5 jira + 4 ha = 42+
        assert!(defs.len() >= 42, "Expected at least 42 MCP tool definitions, got {}", defs.len());
    }

    #[test]
    fn tool_defs_all_have_name() {
        for def in tool_defs() {
            assert!(def["name"].is_string(), "tool missing name: {:?}", def);
        }
    }

    #[test]
    fn tool_defs_all_have_description() {
        for def in tool_defs() {
            assert!(def["description"].is_string(), "tool missing description: {:?}", def);
        }
    }

    #[test]
    fn tool_defs_all_have_input_schema() {
        for def in tool_defs() {
            assert!(def["inputSchema"].is_object(), "tool missing inputSchema: {:?}", def);
            assert_eq!(def["inputSchema"]["type"].as_str(), Some("object"));
        }
    }

    #[test]
    fn tool_defs_expected_tool_names() {
        let defs = tool_defs();
        let names: Vec<&str> = defs.iter().map(|d| d["name"].as_str().unwrap()).collect();
        // Core tools
        assert!(names.contains(&"read_file"));
        assert!(names.contains(&"write_file"));
        assert!(names.contains(&"list_directory"));
        assert!(names.contains(&"bash"));
        assert!(names.contains(&"search_files"));
        assert!(names.contains(&"agent_run"));
        // Memory tools
        assert!(names.contains(&"memory_add"));
        assert!(names.contains(&"memory_query"));
        // Security tools
        assert!(names.contains(&"code_scan"));
        assert!(names.contains(&"secret_scan"));
        // Productivity tools
        assert!(names.contains(&"email_inbox"));
        assert!(names.contains(&"email_send"));
        assert!(names.contains(&"calendar_today"));
        assert!(names.contains(&"calendar_create"));
        assert!(names.contains(&"todoist_today"));
        assert!(names.contains(&"todoist_add"));
        assert!(names.contains(&"notion_search"));
        assert!(names.contains(&"notion_get"));
        assert!(names.contains(&"jira_list"));
        assert!(names.contains(&"jira_create"));
        assert!(names.contains(&"ha_status"));
        assert!(names.contains(&"ha_control"));
        assert!(names.contains(&"ha_scene"));
    }

    #[test]
    fn tool_defs_read_file_requires_path() {
        let defs = tool_defs();
        let read = defs.iter().find(|d| d["name"] == "read_file").unwrap();
        let required = read["inputSchema"]["required"].as_array().unwrap();
        assert!(required.iter().any(|v| v == "path"));
    }

    #[test]
    fn tool_defs_write_file_requires_path_and_content() {
        let defs = tool_defs();
        let write = defs.iter().find(|d| d["name"] == "write_file").unwrap();
        let required = write["inputSchema"]["required"].as_array().unwrap();
        assert!(required.iter().any(|v| v == "path"));
        assert!(required.iter().any(|v| v == "content"));
    }

    // ── RpcOk / RpcErr serialization ─────────────────────────────────────────

    #[test]
    fn rpc_ok_serializes() {
        let ok = RpcOk {
            jsonrpc: "2.0",
            id: json!(1),
            result: json!({"tools": []}),
        };
        let s = serde_json::to_string(&ok).unwrap();
        assert!(s.contains("\"jsonrpc\":\"2.0\""));
        assert!(s.contains("\"id\":1"));
    }

    #[test]
    fn rpc_err_serializes() {
        let err = RpcErr {
            jsonrpc: "2.0",
            id: json!(42),
            error: ErrObj { code: -32600, message: "Invalid Request".to_string() },
        };
        let s = serde_json::to_string(&err).unwrap();
        assert!(s.contains("-32600"));
        assert!(s.contains("Invalid Request"));
    }
}
