#![allow(dead_code)]
//! PR description generator — produces structured pull request titles and bodies
//! from diff stats and commit history. Matches Claude Code 1.x, Cursor 4.0,
//! and Copilot Workspace v2's PR description generation.
//!
//! Generates:
//! - A concise PR title (≤ 70 chars) from the most significant commit
//! - Summary bullet points (what changed and why)
//! - Test plan checklist
//! - Reviewer hints for high-risk changes

use std::collections::HashMap;

// ---------------------------------------------------------------------------
// Types
// ---------------------------------------------------------------------------

/// Input context for PR description generation.
#[derive(Debug, Clone)]
pub struct PrContext {
    pub branch_name: String,
    pub base_branch: String,
    /// (hash, conventional commit message)
    pub commits: Vec<(String, String)>,
    /// file path → (added_lines, removed_lines)
    pub diff_stats: HashMap<String, (usize, usize)>,
    pub linked_issues: Vec<String>,
    pub author: Option<String>,
}

impl PrContext {
    pub fn new(branch: impl Into<String>, base: impl Into<String>) -> Self {
        Self {
            branch_name: branch.into(),
            base_branch: base.into(),
            commits: Vec::new(),
            diff_stats: HashMap::new(),
            linked_issues: Vec::new(),
            author: None,
        }
    }

    pub fn total_added(&self) -> usize { self.diff_stats.values().map(|(a, _)| a).sum() }
    pub fn total_removed(&self) -> usize { self.diff_stats.values().map(|(_, r)| r).sum() }
    pub fn files_changed(&self) -> usize { self.diff_stats.len() }

    pub fn is_large_pr(&self) -> bool {
        self.files_changed() > 20 || self.total_added() + self.total_removed() > 500
    }

    pub fn touches_sensitive_files(&self) -> bool {
        let sensitive = ["auth", "security", "crypto", "password", "token", "secret", "key", "permission"];
        self.diff_stats.keys().any(|f| sensitive.iter().any(|s| f.contains(s)))
    }
}

/// Generated PR description.
#[derive(Debug)]
pub struct PrDescription {
    pub title: String,
    pub body: String,
    pub labels: Vec<String>,
    pub reviewer_hints: Vec<String>,
}

// ---------------------------------------------------------------------------
// Generator
// ---------------------------------------------------------------------------

pub struct PrDescriptionGenerator {
    pub max_title_length: usize,
    pub include_test_plan: bool,
    pub include_reviewer_hints: bool,
}

impl Default for PrDescriptionGenerator {
    fn default() -> Self {
        Self {
            max_title_length: 70,
            include_test_plan: true,
            include_reviewer_hints: true,
        }
    }
}

impl PrDescriptionGenerator {
    pub fn new() -> Self { Self::default() }

    pub fn generate(&self, ctx: &PrContext) -> PrDescription {
        let title = self.generate_title(ctx);
        let body = self.generate_body(ctx);
        let labels = self.infer_labels(ctx);
        let reviewer_hints = if self.include_reviewer_hints {
            self.generate_reviewer_hints(ctx)
        } else {
            Vec::new()
        };

        PrDescription { title, body, labels, reviewer_hints }
    }

    fn generate_title(&self, ctx: &PrContext) -> String {
        // Prefer first feat or fix commit; fall back to branch name
        let from_commit = ctx.commits.iter().find_map(|(_, msg)| {
            let lower = msg.to_lowercase();
            if lower.starts_with("feat") || lower.starts_with("fix") {
                let desc = msg.find(':').map(|i| msg[i + 1..].trim().to_string());
                desc.filter(|s| !s.is_empty())
            } else {
                None
            }
        });

        let raw = from_commit.unwrap_or_else(|| {
            // Convert branch name to title: "fix/auth-token-expiry" → "Fix auth token expiry"
            let name = ctx.branch_name
                .trim_start_matches("feat/")
                .trim_start_matches("fix/")
                .trim_start_matches("chore/")
                .replace('-', " ")
                .replace('_', " ");
            let mut chars = name.chars();
            match chars.next() {
                None => String::new(),
                Some(c) => c.to_uppercase().to_string() + chars.as_str(),
            }
        });

        // Truncate to max_title_length
        if raw.len() <= self.max_title_length {
            raw
        } else {
            format!("{}...", &raw[..self.max_title_length.saturating_sub(3)])
        }
    }

