//! BugBot — automated PR/diff review using LLM analysis.
//!
//! Usage:
//! - `vibecli --bugbot --diff` — review staged changes
//! - `vibecli --bugbot --pr 123` — review GitHub PR and optionally post inline comments
//! - `vibecli --bugbot --watch` — poll for new PRs and auto-review
//!
//! BugBot focuses on:
//! - Logic errors and off-by-one mistakes
//! - Security vulnerabilities (injection, unvalidated input, secrets in code)
//! - Missing error handling
//! - Performance issues
//! - Test coverage gaps

use anyhow::Result;
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use vibe_ai::provider::{AIProvider as LLMProvider, Message, MessageRole};

// ── Severity ──────────────────────────────────────────────────────────────────

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Severity {
    Error,
    Warning,
    Info,
}

impl std::fmt::Display for Severity {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Severity::Error   => write!(f, "error"),
            Severity::Warning => write!(f, "warning"),
            Severity::Info    => write!(f, "info"),
        }
    }
}

// ── BugReport ─────────────────────────────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BugReport {
    pub file: String,
    pub line: u32,
    pub severity: Severity,
    pub message: String,
    #[serde(default)]
    pub suggestion: Option<String>,
    /// CLI command that would fix this issue automatically.
    #[serde(default)]
    pub fix_command: Option<String>,
    /// Category: "logic" | "security" | "error-handling" | "performance" | "test-coverage"
    #[serde(default)]
    pub category: Option<String>,
}

impl BugReport {
    pub fn icon(&self) -> &'static str {
        match self.severity {
            Severity::Error   => "❌",
            Severity::Warning => "⚠️ ",
            Severity::Info    => "ℹ️ ",
        }
    }
}

// ── BugBot ────────────────────────────────────────────────────────────────────

pub struct BugBot {
    pub llm: Arc<dyn LLMProvider>,
    pub gh_token: Option<String>,
}

impl BugBot {
    pub fn new(llm: Arc<dyn LLMProvider>) -> Self {
        Self { llm, gh_token: std::env::var("GITHUB_TOKEN").ok() }
    }

    pub fn with_gh_token(mut self, token: impl Into<String>) -> Self {
        self.gh_token = Some(token.into());
        self
    }

    /// Analyze a unified diff and return bug reports.
    pub async fn review_diff(&self, diff: &str) -> Vec<BugReport> {
        if diff.trim().is_empty() { return vec![]; }

        let prompt = format!(
            r#"You are BugBot, an expert code reviewer. Analyze this diff for bugs.

Focus on:
- Logic errors and off-by-one mistakes
- Security issues (injection, unvalidated input, secrets committed)
- Missing error handling (unchecked Results, panics, nulls)
- Performance regressions
- Missing test coverage for new code

For each issue return a JSON object. Return ONLY a JSON array, no explanation:
[
  {{
    "file": "src/foo.rs",
    "line": 42,
    "severity": "error",
    "message": "Division by zero when denominator is 0",
    "suggestion": "Add a guard: if denominator == 0 {{ return Err(...) }}",
    "category": "logic"
  }}
]

Return an empty array [] if there are no issues.

Diff:
```diff
{}
```
"#,
            &diff[..diff.len().min(8000)]
        );

        let msgs = vec![Message { role: MessageRole::User, content: prompt }];

        match self.llm.chat(&msgs, None).await {
            Ok(response) => {
                let json_start = response.find('[').unwrap_or(0);
                let json_end = response.rfind(']').map(|i| i + 1).unwrap_or(response.len());
                let json_str = &response[json_start..json_end];
                serde_json::from_str::<Vec<BugReport>>(json_str).unwrap_or_default()
            }
            Err(_) => vec![],
        }
    }

    /// Get staged diff using `git diff --cached`.
    pub fn get_staged_diff(cwd: &std::path::Path) -> Result<String> {
        let output = std::process::Command::new("git")
            .args(["diff", "--cached", "--unified=5"])
            .current_dir(cwd)
            .output()?;
        Ok(String::from_utf8_lossy(&output.stdout).to_string())
    }

    /// Get uncommitted changes diff (staged + unstaged).
    pub fn get_working_diff(cwd: &std::path::Path) -> Result<String> {
        let output = std::process::Command::new("git")
            .args(["diff", "HEAD", "--unified=5"])
            .current_dir(cwd)
            .output()?;
        Ok(String::from_utf8_lossy(&output.stdout).to_string())
    }

