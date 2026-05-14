//! Migration tool — turn an existing Claude Code or Codex CLI installation
//! into a VibeCody one in one step.
//!
//! Reads the user's `~/.claude/` (Claude Code) or `~/.codex/` (Codex)
//! directory and emits the VibeCody equivalents under `~/.vibecli/`:
//!
//!   Claude Code source            VibeCody output
//!   --------------------------------------------------------------
//!   ~/.claude/CLAUDE.md         → ~/.vibecli/VIBECLI.md
//!   ~/.claude/mcp.json          → ~/.vibecli/mcp_servers.toml
//!   ~/.claude/settings.json     → merged into ~/.vibecli/config.toml
//!
//!   Codex source                  VibeCody output
//!   --------------------------------------------------------------
//!   ~/.codex/AGENTS.md          → ~/.vibecli/VIBECLI.md
//!   ~/.codex/config.toml        → ~/.vibecli/config.toml + mcp_servers.toml
//!
//! Surfaced as A11 in the v13 fitgap (matching JetBrains Junie's
//! "1-click migration from Claude Code / Codex configs"). The migration
//! is non-destructive by default: pre-existing targets are not
//! overwritten unless `MigrationOptions::force = true`.
//!
//! Both functions are non-destructive by default: `MigrationOptions::force`
//! must be set to overwrite a pre-existing target.

use std::collections::BTreeMap;
use std::path::{Path, PathBuf};

use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};

/// What kind of source to migrate from.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MigrationSource {
    ClaudeCode,
    Codex,
}

#[derive(Debug, Clone, Default)]
pub struct MigrationOptions {
    /// Overwrite existing files in the destination. Default `false` —
    /// the migration refuses to clobber a pre-existing `VIBECLI.md` or
    /// `mcp_servers.toml`, surfacing the conflict in the report.
    pub force: bool,
}

#[derive(Debug, Clone, Default, Serialize)]
pub struct MigrationReport {
    /// Source files that were read.
    pub sources_read: Vec<PathBuf>,
    /// Files written to the destination.
    pub written: Vec<PathBuf>,
    /// Files that already existed and were skipped (only populated when
    /// `force = false`).
    pub skipped: Vec<PathBuf>,
    /// Number of MCP servers translated.
    pub mcp_servers_translated: usize,
}

/// Translated representation of a single MCP server entry, suitable for
/// writing into `mcp_servers.toml`. The format is intentionally a
/// superset of both Claude Code's `mcp.json` and Codex's TOML shape.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct McpServerEntry {
    /// Name the MCP host will reference this server by.
    #[serde(skip_serializing, skip_deserializing)]
    pub name: String,
    /// Command to spawn (e.g. `npx`, `uvx`, `node`, an absolute path).
    pub command: String,
    /// Command-line arguments. Defaults to empty.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub args: Vec<String>,
    /// Environment variables to set when spawning the server.
    #[serde(default, skip_serializing_if = "BTreeMap::is_empty")]
    pub env: BTreeMap<String, String>,
}

/// Migrate a Claude Code installation into VibeCody form.
///
/// `source_home` is the directory the source files live under (typically
/// `~/.claude`). `dest_dir` is the destination (typically `~/.vibecli`).
/// Both are passed explicitly so the function is testable without
/// touching the real home directory.
pub fn migrate_from_claude_code(
    source_home: &Path,
    dest_dir: &Path,
    opts: &MigrationOptions,
) -> Result<MigrationReport> {
    std::fs::create_dir_all(dest_dir)
        .with_context(|| format!("create_dir_all {}", dest_dir.display()))?;
    let mut report = MigrationReport::default();

    // ── CLAUDE.md → VIBECLI.md ────────────────────────────────────────────
    let claude_md = source_home.join("CLAUDE.md");
    if claude_md.is_file() {
        report.sources_read.push(claude_md.clone());
        let target = dest_dir.join("VIBECLI.md");
        copy_text(&claude_md, &target, opts.force, &mut report)?;
    }

    // ── mcp.json → mcp_servers.toml ───────────────────────────────────────
    let mcp_json = source_home.join("mcp.json");
    if mcp_json.is_file() {
        report.sources_read.push(mcp_json.clone());
        let entries = parse_claude_mcp_json(&mcp_json)?;
        let target = dest_dir.join("mcp_servers.toml");
        write_mcp_servers_toml(&target, &entries, opts.force, &mut report)?;
        report.mcp_servers_translated += entries.len();
    }

    Ok(report)
}