    fn generate_body(&self, ctx: &PrContext) -> String {
        let mut out = String::new();
        out.push_str("## Summary\n\n");

        // Bullet points from commits (deduplicated)
        let notable: Vec<String> = ctx.commits.iter()
            .filter_map(|(_, msg)| {
                let lower = msg.to_lowercase();
                if lower.starts_with("feat") || lower.starts_with("fix") || lower.starts_with("perf") {
                    msg.find(':').map(|i| format!("- {}", msg[i + 1..].trim()))
                } else {
                    None
                }
            })
            .take(5)
            .collect();

        if notable.is_empty() {
            out.push_str("- See diff for details\n");
        } else {
            for point in &notable {
                out.push_str(point);
                out.push('\n');
            }
        }

        // Stats
        out.push_str(&format!(
            "\n**Changes**: {} files, +{} −{}\n",
            ctx.files_changed(), ctx.total_added(), ctx.total_removed()
        ));

        // Linked issues
        if !ctx.linked_issues.is_empty() {
            out.push_str("\n**Closes**: ");
            out.push_str(&ctx.linked_issues.join(", "));
            out.push('\n');
        }

        // Breaking changes
        let breaking: Vec<&str> = ctx.commits.iter()
            .filter(|(_, m)| m.contains("!:") || m.contains("BREAKING CHANGE"))
            .map(|(_, m)| m.as_str())
            .collect();
        if !breaking.is_empty() {
            out.push_str("\n⚠ **Breaking changes** — review carefully before merging.\n");
        }

        // Test plan
        if self.include_test_plan {
            out.push_str("\n## Test plan\n\n");
            out.push_str("- [ ] Unit tests pass (`cargo test`)\n");
            out.push_str("- [ ] Tested the happy path manually\n");
            if ctx.touches_sensitive_files() {
                out.push_str("- [ ] Security: reviewed auth/permission logic\n");
            }
            if ctx.is_large_pr() {
                out.push_str("- [ ] Large PR: consider splitting into smaller PRs\n");
            }
        }

        out.push_str("\n🤖 Generated with [VibeCLI](https://github.com/vibecody/vibe)\n");
        out
    }

    fn infer_labels(&self, ctx: &PrContext) -> Vec<String> {
        let mut labels = Vec::new();
        let commit_types: Vec<&str> = ctx.commits.iter().map(|(_, m)| m.as_str()).collect();

        if commit_types.iter().any(|m| m.to_lowercase().starts_with("feat")) {
            labels.push("enhancement".into());
        }
        if commit_types.iter().any(|m| m.to_lowercase().starts_with("fix")) {
            labels.push("bug".into());
        }
        if commit_types.iter().any(|m| m.contains("!:") || m.contains("BREAKING")) {
            labels.push("breaking-change".into());
        }
        if ctx.is_large_pr() {
            labels.push("large-pr".into());
        }
        if ctx.touches_sensitive_files() {
            labels.push("security".into());
        }
        labels
    }

    fn generate_reviewer_hints(&self, ctx: &PrContext) -> Vec<String> {
        let mut hints = Vec::new();
        if ctx.is_large_pr() {
            hints.push("This is a large PR. Consider reviewing by file area rather than all at once.".into());
        }
        if ctx.touches_sensitive_files() {
            hints.push("Touches auth/security files. Prioritise security review.".into());
        }
        if ctx.commits.iter().any(|(_, m)| m.contains("!:") || m.contains("BREAKING")) {
            hints.push("Contains breaking changes. Verify downstream compatibility.".into());
        }
        hints
    }
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    fn basic_ctx() -> PrContext {
        let mut ctx = PrContext::new("feat/dark-mode", "main");
        ctx.commits = vec![
            ("abc1234".into(), "feat(ui): add dark mode toggle".into()),
            ("def5678".into(), "fix: prevent theme flash on load".into()),
        ];
        ctx.diff_stats.insert("src/App.tsx".into(), (50, 20));
        ctx.diff_stats.insert("src/theme.ts".into(), (80, 10));
        ctx
    }

