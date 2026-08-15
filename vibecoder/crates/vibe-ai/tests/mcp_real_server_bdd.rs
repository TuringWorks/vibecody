//! End-to-end check against a real MCP server.
//!
//! `#[ignore]` because it shells out to `npx`, which needs Node and (on a cold
//! cache) the network. It is not skipped conditionally: a test that passes when
//! its dependency is missing reports coverage it does not have, and the whole
//! point of this one is that the unit tests could not have caught the bug —
//! only a real server that speaks out of turn does.
//!
//! Run it with:
//!
//! ```text
//! cargo test -p vibe-ai --test mcp_real_server_bdd -- --ignored --nocapture
//! ```

use std::collections::HashMap;

use vibe_ai::mcp::{McpClient, McpServerConfig};

fn everything_server() -> McpServerConfig {
    McpServerConfig {
        name: "everything".to_string(),
        command: "npx".to_string(),
        args: vec![
            "-y".to_string(),
            "@modelcontextprotocol/server-everything".to_string(),
        ],
        env: HashMap::new(),
    }
}

/// Given a server that sends a notification before answering,
/// When its tools are listed,
/// Then the tools are returned rather than the notification.
///
/// `@modelcontextprotocol/server-everything` emits
/// `notifications/tools/list_changed` before its `tools/list` reply. Reading
/// one line and calling it the response yielded an empty tool array — and an
/// empty array is a valid result, so `connectors::probe` reported the connector
/// **ok with zero tools**. Measured against this server: 13 tools, reported 0.
///
/// The assertion is deliberately "more than zero" rather than a fixed count:
/// the server is free to add tools, and pinning the number would turn an
/// upstream release into a failure here.
#[test]
#[ignore = "spawns npx; run with --ignored"]
fn a_server_that_speaks_before_answering_still_lists_its_tools() {
    let mut client = McpClient::connect(&everything_server())
        .expect("everything server should start — is npx on PATH?");

    let tools = client.list_tools().expect("tools/list should succeed");

    assert!(
        !tools.is_empty(),
        "server reported no tools; before the id-matching fix this was the \
         silent failure that made a working connector look empty"
    );
    assert!(
        tools.iter().any(|t| t.name == "echo"),
        "expected the reference server's `echo` tool, got: {:?}",
        tools.iter().map(|t| &t.name).collect::<Vec<_>>()
    );
}

/// Given a server with no notification quirk,
/// When its tools are listed,
/// Then it still works — the fix must not have broken the ordinary path.
#[test]
#[ignore = "spawns npx; run with --ignored"]
fn an_ordinary_server_is_unaffected() {
    let cfg = McpServerConfig {
        name: "memory".to_string(),
        command: "npx".to_string(),
        args: vec![
            "-y".to_string(),
            "@modelcontextprotocol/server-memory".to_string(),
        ],
        env: HashMap::new(),
    };
    let mut client = McpClient::connect(&cfg).expect("memory server should start");
    let tools = client.list_tools().expect("tools/list should succeed");
    assert!(!tools.is_empty(), "memory server should expose tools");
}