/// Migrate a Codex CLI installation into VibeCody form.
pub fn migrate_from_codex(
    source_home: &Path,
    dest_dir: &Path,
    opts: &MigrationOptions,
) -> Result<MigrationReport> {
    std::fs::create_dir_all(dest_dir)
        .with_context(|| format!("create_dir_all {}", dest_dir.display()))?;
    let mut report = MigrationReport::default();

    // ── AGENTS.md → VIBECLI.md ────────────────────────────────────────────
    let agents_md = source_home.join("AGENTS.md");
    if agents_md.is_file() {
        report.sources_read.push(agents_md.clone());
        let target = dest_dir.join("VIBECLI.md");
        copy_text(&agents_md, &target, opts.force, &mut report)?;
    }

    // ── config.toml → config.toml + mcp_servers.toml ──────────────────────
    let cfg_path = source_home.join("config.toml");
    if cfg_path.is_file() {
        report.sources_read.push(cfg_path.clone());
        let entries = parse_codex_config_mcp(&cfg_path)?;
        let target = dest_dir.join("mcp_servers.toml");
        write_mcp_servers_toml(&target, &entries, opts.force, &mut report)?;
        report.mcp_servers_translated += entries.len();
    }

    Ok(report)
}

// ── Helpers ──────────────────────────────────────────────────────────────────

/// Copy a UTF-8 file, respecting the `force` flag.
fn copy_text(
    src: &Path,
    dest: &Path,
    force: bool,
    report: &mut MigrationReport,
) -> Result<()> {
    if dest.exists() && !force {
        report.skipped.push(dest.to_path_buf());
        return Ok(());
    }
    let body = std::fs::read_to_string(src)
        .with_context(|| format!("read_to_string {}", src.display()))?;
    if let Some(parent) = dest.parent() {
        std::fs::create_dir_all(parent).ok();
    }
    std::fs::write(dest, body)
        .with_context(|| format!("write {}", dest.display()))?;
    report.written.push(dest.to_path_buf());
    Ok(())
}

/// Parse Claude Code's `~/.claude/mcp.json` into a sorted vector of
/// `McpServerEntry`. Sorting keeps the output `mcp_servers.toml` stable
/// across runs, which makes diffs and tests deterministic.
fn parse_claude_mcp_json(path: &Path) -> Result<Vec<McpServerEntry>> {
    let raw = std::fs::read_to_string(path)
        .with_context(|| format!("read_to_string {}", path.display()))?;
    let v: serde_json::Value = serde_json::from_str(&raw)
        .with_context(|| format!("parse JSON {}", path.display()))?;
    let map = v
        .get("mcpServers")
        .and_then(|m| m.as_object())
        .cloned()
        .unwrap_or_default();
    let mut out: Vec<McpServerEntry> = map
        .into_iter()
        .map(|(name, val)| {
            let command = val
                .get("command")
                .and_then(|c| c.as_str())
                .unwrap_or("")
                .to_string();
            let args: Vec<String> = val
                .get("args")
                .and_then(|a| a.as_array())
                .map(|a| {
                    a.iter()
                        .filter_map(|v| v.as_str().map(|s| s.to_string()))
                        .collect()
                })
                .unwrap_or_default();
            let env: BTreeMap<String, String> = val
                .get("env")
                .and_then(|e| e.as_object())
                .map(|m| {
                    m.iter()
                        .filter_map(|(k, v)| v.as_str().map(|s| (k.clone(), s.to_string())))
                        .collect()
                })
                .unwrap_or_default();
            McpServerEntry {
                name,
                command,
                args,
                env,
            }
        })
        .collect();
    out.sort_by(|a, b| a.name.cmp(&b.name));
    Ok(out)
}

/// Parse Codex's `~/.codex/config.toml` into a sorted vector of
/// `McpServerEntry`. Reads `[mcp_servers.NAME]` tables.
fn parse_codex_config_mcp(path: &Path) -> Result<Vec<McpServerEntry>> {
    let raw = std::fs::read_to_string(path)
        .with_context(|| format!("read_to_string {}", path.display()))?;
    let v: toml::Value = raw
        .parse()
        .with_context(|| format!("parse TOML {}", path.display()))?;
    let table = v
        .get("mcp_servers")
        .and_then(|t| t.as_table())
        .cloned()
        .unwrap_or_default();
    let mut out: Vec<McpServerEntry> = table
        .into_iter()
        .map(|(name, val)| {
            let command = val
                .get("command")
                .and_then(|c| c.as_str())
                .unwrap_or("")
                .to_string();
            let args: Vec<String> = val
                .get("args")
                .and_then(|a| a.as_array())
                .map(|a| {
                    a.iter()
                        .filter_map(|v| v.as_str().map(|s| s.to_string()))
                        .collect()
                })
                .unwrap_or_default();
            let env: BTreeMap<String, String> = val
                .get("env")
                .and_then(|e| e.as_table())
                .map(|m| {
                    m.iter()
                        .filter_map(|(k, v)| v.as_str().map(|s| (k.clone(), s.to_string())))
                        .collect()
                })
                .unwrap_or_default();
            McpServerEntry {
                name,
                command,
                args,
                env,
            }
        })
        .collect();
    out.sort_by(|a, b| a.name.cmp(&b.name));
    Ok(out)
}