    #[test]
    fn test_title_from_feat_commit() {
        let gen = PrDescriptionGenerator::new();
        let pr = gen.generate(&basic_ctx());
        assert!(pr.title.contains("dark mode toggle"));
        assert!(pr.title.len() <= 70);
    }

    #[test]
    fn test_title_from_branch_name() {
        let mut ctx = PrContext::new("fix/auth-token-expiry", "main");
        ctx.diff_stats.insert("auth.rs".into(), (10, 5));
        let gen = PrDescriptionGenerator::new();
        let pr = gen.generate(&ctx);
        assert!(pr.title.to_lowercase().contains("auth token expiry"));
    }

    #[test]
    fn test_title_truncation() {
        let mut ctx = PrContext::new("main", "main");
        ctx.commits = vec![("abc".into(), "feat: ".to_string() + &"x".repeat(100))];
        let gen = PrDescriptionGenerator::new();
        let pr = gen.generate(&ctx);
        assert!(pr.title.len() <= 70);
    }

    #[test]
    fn test_body_contains_summary() {
        let gen = PrDescriptionGenerator::new();
        let pr = gen.generate(&basic_ctx());
        assert!(pr.body.contains("## Summary"));
        assert!(pr.body.contains("dark mode toggle"));
    }

    #[test]
    fn test_body_contains_diff_stats() {
        let gen = PrDescriptionGenerator::new();
        let pr = gen.generate(&basic_ctx());
        assert!(pr.body.contains("2 files"));
    }

    #[test]
    fn test_body_contains_test_plan() {
        let gen = PrDescriptionGenerator::new();
        let pr = gen.generate(&basic_ctx());
        assert!(pr.body.contains("## Test plan"));
        assert!(pr.body.contains("cargo test"));
    }

    #[test]
    fn test_linked_issues() {
        let mut ctx = basic_ctx();
        ctx.linked_issues = vec!["#123".into(), "#456".into()];
        let gen = PrDescriptionGenerator::new();
        let pr = gen.generate(&ctx);
        assert!(pr.body.contains("#123"));
    }

    #[test]
    fn test_labels_feat_fix() {
        let gen = PrDescriptionGenerator::new();
        let pr = gen.generate(&basic_ctx());
        assert!(pr.labels.contains(&"enhancement".to_string()));
        assert!(pr.labels.contains(&"bug".to_string()));
    }

    #[test]
    fn test_breaking_change_label() {
        let mut ctx = PrContext::new("feat/new-api", "main");
        ctx.commits = vec![("abc".into(), "feat!: remove old API".into())];
        ctx.diff_stats.insert("src/api.rs".into(), (10, 50));
        let gen = PrDescriptionGenerator::new();
        let pr = gen.generate(&ctx);
        assert!(pr.labels.contains(&"breaking-change".to_string()));
        assert!(pr.body.contains("Breaking changes"));
    }

    #[test]
    fn test_security_hint_on_sensitive_files() {
        let mut ctx = PrContext::new("fix/auth", "main");
        ctx.commits = vec![("abc".into(), "fix: fix token refresh".into())];
        ctx.diff_stats.insert("src/auth/tokens.rs".into(), (5, 3));
        let gen = PrDescriptionGenerator::new();
        let pr = gen.generate(&ctx);
        assert!(pr.labels.contains(&"security".to_string()));
        assert!(pr.reviewer_hints.iter().any(|h| h.contains("security")));
    }

    #[test]
    fn test_large_pr_detection() {
        let mut ctx = PrContext::new("refactor/big-cleanup", "main");
        for i in 0..25 {
            ctx.diff_stats.insert(format!("file{}.rs", i), (20, 10));
        }
        assert!(ctx.is_large_pr());
        let gen = PrDescriptionGenerator::new();
        let pr = gen.generate(&ctx);
        assert!(pr.labels.contains(&"large-pr".to_string()));
    }

    #[test]
    fn test_no_test_plan_option() {
        let gen = PrDescriptionGenerator { include_test_plan: false, ..Default::default() };
        let pr = gen.generate(&basic_ctx());
        assert!(!pr.body.contains("## Test plan"));
    }
}
