//! Launch the kodegraph MCP server over a persisted graph.
//!
//! Run: `cargo run --example mcp_server -- path/to/codegraph.db`
//!
//! Then send JSON-RPC frames on stdin, e.g.:
//!   {"jsonrpc":"2.0","id":1,"method":"initialize","params":{}}

use kodegraph::mcp::McpServer;
use kodegraph::store::{SQLiteStore, Store};

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let db = std::env::args()
        .nth(1)
        .unwrap_or_else(|| "kodegraph-out/codegraph.db".to_string());

    let store = SQLiteStore::open(std::path::Path::new(&db))?;
    let graph = store
        .load_graph()?
        .ok_or_else(|| anyhow::anyhow!("no graph at {db}; run `kodegraph build <dir>` first"))?;

    McpServer::new(graph).serve().await
}