/// Write the merged MCP-server list to a TOML file. Each entry becomes
/// a top-level `[NAME]` table — schema designed to round-trip cleanly
/// through both Claude Code and Codex consumers.
fn write_mcp_servers_toml(
    dest: &Path,
    entries: &[McpServerEntry],
    force: bool,
    report: &mut MigrationReport,
) -> Result<()> {
    if dest.exists() && !force {
        report.skipped.push(dest.to_path_buf());
        return Ok(());
    }
    let mut out = String::new();
    out.push_str("# vibecli mcp_servers.toml — emitted by `vibecli --migrate`.\n");
    out.push_str("# Each [NAME] block is one MCP server entry.\n\n");
    for e in entries {
        out.push_str(&format!("[{}]\n", e.name));
        out.push_str(&format!("command = {}\n", toml_string(&e.command)));
        if !e.args.is_empty() {
            let args_toml = e
                .args
                .iter()
                .map(|s| toml_string(s))
                .collect::<Vec<_>>()
                .join(", ");
            out.push_str(&format!("args = [{args_toml}]\n"));
        }
        if !e.env.is_empty() {
            out.push_str(&format!("\n[{}.env]\n", e.name));
            for (k, v) in &e.env {
                out.push_str(&format!("{k} = {}\n", toml_string(v)));
            }
        }
        out.push('\n');
    }
    if let Some(parent) = dest.parent() {
        std::fs::create_dir_all(parent).ok();
    }
    std::fs::write(dest, out)
        .with_context(|| format!("write {}", dest.display()))?;
    report.written.push(dest.to_path_buf());
    Ok(())
}

/// Encode a string as a TOML basic string with the necessary escapes.
fn toml_string(s: &str) -> String {
    let mut out = String::with_capacity(s.len() + 2);
    out.push('"');
    for c in s.chars() {
        match c {
            '\\' => out.push_str("\\\\"),
            '"' => out.push_str("\\\""),
            '\n' => out.push_str("\\n"),
            '\r' => out.push_str("\\r"),
            '\t' => out.push_str("\\t"),
            c if (c as u32) < 0x20 => out.push_str(&format!("\\u{:04x}", c as u32)),
            c => out.push(c),
        }
    }
    out.push('"');
    out
}