    /// Fetch PR diff from GitHub.
    pub async fn fetch_pr_diff(&self, owner: &str, repo: &str, pr_number: u64) -> Result<String> {
        let url = format!("https://api.github.com/repos/{}/{}/pulls/{}", owner, repo, pr_number);
        let client = reqwest::Client::new();
        let mut req = client.get(&url)
            .header("Accept", "application/vnd.github.v3.diff")
            .header("User-Agent", "vibecli-bugbot/1.0");

        if let Some(token) = &self.gh_token {
            req = req.header("Authorization", format!("Bearer {}", token));
        }

        let resp = req.send().await?;
        if !resp.status().is_success() {
            anyhow::bail!("GitHub API error: {}", resp.status());
        }
        Ok(resp.text().await?)
    }

    /// Post inline review comments on a GitHub PR.
    pub async fn post_github_review(
        &self,
        owner: &str,
        repo: &str,
        pr_number: u64,
        reports: &[BugReport],
        commit_sha: &str,
    ) -> Result<()> {
        let token = self.gh_token.as_ref()
            .ok_or_else(|| anyhow::anyhow!("GITHUB_TOKEN not set"))?;

        let client = reqwest::Client::new();
        let url = format!(
            "https://api.github.com/repos/{}/{}/pulls/{}/reviews",
            owner, repo, pr_number
        );

        let comments: Vec<serde_json::Value> = reports.iter()
            .filter(|r| r.severity == Severity::Error || r.severity == Severity::Warning)
            .map(|r| {
                let mut body = format!("**{}** {}: {}", r.icon(), r.severity, r.message);
                if let Some(sug) = &r.suggestion {
                    body.push_str(&format!("\n\n💡 **Suggestion:** {}", sug));
                }
                serde_json::json!({
                    "path": r.file,
                    "line": r.line,
                    "body": body,
                })
            })
            .collect();

        if comments.is_empty() { return Ok(()); }

        let body_text = if reports.iter().any(|r| r.severity == Severity::Error) {
            "🤖 **BugBot** found issues that need attention. Please review the inline comments."
        } else {
            "🤖 **BugBot** found some warnings. See inline comments."
        };

        let payload = serde_json::json!({
            "commit_id": commit_sha,
            "body": body_text,
            "event": "COMMENT",
            "comments": comments,
        });

        let resp = client.post(&url)
            .header("Authorization", format!("Bearer {}", token))
            .header("Accept", "application/vnd.github.v3+json")
            .header("User-Agent", "vibecli-bugbot/1.0")
            .json(&payload)
            .send().await?;

        if !resp.status().is_success() {
            let err = resp.text().await?;
            anyhow::bail!("GitHub review post error: {}", err);
        }
        Ok(())
    }

    /// Format reports for terminal output.
    pub fn format_reports(reports: &[BugReport]) -> String {
        if reports.is_empty() {
            return "✅ BugBot found no issues.\n".to_string();
        }

        let errors = reports.iter().filter(|r| r.severity == Severity::Error).count();
        let warnings = reports.iter().filter(|r| r.severity == Severity::Warning).count();
        let infos = reports.iter().filter(|r| r.severity == Severity::Info).count();

        let mut out = format!(
            "\n🤖 BugBot Review: {} errors, {} warnings, {} info\n{}\n",
            errors, warnings, infos,
            "─".repeat(50)
        );

        for r in reports {
            out.push_str(&format!(
                "\n{} [{}] {}:{}\n   {}\n",
                r.icon(), r.severity, r.file, r.line, r.message
            ));
            if let Some(sug) = &r.suggestion {
                out.push_str(&format!("   💡 {}\n", sug));
            }
        }
        out.push('\n');
        out
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn format_empty_reports() {
        let output = BugBot::format_reports(&[]);
        assert!(output.contains("no issues"));
    }

    #[test]
    fn format_reports_with_issues() {
        let reports = vec![
            BugReport {
                file: "src/main.rs".to_string(),
                line: 42,
                severity: Severity::Error,
                message: "Division by zero".to_string(),
                suggestion: Some("Add guard".to_string()),
                fix_command: None,
                category: Some("logic".to_string()),
            },
        ];
        let output = BugBot::format_reports(&reports);
        assert!(output.contains("1 errors"));
        assert!(output.contains("src/main.rs:42"));
        assert!(output.contains("Add guard"));
    }

    #[test]
    fn severity_display() {
        assert_eq!(Severity::Error.to_string(), "error");
        assert_eq!(Severity::Warning.to_string(), "warning");
        assert_eq!(Severity::Info.to_string(), "info");
    }
}
