//! Admin policy enforcement for agent tool calls.
//!
//! Workspace administrators can place a `.vibecli/policy.toml` file in the
//! repository root (or a global policy at `~/.vibecli/policy.toml`) to
//! restrict what the agent is allowed to do.
//!
//! # Example `.vibecli/policy.toml`
//!
//! ```toml
//! max_steps = 20
//! denied_tools = ["bash"]
//! require_approval_tools = ["write_file", "apply_patch"]
//!
//! [[allow_paths]]
//! pattern = "src/**"
//!
//! [[deny_paths]]
//! pattern = "**/.env"
//! pattern = "**/secrets/**"
//! ```

use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};

// ── AdminPolicy ───────────────────────────────────────────────────────────────

/// Workspace-level restrictions applied before every tool call.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct AdminPolicy {
    /// Maximum number of agent loop steps. `None` = unlimited.
    #[serde(default)]
    pub max_steps: Option<usize>,

    /// Tool names that are completely blocked.
    /// Example: `["bash"]` to prevent shell execution.
    #[serde(default)]
    pub denied_tools: Vec<String>,

    /// Tool names that always require user approval regardless of ApprovalPolicy.
    #[serde(default)]
    pub require_approval_tools: Vec<String>,

    /// Glob patterns for file paths the agent is allowed to read/write.
    /// Empty list = allow all paths.
    #[serde(default)]
    pub allow_paths: Vec<String>,

    /// Glob patterns for file paths the agent must never touch.
    #[serde(default)]
    pub deny_paths: Vec<String>,

    /// Whether to log every policy check to stderr. Useful for auditing.
    #[serde(default)]
    pub audit_log: bool,

    /// Wildcard tool-argument patterns to deny.
    /// Format: `"bash(rm*)"` — blocks `bash` tool when first argument matches `rm*`.
    /// Plain glob (no parens) matches just the tool name.
    /// Example: `["bash(rm*)", "bash(sudo*)"]`
    #[serde(default)]
    pub denied_tool_patterns: Vec<String>,
}

impl AdminPolicy {
    /// Load policy from the workspace root (`.vibecli/policy.toml`).
    /// Falls back to global `~/.vibecli/policy.toml`.
    /// Returns `Default::default()` (permissive) if neither file exists.
    pub fn load(workspace_root: &Path) -> Self {
        // Workspace-level policy takes precedence
        let workspace_policy = workspace_root.join(".vibecli").join("policy.toml");
        if let Some(policy) = Self::try_load(&workspace_policy) {
            return policy;
        }

        // Global fallback
        if let Ok(home) = std::env::var("HOME") {
            let global = PathBuf::from(home).join(".vibecli").join("policy.toml");
            if let Some(policy) = Self::try_load(&global) {
                return policy;
            }
        }

        Self::default()
    }

    fn try_load(path: &Path) -> Option<Self> {
        let content = std::fs::read_to_string(path).ok()?;
        toml::from_str(&content).ok()
    }

    /// Check whether the given tool call is allowed under this policy.
    /// Returns `Err(reason)` if the call should be blocked.
    pub fn check_tool(&self, tool_name: &str) -> PolicyDecision {
        // Check denied tools
        let tool_lower = tool_name.to_lowercase();
        for denied in &self.denied_tools {
            if denied.to_lowercase() == tool_lower || denied == "*" {
                let reason = format!("Tool '{}' is blocked by admin policy", tool_name);
                if self.audit_log {
                    eprintln!("[policy] BLOCKED: {}", reason);
                }
                return PolicyDecision::Block(reason);
            }
        }

        // Check require-approval tools
        for required in &self.require_approval_tools {
            if required.to_lowercase() == tool_lower || required == "*" {
                if self.audit_log {
                    eprintln!("[policy] REQUIRE_APPROVAL: {}", tool_name);
                }
                return PolicyDecision::RequireApproval;
            }
        }

        if self.audit_log {
            eprintln!("[policy] ALLOW: {}", tool_name);
        }
        PolicyDecision::Allow
    }

