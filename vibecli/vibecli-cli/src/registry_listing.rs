//! ACP + MCP Registry self-listing (gap C6).
//!
//! Being discoverable matters: the ACP Registry (built into Zed + JetBrains)
//! lists 28+ agents (Claude Code, Codex CLI, Copilot CLI, Gemini CLI, OpenCode,
//! Goose, Cline, Auggie); the MCP Registry froze its v0.1 API as an app-store
//! for servers. VibeCLI already speaks ACP as a server ([`crate::acp_stdio`])
//! and ships an MCP server, but it was absent from both registries.
//!
//! This module produces the two manifests VibeCLI submits to register itself —
//! an **ACP agent card** and an **MCP Registry server entry** — so the listing
//! is generated from one source of truth (versioned with the binary) rather than
//! hand-maintained JSON. The `vibecli registry <acp|mcp>` surface prints these
//! for a registry PR; the daemon can also serve the ACP card at
//! `/.well-known/agent.json` alongside the existing `/.well-known/mcp.json` (A3).

use serde::{Deserialize, Serialize};

/// Canonical identity used across both registry manifests.
pub struct ListingIdentity<'a> {
    pub name: &'a str,
    pub version: &'a str,
    pub description: &'a str,
    pub homepage: &'a str,
    pub repository: &'a str,
}

/// VibeCLI's identity for registry listings. Version tracks the crate version.
pub fn vibecli_identity() -> ListingIdentity<'static> {
    ListingIdentity {
        name: "vibecli",
        version: env!("CARGO_PKG_VERSION"),
        description: "Provider-agnostic, self-hostable AI coding agent — CLI + daemon \
                      spanning desktop, mobile, and watch, with 22 LLM providers.",
        homepage: "https://github.com/TuringWorks/vibecody",
        repository: "https://github.com/TuringWorks/vibecody",
    }
}

/// An ACP Registry agent card (the shape Zed/JetBrains discover).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct AcpAgentCard {
    /// Stable agent id (reverse-DNS recommended by the registry).
    pub id: String,
    pub name: String,
    pub version: String,
    pub description: String,
    /// How the host launches VibeCLI as an ACP server over stdio.
    pub command: String,
    pub args: Vec<String>,
    /// Protocols this agent speaks.
    pub protocols: Vec<String>,
    pub homepage: String,
}

/// Build the ACP agent card from the canonical identity.
pub fn acp_agent_card(id: &ListingIdentity<'_>) -> AcpAgentCard {
    AcpAgentCard {
        id: "ai.vibecody.vibecli".to_string(),
        name: id.name.to_string(),
        version: id.version.to_string(),
        description: id.description.to_string(),
        command: "vibecli".to_string(),
        // `vibecli acp` runs the ACP stdio JSON-RPC dispatcher (A4).
        args: vec!["acp".to_string()],
        protocols: vec!["acp/0.11".to_string(), "mcp/2026-07-28".to_string()],
        homepage: id.homepage.to_string(),
    }
}

/// An MCP Registry v0.1 server entry.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct McpRegistryEntry {
    /// Reverse-DNS server name per the registry naming rule.
    pub name: String,
    pub description: String,
    pub version: String,
    pub repository: String,
    /// How to run the server (stdio package invocation).
    pub packages: Vec<McpPackage>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct McpPackage {
    /// Registry of the package (`oci`, `npm`, `pypi`, or `binary`).
    pub registry: String,
    pub identifier: String,
    pub version: String,
    pub command: String,
    pub args: Vec<String>,
    /// Transport: `stdio` or `streamable-http`.
    pub transport: String,
}

/// Build the MCP Registry entry from the canonical identity.
pub fn mcp_registry_entry(id: &ListingIdentity<'_>) -> McpRegistryEntry {
    McpRegistryEntry {
        name: "ai.vibecody/vibecli".to_string(),
        description: id.description.to_string(),
        version: id.version.to_string(),
        repository: id.repository.to_string(),
        packages: vec![McpPackage {
            registry: "binary".to_string(),
            identifier: "vibecli".to_string(),
            version: id.version.to_string(),
            command: "vibecli".to_string(),
            // `vibecli mcp-serve` exposes the daemon's MCP server over stdio.
            args: vec!["mcp-serve".to_string()],
            transport: "stdio".to_string(),
        }],
    }
}

/// Render both manifests as pretty JSON for a `vibecli registry` invocation /
/// a registry-submission PR. `which` selects `"acp"`, `"mcp"`, or `"all"`.
pub fn render_listing(which: &str) -> String {
    let id = vibecli_identity();
    match which {
        "acp" => serde_json::to_string_pretty(&acp_agent_card(&id))
            .unwrap_or_else(|_| "{}".to_string()),
        "mcp" => serde_json::to_string_pretty(&mcp_registry_entry(&id))
            .unwrap_or_else(|_| "{}".to_string()),
        _ => {
            let combined = serde_json::json!({
                "acp": acp_agent_card(&id),
                "mcp": mcp_registry_entry(&id),
            });
            serde_json::to_string_pretty(&combined).unwrap_or_else(|_| "{}".to_string())
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn acp_card_has_stdio_launch_and_protocols() {
        let card = acp_agent_card(&vibecli_identity());
        assert_eq!(card.id, "ai.vibecody.vibecli");
        assert_eq!(card.command, "vibecli");
        assert_eq!(card.args, vec!["acp".to_string()]);
        assert!(card.protocols.iter().any(|p| p.starts_with("acp/")));
        assert!(card.protocols.iter().any(|p| p.starts_with("mcp/")));
    }

    #[test]
    fn mcp_entry_has_reverse_dns_name_and_package() {
        let entry = mcp_registry_entry(&vibecli_identity());
        assert!(entry.name.contains('/'));
        assert_eq!(entry.packages.len(), 1);
        assert_eq!(entry.packages[0].transport, "stdio");
        assert_eq!(entry.packages[0].command, "vibecli");
    }

    #[test]
    fn version_tracks_crate_version() {
        let id = vibecli_identity();
        assert_eq!(id.version, env!("CARGO_PKG_VERSION"));
        assert_eq!(acp_agent_card(&id).version, env!("CARGO_PKG_VERSION"));
    }

    #[test]
    fn render_listing_variants_are_valid_json() {
        for which in ["acp", "mcp", "all"] {
            let s = render_listing(which);
            let v: serde_json::Value = serde_json::from_str(&s).unwrap();
            assert!(v.is_object());
        }
    }
}
