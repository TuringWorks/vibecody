//! stdio MCP server — exposes the graph as Model Context Protocol tools so an
//! external agent (Claude Code, Cursor, Zed, …) can query a built graph.
//!
//! Tools (Graphify-compatible names):
//! - `query_graph`     `{ query, budget }`        → focused subgraph
//! - `get_node`        `{ name }`                 → single node payload
//! - `get_neighbors`   `{ name }`                 → adjacent nodes
//! - `shortest_path`   `{ from, to }`             → hop count + path
//! - `blast_radius`    `{ name, max_hops }`       → reachable set by hop
//!
//! The server reads JSON-RPC 2.0 frames from stdin and writes responses to stdout.
//! It is launched with an already-built `CodeGraph` (typically loaded from a store).

use std::sync::Arc;

use anyhow::Result;
use serde_json::{json, Value};
use tokio::io::{AsyncBufReadExt, AsyncReadExt, AsyncWriteExt, BufReader};
use tokio::sync::Mutex;

use crate::model::graph::CodeGraph;
use crate::query;

/// An MCP server over a loaded graph.
pub struct McpServer {
    graph: Arc<Mutex<CodeGraph>>,
}

impl McpServer {
    /// Construct with a loaded graph.
    pub fn new(graph: CodeGraph) -> Self {
        Self { graph: Arc::new(Mutex::new(graph)) }
    }

    /// Run the stdio loop until EOF.
    pub async fn serve(self) -> Result<()> {
        let mut stdin = BufReader::new(tokio::io::stdin());
        let mut stdout = tokio::io::stdout();
        let mut content_len: Option<usize> = None;
        let mut line = String::new();

        loop {
            line.clear();
            let n = stdin.read_line(&mut line).await?;
            if n == 0 {
                return Ok(());
            }
            let trimmed = line.trim_end_matches(&['\r', '\n'][..]);
            if trimmed.is_empty() {
                if let Some(len) = content_len.take() {
                    let mut buf = vec![0u8; len];
                    stdin.read_exact(&mut buf).await?;
                    let frame = String::from_utf8_lossy(&buf);
                    if let Ok(val) = serde_json::from_str::<Value>(&frame) {
                        if let Some(resp) = self.handle(val).await {
                            let body = serde_json::to_string(&resp)?;
                            let frame = format!("Content-Length: {}\r\n\r\n{}", body.len(), body);
                            stdout.write_all(frame.as_bytes()).await?;
                            stdout.flush().await?;
                        }
                    }
                }
                continue;
            }
            if let Some(rest) = trimmed.strip_prefix("Content-Length:") {
                content_len = Some(rest.trim().parse::<usize>()?);
            }
        }
    }

    async fn handle(&self, req: Value) -> Option<Value> {
        let id = req.get("id").cloned();
        let method = req.get("method").and_then(|m| m.as_str())?;
        let params = req.get("params").cloned().unwrap_or(Value::Null);

        let result: Value = match method {
            "initialize" => json!({
                "capabilities": { "tools": {} },
                "serverInfo": { "name": "kodegraph", "version": env!("CARGO_PKG_VERSION") },
                "protocolVersion": "2024-11-05",
            }),
            "tools/list" => json!({ "tools": tools_list() }),
            "tools/call" => {
                let name = params.get("name").and_then(|n| n.as_str()).unwrap_or("");
                let args = params.get("arguments").cloned().unwrap_or(Value::Null);
                let graph = self.graph.lock().await;
                tool_call(&graph, name, args)
            }
            _ => {
                // Unknown method — return a JSON-RPC error if this is a request.
                if let Some(id) = id {
                    return Some(json!({
                        "jsonrpc": "2.0", "id": id,
                        "error": { "code": -32601, "message": "method not found" }
                    }));
                }
                return None;
            }
        };

        // Notifications (no id) get no response.
        let id = id?;
        Some(json!({ "jsonrpc": "2.0", "id": id, "result": result }))
    }
}

