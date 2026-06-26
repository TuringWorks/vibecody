//! Opt-in always-on security review (gap B3) — §18.B3 cleared shape.
//!
//! Competitors shipped always-on agentic security review (Cursor Security
//! Review; GitHub Copilot's agentic review on Actions). The VibeCody shape is
//! deliberately *distant* from those, honoring the [§18](../../docs/FIT-GAP-ANALYSIS.md)
//! patent-distance principles:
//!
//! * **Opt-in, default OFF** (#5). The trigger is a user-configured workspace
//!   flag + file-watcher rule; the daemon ships no system-imposed always-on
//!   default and no privileged "security agent" canvas.
//! * **Findings flow through the generic [`crate::self_review::Finding`] schema**
//!   (#6), alongside clippy / eslint / semgrep — the LLM is one finding source
//!   among many, never singled out with a one-click "apply fix" gesture.
//! * **Acting on a finding is an explicit user diffcomplete (⌘.)** — this module
//!   only *produces* findings into the existing `ReviewPanel`; it never mutates
//!   files or runs a hidden fix loop (#6, #8).
//! * **No hidden RAG / cross-file taint** (#9): each review sees only the changed
//!   file the user's own watcher rule selected.
//!
//! This module is the pure controller — the opt-in gate, the path/glob filter,
//! the provider-agnostic prompt, and the finding parser. The daemon supplies the
//! [`crate::file_watcher`] events and the LLM call, so the policy is testable
//! without a watcher or a provider.

use crate::file_watcher::ChangeBatch;
use crate::self_review::{CheckKind, Finding, Severity};
use std::path::{Path, PathBuf};

/// Workspace configuration for opt-in security review. **Disabled by default.**
#[derive(Debug, Clone, PartialEq)]
pub struct SecurityReviewConfig {
    /// Master switch. Off unless the user explicitly opts the workspace in.
    pub enabled: bool,
    /// File suffixes the user's watcher rule covers (e.g. `.rs`, `.ts`). Empty
    /// means "all files" — but only matters when `enabled` is true.
    pub watched_suffixes: Vec<String>,
    /// Findings below this severity are dropped before reaching the panel.
    pub min_severity: Severity,
}

impl Default for SecurityReviewConfig {
    fn default() -> Self {
        Self {
            // Default OFF — §18.B3 principle #5.
            enabled: false,
            watched_suffixes: Vec::new(),
            min_severity: Severity::Warning,
        }
    }
}

impl SecurityReviewConfig {
    /// Whether a changed file should trigger a review. Always false when the
    /// feature is disabled (the opt-in gate), regardless of suffix rules.
    pub fn should_review(&self, path: &Path) -> bool {
        if !self.enabled {
            return false;
        }
        if self.watched_suffixes.is_empty() {
            return true;
        }
        let name = path.to_string_lossy();
        self.watched_suffixes.iter().any(|s| name.ends_with(s))
    }

    /// The always-on bridge (§18.B3): from a [`ChangeBatch`] the file-watcher
    /// flushed, return the changed files that should be security-reviewed —
    /// honoring the opt-in gate + suffix filter. Empty when disabled, so the
    /// daemon's watcher loop is a no-op until a user opts the workspace in. The
    /// daemon then runs [`build_review_prompt`] + the LLM + [`parse_findings`]
    /// per returned path and surfaces the findings in the existing ReviewPanel.
    pub fn review_targets(&self, batch: &ChangeBatch) -> Vec<PathBuf> {
        if !self.enabled {
            return Vec::new();
        }
        let mut targets: Vec<PathBuf> = batch
            .changed_paths()
            .into_iter()
            .filter(|p| self.should_review(p))
            .cloned()
            .collect();
        // Deterministic order so repeated batches review in a stable sequence.
        targets.sort();
        targets
    }
}

/// Build the provider-agnostic review prompt for one changed file. No RAG, no
/// cross-file context — only the file the watcher rule selected (§18 #9).
pub fn build_review_prompt(file: &str, contents: &str) -> String {
    format!(
        "You are a security reviewer. Review ONLY the file below for genuine \
         security issues (injection, auth/authz flaws, secret leakage, unsafe \
         deserialization, path traversal, SSRF, memory safety). Ignore style.\n\n\
         Return ONE finding per line in EXACTLY this format:\n\
         SEVERITY|LINE|MESSAGE|SUGGESTION\n\
         where SEVERITY is one of info|warning|error|critical, LINE is a 1-based \
         line number or 0 if unknown, and SUGGESTION is a short fix hint. If there \
         are no issues, output the single line: NONE\n\n\
         File: {file}\n```\n{contents}\n```"
    )
}

