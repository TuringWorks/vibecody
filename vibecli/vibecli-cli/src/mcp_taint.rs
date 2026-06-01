#![allow(dead_code)] // Staged wave6 / Phase 53 module — wired up in a later cycle
//! Tainted boundary for the **MCP client** — when VibeCody invokes an
//! external MCP server and consumes its tool-call output.
//!
//! DREAD #1 Slice D. The external MCP server is a T5 actor — its `text`
//! return value can contain a prompt-injection payload that the model
//! will faithfully echo back into a `ToolCall::Bash` argument unless we
//! mark it tainted at the boundary.
//!
//! ## Where this helper sits
//!
//! `vibe_ai::mcp::McpClient::call_tool` is the raw transport — it speaks
//! JSON-RPC 2.0 over stdio to the MCP server process. It returns
//! `Result<String>`. That return value crosses our T0/T5 trust
//! boundary and **must** be wrapped in [`Tainted<String>`] before
//! any caller in vibecli-cli consumes it.
//!
//! `vibe-ai` itself does not depend on `vibecli-cli`, so the
//! [`Tainted`] type can't live there without a workspace-level
//! refactor. This module is the typed boundary helper: every future
//! call site in `vibecli-cli` that invokes an external MCP server
//! routes through [`call_tool_tainted`] and receives back a
//! [`Tainted<String>`] with [`Provenance::Mcp`] populated. The model
//! tool-call dispatchers (slice B `confirm_shell_command`, slice C
//! `confirm_http_outbound`) then see the right provenance kind when
//! the tainted bytes eventually flow into a privileged sink.
//!
//! ## Current state
//!
//! The agent loop in `vibecli-cli/src/main.rs` does not yet have a
//! model→MCP call-tool path — only `/mcp list` and `/mcp tools` are
//! wired. This module ships the boundary now so when the call-tool
//! wiring lands (cross-cutting change tracked in the AGENTS.md product
//! matrix), the design forces the right discipline by type.
//!
//! See [`docs/security/tainted-data-flow.md`](../../docs/security/tainted-data-flow.md) §5
//! entry #3 + §6.1.

use anyhow::Result;
use serde_json::Value;
use uuid::Uuid;

use crate::tainted::{Provenance, Tainted};

/// Invoke an MCP tool and wrap the returned text in [`Tainted<String>`]
/// at the boundary.
///
/// The `server` name and `tool` name are recorded in the
/// [`Provenance::Mcp`] payload so the audit log + future admin policy
/// can attribute a downstream tool-call rejection to a specific MCP
/// server. A fresh `call_id` (UUID v4) is generated per invocation so
/// the gate-decision tracing line can be correlated to the originating
/// MCP request.
///
/// Callers in the agent loop **must** use this helper rather than
/// invoking `client.call_tool(...)` directly. A semgrep rule guards
/// the boundary; see `.semgrep/mcp-taint-boundary.yml`.
pub fn call_tool_tainted(
    client: &mut vibe_ai::mcp::McpClient,
    server: impl Into<String>,
    tool: impl Into<String>,
    arguments: Value,
) -> Result<Tainted<String>> {
    let server = server.into();
    let tool = tool.into();
    let call_id = Uuid::new_v4().to_string();

    // Audit the boundary crossing *before* the call goes out. If the
    // MCP server is slow or stalls, the audit line still names the
    // server/tool/call_id so an operator can correlate a hung tool
    // call to its MCP origin.
    tracing::debug!(
        target: "vibecody::tainted::mcp_boundary",
        server = %server,
        tool = %tool,
        call_id = %call_id,
        "mcp.call_tool dispatched (tainted boundary)",
    );

    let raw = client.call_tool(&tool, arguments)?;
    let tainted = Tainted::new(
        raw,
        Provenance::Mcp {
            server: server.clone(),
            tool: tool.clone(),
            call_id: call_id.clone(),
        },
    );

    // Slice F: surface the per-payload fingerprint at the boundary so
    // downstream `shell_gate` / `http_gate` rejection lines can be
    // correlated to *this* MCP response in the audit log (`grep
    // fingerprint=[tainted/mcp/abcdef12]`).
    tracing::debug!(
        target: "vibecody::tainted::mcp_boundary",
        server = %server,
        tool = %tool,
        call_id = %call_id,
        bytes = tainted.byte_len(),
        fingerprint = %tainted.log_fingerprint(),
        "mcp.call_tool returned (wrapping with Provenance::Mcp)",
    );

    Ok(tainted)
}

/// Per-MCP-server admin policy hook — surfaces the boundary to the
/// confirmation flow that Slice G ships. Today this is a no-op (no
/// policy engine wired); it is checked-in shape so future callers can
/// drop in deny rules without changing the boundary signature.
///
/// Returns `Ok(())` to permit the response, or `Err(reason)` to reject
/// — caller is expected to surface the rejection as a `tool_result`
/// with `status: "user_rejected"` matching the slice-B / slice-C
/// pattern.
pub fn audit_mcp_response(response: &Tainted<String>) -> std::result::Result<(), String> {
    match response.origin() {
        Provenance::Mcp {
            server,
            tool,
            call_id,
        } => {
            tracing::debug!(
                target: "vibecody::tainted::mcp_boundary",
                server = %server,
                tool = %tool,
                call_id = %call_id,
                fingerprint = %response.log_fingerprint(),
                "mcp.response audited (slice D policy hook — no-op until slice G ships admin policy)",
            );
            Ok(())
        }
        // The boundary helper is the only constructor of MCP-provenance
        // taint. A caller that hand-built a `Tainted<String>` with a
        // non-MCP origin and routed it through here is a bug — surface
        // it loud rather than silently accept.
        other => Err(format!(
            "audit_mcp_response received non-MCP provenance: {} \
             (boundary helper invariant violated)",
            other.kind()
        )),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn audit_mcp_response_accepts_mcp_provenance() {
        let t = Tainted::new(
            "tool returned this text".to_string(),
            Provenance::Mcp {
                server: "fs-server".into(),
                tool: "read".into(),
                call_id: "call-1".into(),
            },
        );
        assert!(audit_mcp_response(&t).is_ok());
    }

    #[test]
    fn audit_mcp_response_rejects_file_provenance() {
        // The boundary helper is the *only* constructor of MCP-tainted
        // values. Anything else routed in is a bug.
        let t = Tainted::from_file("/repo/README.md", "x".into());
        let err = audit_mcp_response(&t).unwrap_err();
        assert!(err.contains("non-MCP provenance"), "got: {err}");
        assert!(err.contains("file"), "got: {err}");
    }

    #[test]
    fn audit_mcp_response_rejects_llm_provenance() {
        let t = Tainted::from_llm_response("anthropic", "claude-opus-4-7", "req-1", "x".into());
        let err = audit_mcp_response(&t).unwrap_err();
        assert!(err.contains("non-MCP provenance"), "got: {err}");
        assert!(err.contains("llm"), "got: {err}");
    }
}