fn tools_list() -> Vec<Value> {
    vec![
        json!({ "name": "query_graph", "description": "Pull a focused subgraph matching a query within a token budget.",
                "inputSchema": { "type": "object", "properties": { "query": {"type":"string"}, "budget": {"type":"integer"} }, "required": ["query"] } }),
        json!({ "name": "get_node", "description": "Retrieve a single node's payload by name.",
                "inputSchema": { "type": "object", "properties": { "name": {"type":"string"} }, "required": ["name"] } }),
        json!({ "name": "get_neighbors", "description": "Adjacent nodes (callers + callees) of a named symbol.",
                "inputSchema": { "type": "object", "properties": { "name": {"type":"string"} }, "required": ["name"] } }),
        json!({ "name": "shortest_path", "description": "Shortest path (hop count + nodes) between two named symbols.",
                "inputSchema": { "type": "object", "properties": { "from": {"type":"string"}, "to": {"type":"string"} }, "required": ["from","to"] } }),
        json!({ "name": "blast_radius", "description": "Set of symbols reachable within N hops of a change (both directions).",
                "inputSchema": { "type": "object", "properties": { "name": {"type":"string"}, "max_hops": {"type":"integer"} }, "required": ["name"] } }),
    ]
}

fn tool_call(graph: &CodeGraph, name: &str, args: Value) -> Value {
    match name {
        "query_graph" => {
            let q = args.get("query").and_then(|v| v.as_str()).unwrap_or("");
            let budget = args.get("budget").and_then(|v| v.as_u64()).unwrap_or(2000) as usize;
            let sub = query::query_graph(graph, q, budget);
            json!({
                "content": [{ "type": "text", "text": format_subgraph(&sub) }],
                "estTokens": sub.est_tokens,
            })
        }
        "get_node" => {
            let n = args.get("name").and_then(|v| v.as_str()).unwrap_or("");
            match query::get_node(graph, n) {
                Some(node) => json!({ "content": [{ "type": "text", "text": format!("{:#?}", node) }] }),
                None => json!({ "content": [{ "type": "text", "text": format!("no node named {n}") }], "isError": true }),
            }
        }
        "get_neighbors" => {
            let n = args.get("name").and_then(|v| v.as_str()).unwrap_or("");
            let nbrs = query::get_neighbors(graph, n);
            let labels: Vec<String> = nbrs.iter().map(|n| n.label()).collect();
            json!({ "content": [{ "type": "text", "text": labels.join(", ") }] })
        }
        "shortest_path" => {
            let from = args.get("from").and_then(|v| v.as_str()).unwrap_or("");
            let to = args.get("to").and_then(|v| v.as_str()).unwrap_or("");
            match query::shortest_path(graph, from, to) {
                Some((hops, nodes)) => {
                    let labels: Vec<String> = nodes.iter().map(|n| n.label()).collect();
                    json!({ "content": [{ "type": "text", "text": format!("hops={hops}; {}", labels.join(" -> ")) }] })
                }
                None => json!({ "content": [{ "type": "text", "text": "no path" }], "isError": true }),
            }
        }
        "blast_radius" => {
            let n = args.get("name").and_then(|v| v.as_str()).unwrap_or("");
            let hops = args.get("max_hops").and_then(|v| v.as_u64()).unwrap_or(2) as usize;
            let br = query::blast_radius(graph, n, hops);
            let mut lines = Vec::new();
            for hop in 0..=hops {
                let names: Vec<String> = br
                    .at_hop(hop)
                    .iter()
                    .filter_map(|id| graph.node(*id).map(|n| n.label()))
                    .collect();
                if !names.is_empty() {
                    lines.push(format!("hop {hop}: {}", names.join(", ")));
                }
            }
            json!({ "content": [{ "type": "text", "text": lines.join("\n") }], "affected": br.affected() })
        }
        _ => json!({ "content": [{ "type": "text", "text": format!("unknown tool {name}") }], "isError": true }),
    }
}

/// Pretty-print a [`query::Subgraph`] as text (used by the MCP tools and the CLI).
pub fn format_subgraph(sub: &query::Subgraph) -> String {
    let mut s = String::new();
    for n in &sub.nodes {
        s.push_str(&format!("node: {}\n", n.label()));
    }
    for (from, to, kind, prov) in &sub.edges {
        s.push_str(&format!(
            "{:?} --{:?}--> {:?}  [conf {:.2}]\n",
            from.index(),
            kind,
            to.index(),
            prov.confidence
        ));
    }
    s
}