    /// Check whether a file path is allowed under this policy.
    /// Returns `Err(reason)` if the path should be blocked.
    pub fn check_path(&self, path: &str) -> PolicyDecision {
        // Check deny_paths first
        for pattern in &self.deny_paths {
            if glob_match(pattern, path) {
                let reason = format!(
                    "Path '{}' is blocked by admin policy (deny pattern: {})",
                    path, pattern
                );
                if self.audit_log {
                    eprintln!("[policy] PATH_BLOCKED: {}", reason);
                }
                return PolicyDecision::Block(reason);
            }
        }

        // Check allow_paths (if list is non-empty, path must match at least one)
        if !self.allow_paths.is_empty() {
            let allowed = self.allow_paths.iter().any(|p| glob_match(p, path));
            if !allowed {
                let reason = format!(
                    "Path '{}' is not in allowed paths (policy allow_paths is restricted)",
                    path
                );
                if self.audit_log {
                    eprintln!("[policy] PATH_NOT_ALLOWED: {}", reason);
                }
                return PolicyDecision::Block(reason);
            }
        }

        PolicyDecision::Allow
    }

    /// Check a tool call including its primary argument against `denied_tool_patterns`.
    ///
    /// Pattern format: `"bash(rm*)"` — tool name + optional arg glob in parentheses.
    /// Plain `"bash"` matches the tool name regardless of args.
    pub fn check_tool_with_args(&self, tool_name: &str, primary_arg: &str) -> PolicyDecision {
        // First run the regular check (exact-name denied_tools)
        let base = self.check_tool(tool_name);
        if !matches!(base, PolicyDecision::Allow) {
            return base;
        }

        for pattern in &self.denied_tool_patterns {
            if let Some((tool_pat, arg_pat)) = parse_tool_pattern(pattern) {
                if glob_match(&tool_pat, tool_name) && glob_match(&arg_pat, primary_arg) {
                    let reason = format!(
                        "Tool call '{}({})' blocked by denied_tool_patterns (pattern: {})",
                        tool_name, primary_arg, pattern
                    );
                    if self.audit_log {
                        eprintln!("[policy] BLOCKED (pattern): {}", reason);
                    }
                    return PolicyDecision::Block(reason);
                }
            } else {
                // No parens: treat as plain tool name glob
                if glob_match(pattern, tool_name) {
                    let reason = format!("Tool '{}' blocked by denied_tool_patterns", tool_name);
                    if self.audit_log {
                        eprintln!("[policy] BLOCKED (pattern): {}", reason);
                    }
                    return PolicyDecision::Block(reason);
                }
            }
        }

        PolicyDecision::Allow
    }

    /// Returns true if the policy overrides the approval policy to require
    /// manual approval for this tool.
    pub fn requires_approval(&self, tool_name: &str) -> bool {
        matches!(self.check_tool(tool_name), PolicyDecision::RequireApproval)
    }

    /// Returns true if this policy allows a higher step limit than `current`.
    pub fn effective_max_steps(&self, default: usize) -> usize {
        self.max_steps.unwrap_or(default)
    }
}

// ── PolicyDecision ────────────────────────────────────────────────────────────

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum PolicyDecision {
    /// The action is permitted.
    Allow,
    /// The action is permitted but requires explicit user approval.
    RequireApproval,
    /// The action is blocked. Contains a human-readable reason.
    Block(String),
}

// ── Glob matching ─────────────────────────────────────────────────────────────

/// Parse `"bash(rm*)"` → `Some(("bash", "rm*"))`. Returns `None` if no parens.
fn parse_tool_pattern(pattern: &str) -> Option<(String, String)> {
    let paren = pattern.find('(')?;
    let end = pattern.rfind(')')?;
    if end <= paren { return None; }
    let tool_pat = pattern[..paren].trim().to_string();
    let arg_pat = pattern[paren + 1..end].trim().to_string();
    Some((tool_pat, arg_pat))
}