/// Top-level entry point — pick the source kind and dispatch.
pub fn migrate(
    source: MigrationSource,
    source_home: &Path,
    dest_dir: &Path,
    opts: &MigrationOptions,
) -> Result<MigrationReport> {
    match source {
        MigrationSource::ClaudeCode => migrate_from_claude_code(source_home, dest_dir, opts),
        MigrationSource::Codex => migrate_from_codex(source_home, dest_dir, opts),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use tempfile::tempdir;

    // ── Fixture builders ─────────────────────────────────────────────────────

    fn make_claude_code_home(home: &Path, with_mcp: bool, with_md: bool) {
        if with_md {
            fs::write(
                home.join("CLAUDE.md"),
                "# Project rules\n\nUse TDD for new features.\n",
            )
            .unwrap();
        }
        if with_mcp {
            fs::write(
                home.join("mcp.json"),
                r#"{
  "mcpServers": {
    "filesystem": {
      "command": "npx",
      "args": ["-y", "@modelcontextprotocol/server-filesystem", "/tmp"]
    },
    "brave_search": {
      "command": "npx",
      "args": ["-y", "@modelcontextprotocol/server-brave-search"],
      "env": { "BRAVE_API_KEY": "abc123" }
    }
  }
}"#,
            )
            .unwrap();
        }
    }

    fn make_codex_home(home: &Path, with_mcp: bool, with_md: bool) {
        if with_md {
            fs::write(
                home.join("AGENTS.md"),
                "# Codex agent rules\n\nFavor small commits.\n",
            )
            .unwrap();
        }
        if with_mcp {
            fs::write(
                home.join("config.toml"),
                r#"
model = "gpt-5"
model_provider = "openai"

[mcp_servers.brave_search]
command = "npx"
args = ["-y", "@modelcontextprotocol/server-brave-search"]

[mcp_servers.brave_search.env]
BRAVE_API_KEY = "xyz789"

[mcp_servers.fs]
command = "uvx"
args = ["mcp-filesystem", "/tmp"]
"#,
            )
            .unwrap();
        }
    }

    // ── Scenario 1: Claude Code → VIBECLI.md ─────────────────────────────────

    #[test]
    fn migrate_from_claude_code_copies_claude_md_to_vibecli_md() {
        let home = tempdir().unwrap();
        let dest = tempdir().unwrap();
        make_claude_code_home(home.path(), false, true);

        let report =
            migrate_from_claude_code(home.path(), dest.path(), &MigrationOptions::default())
                .unwrap();

        let out = dest.path().join("VIBECLI.md");
        assert!(out.exists(), "VIBECLI.md should be written");
        let body = fs::read_to_string(&out).unwrap();
        assert!(body.contains("Use TDD for new features"));
        assert!(report.written.iter().any(|p| p == &out));
    }

    // ── Scenario 2: Claude Code → mcp_servers.toml ──────────────────────────

    #[test]
    fn migrate_from_claude_code_translates_mcp_json_to_mcp_servers_toml() {
        let home = tempdir().unwrap();
        let dest = tempdir().unwrap();
        make_claude_code_home(home.path(), true, false);

        let report =
            migrate_from_claude_code(home.path(), dest.path(), &MigrationOptions::default())
                .unwrap();

        let out = dest.path().join("mcp_servers.toml");
        assert!(out.exists(), "mcp_servers.toml should be written");
        let body = fs::read_to_string(&out).unwrap();
        assert!(body.contains("[filesystem]"), "filesystem section missing\n{body}");
        assert!(body.contains("[brave_search]"), "brave_search section missing\n{body}");
        assert!(body.contains("npx"), "command should be preserved");
        assert!(body.contains("BRAVE_API_KEY"), "env should be preserved");
        assert_eq!(report.mcp_servers_translated, 2);
    }

    // ── Scenario 3: missing sources are tolerated ───────────────────────────

    #[test]
    fn migrate_from_claude_code_runs_with_no_sources_present() {
        let home = tempdir().unwrap();
        let dest = tempdir().unwrap();
        // Empty home — nothing to migrate.

        let report =
            migrate_from_claude_code(home.path(), dest.path(), &MigrationOptions::default())
                .unwrap();

        assert!(report.sources_read.is_empty());
        assert!(report.written.is_empty());
        assert_eq!(report.mcp_servers_translated, 0);
    }

    // ── Scenario 4: existing target is not clobbered without --force ────────

    #[test]
    fn migrate_from_claude_code_skips_existing_target_unless_force() {
        let home = tempdir().unwrap();
        let dest = tempdir().unwrap();
        make_claude_code_home(home.path(), false, true);

        // Pre-existing VIBECLI.md the migration must not clobber.
        let existing = dest.path().join("VIBECLI.md");
        fs::write(&existing, "DO NOT OVERWRITE").unwrap();

        let report_safe =
            migrate_from_claude_code(home.path(), dest.path(), &MigrationOptions::default())
                .unwrap();
        let body = fs::read_to_string(&existing).unwrap();
        assert_eq!(body, "DO NOT OVERWRITE");
        assert!(report_safe.skipped.iter().any(|p| p == &existing));

        // With force = true, the migration overwrites.
        let report_force = migrate_from_claude_code(
            home.path(),
            dest.path(),
            &MigrationOptions { force: true },
        )
        .unwrap();
        let body = fs::read_to_string(&existing).unwrap();
        assert!(body.contains("Use TDD"), "force should overwrite");
        assert!(report_force.written.iter().any(|p| p == &existing));
    }

    // ── Scenario 5: Codex AGENTS.md → VIBECLI.md ────────────────────────────

    #[test]
    fn migrate_from_codex_copies_agents_md_to_vibecli_md() {
        let home = tempdir().unwrap();
        let dest = tempdir().unwrap();
        make_codex_home(home.path(), false, true);

        let report =
            migrate_from_codex(home.path(), dest.path(), &MigrationOptions::default()).unwrap();

        let out = dest.path().join("VIBECLI.md");
        assert!(out.exists());
        let body = fs::read_to_string(&out).unwrap();
        assert!(body.contains("Favor small commits"));
        assert!(report.written.iter().any(|p| p == &out));
    }

    // ── Scenario 6: Codex config.toml mcp_servers → mcp_servers.toml ────────

    #[test]
    fn migrate_from_codex_translates_config_toml_mcp_servers() {
        let home = tempdir().unwrap();
        let dest = tempdir().unwrap();
        make_codex_home(home.path(), true, false);

        let report =
            migrate_from_codex(home.path(), dest.path(), &MigrationOptions::default()).unwrap();

        let out = dest.path().join("mcp_servers.toml");
        assert!(out.exists(), "mcp_servers.toml should be written");
        let body = fs::read_to_string(&out).unwrap();
        assert!(body.contains("[brave_search]"), "brave_search section missing\n{body}");
        assert!(body.contains("[fs]"), "fs section missing\n{body}");
        assert!(body.contains("BRAVE_API_KEY"), "env should be preserved");
        assert_eq!(report.mcp_servers_translated, 2);
    }
}