/// Parse the LLM's `SEVERITY|LINE|MESSAGE|SUGGESTION` lines into standard
/// [`Finding`]s tagged [`CheckKind::Security`]. Robust to blank lines, a `NONE`
/// sentinel, and malformed rows (skipped). Findings below `min_severity` are
/// filtered so the panel only surfaces what the user opted into seeing.
pub fn parse_findings(file: &str, output: &str, cfg: &SecurityReviewConfig) -> Vec<Finding> {
    let mut findings = Vec::new();
    for raw in output.lines() {
        let line = raw.trim();
        if line.is_empty() || line.eq_ignore_ascii_case("none") {
            continue;
        }
        let parts: Vec<&str> = line.splitn(4, '|').collect();
        if parts.len() < 3 {
            continue; // malformed — skip rather than surface noise
        }
        let severity = match parts[0].trim().to_lowercase().as_str() {
            "info" => Severity::Info,
            "warning" | "warn" => Severity::Warning,
            "error" => Severity::Error,
            "critical" | "crit" => Severity::Critical,
            _ => continue,
        };
        if severity < cfg.min_severity {
            continue;
        }
        let line_no: usize = parts[1].trim().parse().unwrap_or(0);
        let message = parts[2].trim();
        if message.is_empty() {
            continue;
        }
        let mut finding = Finding::new(CheckKind::Security, severity, message);
        if line_no > 0 {
            finding = finding.with_location(file, line_no);
        } else {
            finding.file = Some(file.to_string());
        }
        if let Some(sugg) = parts.get(3) {
            let sugg = sugg.trim();
            if !sugg.is_empty() {
                finding = finding.with_suggestion(sugg);
            }
        }
        findings.push(finding);
    }
    findings
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;

    #[test]
    fn disabled_by_default_never_reviews() {
        let cfg = SecurityReviewConfig::default();
        assert!(!cfg.enabled);
        assert!(!cfg.should_review(&PathBuf::from("src/main.rs")));
    }

    #[test]
    fn opt_in_respects_suffix_filter() {
        let cfg = SecurityReviewConfig {
            enabled: true,
            watched_suffixes: vec![".rs".into()],
            ..Default::default()
        };
        assert!(cfg.should_review(&PathBuf::from("src/main.rs")));
        assert!(!cfg.should_review(&PathBuf::from("README.md")));
    }

    #[test]
    fn opt_in_empty_suffixes_reviews_all() {
        let cfg = SecurityReviewConfig {
            enabled: true,
            ..Default::default()
        };
        assert!(cfg.should_review(&PathBuf::from("anything.xyz")));
    }

    #[test]
    fn parse_findings_happy_path() {
        let cfg = SecurityReviewConfig {
            enabled: true,
            min_severity: Severity::Info,
            ..Default::default()
        };
        let out = "critical|42|SQL injection in query builder|Use parameterized queries\n\
                   info|0|Consider rate limiting|Add a token bucket";
        let findings = parse_findings("src/db.rs", out, &cfg);
        assert_eq!(findings.len(), 2);
        assert_eq!(findings[0].severity, Severity::Critical);
        assert_eq!(findings[0].line, Some(42));
        assert_eq!(findings[0].check, CheckKind::Security);
        assert!(findings[0].suggestion.is_some());
        // line 0 → no location line, but file is still attached.
        assert_eq!(findings[1].line, None);
        assert_eq!(findings[1].file.as_deref(), Some("src/db.rs"));
    }

    #[test]
    fn parse_findings_filters_below_min_severity() {
        let cfg = SecurityReviewConfig {
            enabled: true,
            min_severity: Severity::Error,
            ..Default::default()
        };
        let out = "info|1|minor|x\nwarning|2|medium|y\nerror|3|serious|z";
        let findings = parse_findings("f.rs", out, &cfg);
        assert_eq!(findings.len(), 1);
        assert_eq!(findings[0].severity, Severity::Error);
    }

    #[test]
    fn review_targets_gates_a_change_batch() {
        use crate::file_watcher::{ChangeBatch, ChangeKind, FileChangeEvent};
        use std::time::Instant;
        let batch = ChangeBatch {
            events: vec![
                FileChangeEvent::new("src/auth.rs", ChangeKind::Modified),
                FileChangeEvent::new("README.md", ChangeKind::Modified),
                FileChangeEvent::new("src/db.rs", ChangeKind::Created),
            ],
            window_start: Instant::now(),
            window_end: Instant::now(),
        };

        // Disabled → no targets (the daemon loop is a no-op until opt-in).
        let off = SecurityReviewConfig::default();
        assert!(off.review_targets(&batch).is_empty());

        // Opt-in, .rs only → the two Rust files, sorted, README excluded.
        let on = SecurityReviewConfig {
            enabled: true,
            watched_suffixes: vec![".rs".into()],
            ..Default::default()
        };
        let targets = on.review_targets(&batch);
        assert_eq!(
            targets,
            vec![PathBuf::from("src/auth.rs"), PathBuf::from("src/db.rs")]
        );
    }

    #[test]
    fn parse_findings_handles_none_and_malformed() {
        let cfg = SecurityReviewConfig {
            enabled: true,
            min_severity: Severity::Info,
            ..Default::default()
        };
        assert!(parse_findings("f.rs", "NONE", &cfg).is_empty());
        assert!(parse_findings("f.rs", "garbage line\n\nfoo|bar", &cfg).is_empty());
    }
}