/// Minimal glob matcher supporting `*`, `**`, and `?`.
/// Does not require an external crate.
fn glob_match(pattern: &str, path: &str) -> bool {
    glob_match_impl(
        pattern.as_bytes(),
        path.as_bytes(),
    )
}

fn glob_match_impl(pat: &[u8], text: &[u8]) -> bool {
    let mut pi = 0usize;
    let mut ti = 0usize;
    let mut star_pi = usize::MAX;
    let mut star_ti = 0usize;

    while ti < text.len() {
        if pi < pat.len() && (pat[pi] == b'?' || pat[pi] == text[ti]) {
            pi += 1;
            ti += 1;
        } else if pi < pat.len() && pat[pi] == b'*' {
            // Handle **
            if pi + 1 < pat.len() && pat[pi + 1] == b'*' {
                // ** matches any path segment including /
                star_pi = pi;
                star_ti = ti;
                pi += 2;
                if pi < pat.len() && pat[pi] == b'/' {
                    pi += 1;
                }
            } else {
                star_pi = pi;
                star_ti = ti;
                pi += 1;
            }
        } else if star_pi != usize::MAX {
            pi = star_pi + 1;
            star_ti += 1;
            ti = star_ti;
        } else {
            return false;
        }
    }

    // Consume trailing *
    while pi < pat.len() && (pat[pi] == b'*') {
        pi += 1;
    }

    pi == pat.len()
}

// ── Tests ─────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn tool_denied() {
        let policy = AdminPolicy {
            denied_tools: vec!["bash".into()],
            ..Default::default()
        };
        assert_eq!(policy.check_tool("bash"), PolicyDecision::Block("Tool 'bash' is blocked by admin policy".into()));
        assert_eq!(policy.check_tool("read_file"), PolicyDecision::Allow);
    }

    #[test]
    fn tool_require_approval() {
        let policy = AdminPolicy {
            require_approval_tools: vec!["write_file".into()],
            ..Default::default()
        };
        assert_eq!(policy.check_tool("write_file"), PolicyDecision::RequireApproval);
        assert_eq!(policy.check_tool("read_file"), PolicyDecision::Allow);
    }

    #[test]
    fn path_deny() {
        let policy = AdminPolicy {
            deny_paths: vec!["**/.env".into(), "**/secrets/**".into()],
            ..Default::default()
        };
        assert!(matches!(policy.check_path(".env"), PolicyDecision::Block(_)));
        assert!(matches!(policy.check_path("src/.env"), PolicyDecision::Block(_)));
        assert!(matches!(policy.check_path("config/secrets/key.pem"), PolicyDecision::Block(_)));
        assert_eq!(policy.check_path("src/main.rs"), PolicyDecision::Allow);
    }

    #[test]
    fn path_allow_list() {
        let policy = AdminPolicy {
            allow_paths: vec!["src/**".into()],
            ..Default::default()
        };
        assert_eq!(policy.check_path("src/main.rs"), PolicyDecision::Allow);
        assert!(matches!(policy.check_path("scripts/build.sh"), PolicyDecision::Block(_)));
    }

    #[test]
    fn max_steps_override() {
        let policy = AdminPolicy { max_steps: Some(10), ..Default::default() };
        assert_eq!(policy.effective_max_steps(30), 10);

        let permissive = AdminPolicy::default();
        assert_eq!(permissive.effective_max_steps(30), 30);
    }

    #[test]
    fn glob_star() {
        assert!(glob_match("*.rs", "main.rs"));
        assert!(!glob_match("*.rs", "main.ts"));
        assert!(glob_match("src/**", "src/main.rs"));
        assert!(glob_match("src/**", "src/lib/utils.rs"));
        assert!(glob_match("**/.env", ".env"));
        assert!(glob_match("**/.env", "dir/.env"));
    }

    #[test]
    fn policy_default_is_permissive() {
        let policy = AdminPolicy::default();
        assert_eq!(policy.check_tool("bash"), PolicyDecision::Allow);
        assert_eq!(policy.check_path("/etc/passwd"), PolicyDecision::Allow);
    }
}
