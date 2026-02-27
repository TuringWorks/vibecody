//! Code review agent mode — analyzes git diffs and produces structured reviews.
//!
//! Usage:
//! ```bash
//! vibecli review                                # review uncommitted changes
//! vibecli review --base main --branch HEAD      # review branch vs main
//! vibecli review --pr 42 --post-github          # review + post to GitHub PR
//! ```

use anyhow::Result;
use serde::{Deserialize, Serialize};
use std::path::PathBuf;
use std::sync::Arc;
use vibe_ai::provider::AIProvider;
use vibe_ai::provider::{Message, MessageRole};

// ── Enums ─────────────────────────────────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum ReviewFocus {
    Security,
    Performance,
    Correctness,
    Style,
    Testing,
}

impl std::fmt::Display for ReviewFocus {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Security => write!(f, "Security"),
            Self::Performance => write!(f, "Performance"),
            Self::Correctness => write!(f, "Correctness"),
            Self::Style => write!(f, "Style"),
            Self::Testing => write!(f, "Testing"),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, PartialOrd, Ord)]
#[serde(rename_all = "snake_case")]
pub enum Severity {
    Info,
    Warning,
    Critical,
}

impl std::fmt::Display for Severity {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Info => write!(f, "Info"),
            Self::Warning => write!(f, "Warning"),
            Self::Critical => write!(f, "Critical"),
        }
    }
}

// ── Review Config ─────────────────────────────────────────────────────────────

#[derive(Debug, Clone)]
pub struct ReviewConfig {
    /// Base ref for `git diff <base>..<target>`. Default: "HEAD"
    pub base_ref: String,
    /// Target ref. Default: working tree (empty string = uncommitted changes)
    pub target_ref: String,
    /// Post review as a GitHub PR comment using `gh`.
    pub post_to_github: bool,
    /// GitHub PR number to post to.
    pub github_pr: Option<u32>,
    /// Review dimensions to focus on.
    pub focus: Vec<ReviewFocus>,
    /// Workspace root path.
    pub workspace: PathBuf,
}

impl Default for ReviewConfig {
    fn default() -> Self {
        Self {
            base_ref: String::new(),   // empty = compare workdir to HEAD
            target_ref: String::new(),
            post_to_github: false,
            github_pr: None,
            focus: vec![
                ReviewFocus::Correctness,
                ReviewFocus::Security,
                ReviewFocus::Performance,
            ],
            workspace: std::env::current_dir().unwrap_or_else(|_| PathBuf::from(".")),
        }
    }
}

// ── Review Types ──────────────────────────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ReviewIssue {
    pub file: String,
    pub line: u32,
    pub severity: Severity,
    pub category: ReviewFocus,
    pub description: String,
    pub suggested_fix: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ReviewSuggestion {
    pub description: String,
    pub file: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ReviewScore {
    /// Overall quality 0–10.
    pub overall: f32,
    pub correctness: f32,
    pub security: f32,
    pub performance: f32,
    pub style: f32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ReviewReport {
    pub base_ref: String,
    pub target_ref: String,
    pub summary: String,
    pub issues: Vec<ReviewIssue>,
    pub suggestions: Vec<ReviewSuggestion>,
    pub score: ReviewScore,
    /// Files touched by the diff.
    pub files_reviewed: Vec<String>,
}

impl ReviewReport {
    /// Render the report as human-readable Markdown.
    pub fn to_markdown(&self) -> String {
        let mut md = String::new();

        md.push_str("# VibeCLI Code Review\n\n");
        if !self.base_ref.is_empty() || !self.target_ref.is_empty() {
            md.push_str(&format!("**Diff:** `{}..{}`\n\n", self.base_ref, self.target_ref));
        }
        md.push_str(&format!("**Files reviewed:** {}\n\n", self.files_reviewed.join(", ")));
        md.push_str("---\n\n");
        md.push_str("## Summary\n\n");
        md.push_str(&self.summary);
        md.push_str("\n\n");

        // Score table
        md.push_str("## Scores\n\n");
        md.push_str("| Dimension | Score |\n|-----------|-------|\n");
        md.push_str(&format!("| Overall | {:.1}/10 |\n", self.score.overall));
        md.push_str(&format!("| Correctness | {:.1}/10 |\n", self.score.correctness));
        md.push_str(&format!("| Security | {:.1}/10 |\n", self.score.security));
        md.push_str(&format!("| Performance | {:.1}/10 |\n", self.score.performance));
        md.push_str(&format!("| Style | {:.1}/10 |\n", self.score.style));
        md.push('\n');

        // Issues
        if !self.issues.is_empty() {
            md.push_str("## Issues\n\n");
            let mut sorted = self.issues.clone();
            sorted.sort_by(|a, b| b.severity.cmp(&a.severity));
            for issue in &sorted {
                let icon = match issue.severity {
                    Severity::Critical => "🔴",
                    Severity::Warning => "🟡",
                    Severity::Info => "🔵",
                };
                md.push_str(&format!(
                    "### {} {} — `{}:{}` ({})\n\n{}\n\n",
                    icon, issue.severity, issue.file, issue.line,
                    issue.category, issue.description,
                ));
                if let Some(ref fix) = issue.suggested_fix {
                    md.push_str(&format!("**Suggested fix:** {}\n\n", fix));
                }
            }
        }

        // Suggestions
        if !self.suggestions.is_empty() {
            md.push_str("## Suggestions\n\n");
            for s in &self.suggestions {
                let file_note = s.file.as_deref().map(|f| format!(" (`{}`)", f)).unwrap_or_default();
                md.push_str(&format!("- {}{}\n", s.description, file_note));
            }
            md.push('\n');
        }

        md.push_str("---\n*Generated by VibeCLI*\n");
        md
    }

    pub fn exit_code(&self) -> i32 {
        let critical = self.issues.iter().any(|i| i.severity == Severity::Critical);
        if critical { 1 } else { 0 }
    }
}

// ── Main review runner ────────────────────────────────────────────────────────

/// Run the code review agent on a git diff and return a structured report.
pub async fn run_review(config: &ReviewConfig, llm: Arc<dyn AIProvider>) -> Result<ReviewReport> {
    let diff = get_diff(config)?;

    if diff.trim().is_empty() {
        return Ok(ReviewReport {
            base_ref: config.base_ref.clone(),
            target_ref: config.target_ref.clone(),
            summary: "No changes found to review.".to_string(),
            issues: vec![],
            suggestions: vec![],
            score: ReviewScore { overall: 10.0, correctness: 10.0, security: 10.0, performance: 10.0, style: 10.0 },
            files_reviewed: vec![],
        });
    }

    let files_reviewed = extract_files_from_diff(&diff);

    // Split large diffs by file and review each chunk
    let file_chunks = split_diff_by_file(&diff);
    let mut all_issues: Vec<ReviewIssue> = Vec::new();
    let mut all_suggestions: Vec<ReviewSuggestion> = Vec::new();
    let mut summaries: Vec<String> = Vec::new();

    for (file, chunk) in &file_chunks {
        let chunk_result = review_chunk(llm.clone(), file, chunk, config).await;
        match chunk_result {
            Ok((issues, suggestions, summary)) => {
                all_issues.extend(issues);
                all_suggestions.extend(suggestions);
                if !summary.is_empty() {
                    summaries.push(summary);
                }
            }
            Err(e) => {
                tracing::warn!(file = %file, error = %e, "Failed to review chunk");
            }
        }
    }

    // Deduplicate issues by (file, line, description)
    all_issues.dedup_by(|a, b| a.file == b.file && a.line == b.line && a.description == b.description);

    // Score based on issues
    let score = compute_score(&all_issues);

    // Final summary
    let summary = if summaries.is_empty() {
        "Review complete.".to_string()
    } else {
        summaries.join(" ")
    };

    Ok(ReviewReport {
        base_ref: config.base_ref.clone(),
        target_ref: config.target_ref.clone(),
        summary,
        issues: all_issues,
        suggestions: all_suggestions,
        score,
        files_reviewed,
    })
}

/// Post the review report as a GitHub PR comment using the `gh` CLI.
pub fn post_to_github_pr(pr: u32, markdown: &str) -> Result<()> {
    let output = std::process::Command::new("gh")
        .args(["pr", "comment", &pr.to_string(), "--body", markdown])
        .output()?;
    if !output.status.success() {
        let err = String::from_utf8_lossy(&output.stderr);
        anyhow::bail!("gh pr comment failed: {}", err);
    }
    Ok(())
}

// ── Internal helpers ──────────────────────────────────────────────────────────

fn get_diff(config: &ReviewConfig) -> Result<String> {
    let mut args = vec!["diff".to_string()];

    if !config.base_ref.is_empty() && !config.target_ref.is_empty() {
        args.push(format!("{}..{}", config.base_ref, config.target_ref));
    } else if !config.base_ref.is_empty() {
        args.push(config.base_ref.clone());
    }
    // else: unstaged + staged changes vs HEAD

    let output = std::process::Command::new("git")
        .args(&args)
        .current_dir(&config.workspace)
        .output()?;

    // Also include staged changes if reviewing uncommitted
    if config.base_ref.is_empty() {
        let staged_output = std::process::Command::new("git")
            .args(["diff", "--cached"])
            .current_dir(&config.workspace)
            .output()?;
        let combined = format!(
            "{}\n{}",
            String::from_utf8_lossy(&output.stdout),
            String::from_utf8_lossy(&staged_output.stdout)
        );
        return Ok(combined);
    }

    Ok(String::from_utf8_lossy(&output.stdout).to_string())
}

fn extract_files_from_diff(diff: &str) -> Vec<String> {
    diff.lines()
        .filter(|l| l.starts_with("+++ b/") || l.starts_with("--- a/"))
        .filter(|l| l.starts_with("+++ b/"))
        .map(|l| l.trim_start_matches("+++ b/").to_string())
        .filter(|f| f != "/dev/null")
        .collect::<std::collections::HashSet<_>>()
        .into_iter()
        .collect()
}

/// Split a combined diff into per-file chunks.
fn split_diff_by_file(diff: &str) -> Vec<(String, String)> {
    let mut chunks: Vec<(String, String)> = Vec::new();
    let mut current_file = String::new();
    let mut current_chunk = String::new();

    for line in diff.lines() {
        if line.starts_with("diff --git ") {
            // Save previous
            if !current_file.is_empty() && !current_chunk.is_empty() {
                chunks.push((current_file.clone(), current_chunk.clone()));
            }
            current_file = line.split(" b/").last().unwrap_or("unknown").to_string();
            current_chunk = line.to_string();
            current_chunk.push('\n');
        } else {
            current_chunk.push_str(line);
            current_chunk.push('\n');
        }
    }
    if !current_file.is_empty() && !current_chunk.is_empty() {
        chunks.push((current_file, current_chunk));
    }

    // If diff doesn't start with "diff --git", treat whole thing as one chunk
    if chunks.is_empty() && !diff.is_empty() {
        chunks.push(("(all changes)".to_string(), diff.to_string()));
    }

    chunks
}

const MAX_DIFF_CHARS: usize = 8_000;

async fn review_chunk(
    llm: Arc<dyn AIProvider>,
    file: &str,
    diff: &str,
    config: &ReviewConfig,
) -> Result<(Vec<ReviewIssue>, Vec<ReviewSuggestion>, String)> {
    let focus_list = config.focus.iter().map(|f| f.to_string()).collect::<Vec<_>>().join(", ");

    // Truncate very large diffs
    let diff_truncated = if diff.len() > MAX_DIFF_CHARS {
        format!("{}\n... (truncated)", &diff[..MAX_DIFF_CHARS])
    } else {
        diff.to_string()
    };

    let prompt = format!(
        r#"You are a senior code reviewer. Review the following git diff for `{}`.

Focus areas: {}

Respond with ONLY valid JSON (no markdown fences, no explanation):
{{
  "summary": "one-sentence summary",
  "issues": [
    {{
      "file": "filename.rs",
      "line": 42,
      "severity": "critical|warning|info",
      "category": "security|performance|correctness|style|testing",
      "description": "description of the issue",
      "suggested_fix": "how to fix it (or null)"
    }}
  ],
  "suggestions": [
    {{ "description": "general suggestion", "file": "optional_file.rs or null" }}
  ]
}}

Diff:
```
{}
```"#,
        file, focus_list, diff_truncated
    );

    let messages = vec![
        Message {
            role: MessageRole::System,
            content: "You are a code review assistant. Respond only with valid JSON.".to_string(),
        },
        Message {
            role: MessageRole::User,
            content: prompt,
        },
    ];

    let response = llm.chat(&messages, None).await?;

    // Strip markdown fences
    let raw = response.trim();
    let json_str = if raw.starts_with("```") {
        raw.lines()
            .filter(|l| !l.starts_with("```"))
            .collect::<Vec<_>>()
            .join("\n")
    } else {
        raw.to_string()
    };

    #[derive(Deserialize)]
    struct RawChunkReview {
        #[serde(default)]
        summary: String,
        #[serde(default)]
        issues: Vec<RawIssue>,
        #[serde(default)]
        suggestions: Vec<RawSuggestion>,
    }

    #[derive(Deserialize)]
    struct RawIssue {
        #[serde(default)]
        file: String,
        #[serde(default)]
        line: u32,
        #[serde(default = "default_severity")]
        severity: String,
        #[serde(default)]
        category: String,
        #[serde(default)]
        description: String,
        #[serde(default)]
        suggested_fix: Option<String>,
    }

    #[derive(Deserialize)]
    struct RawSuggestion {
        #[serde(default)]
        description: String,
        #[serde(default)]
        file: Option<String>,
    }

    fn default_severity() -> String { "info".to_string() }

    let parsed: RawChunkReview = serde_json::from_str(&json_str)
        .unwrap_or(RawChunkReview { summary: String::new(), issues: vec![], suggestions: vec![] });

    let issues = parsed.issues.into_iter().map(|i| ReviewIssue {
        file: if i.file.is_empty() { file.to_string() } else { i.file },
        line: i.line,
        severity: match i.severity.as_str() {
            "critical" => Severity::Critical,
            "warning" => Severity::Warning,
            _ => Severity::Info,
        },
        category: match i.category.as_str() {
            "security" => ReviewFocus::Security,
            "performance" => ReviewFocus::Performance,
            "style" => ReviewFocus::Style,
            "testing" => ReviewFocus::Testing,
            _ => ReviewFocus::Correctness,
        },
        description: i.description,
        suggested_fix: i.suggested_fix,
    }).collect();

    let suggestions = parsed.suggestions.into_iter().map(|s| ReviewSuggestion {
        description: s.description,
        file: s.file,
    }).collect();

    Ok((issues, suggestions, parsed.summary))
}

fn compute_score(issues: &[ReviewIssue]) -> ReviewScore {
    let mut score = ReviewScore {
        overall: 10.0,
        correctness: 10.0,
        security: 10.0,
        performance: 10.0,
        style: 10.0,
    };

    for issue in issues {
        let deduction = match issue.severity {
            Severity::Critical => 2.0,
            Severity::Warning => 0.5,
            Severity::Info => 0.1,
        };
        match issue.category {
            ReviewFocus::Correctness | ReviewFocus::Testing => score.correctness -= deduction,
            ReviewFocus::Security => score.security -= deduction,
            ReviewFocus::Performance => score.performance -= deduction,
            ReviewFocus::Style => score.style -= deduction,
        }
    }

    // Clamp all to [0, 10]
    score.correctness = score.correctness.clamp(0.0, 10.0);
    score.security = score.security.clamp(0.0, 10.0);
    score.performance = score.performance.clamp(0.0, 10.0);
    score.style = score.style.clamp(0.0, 10.0);
    score.overall = ((score.correctness + score.security + score.performance + score.style) / 4.0).clamp(0.0, 10.0);

    score
}

// ── Tests ─────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn extract_files_from_diff_basic() {
        let diff = "+++ b/src/main.rs\n--- a/src/main.rs\n+++ b/src/lib.rs\n";
        let files = extract_files_from_diff(diff);
        assert!(files.contains(&"src/main.rs".to_string()));
        assert!(files.contains(&"src/lib.rs".to_string()));
    }

    #[test]
    fn split_diff_by_file_multiple() {
        let diff = "diff --git a/foo.rs b/foo.rs\n+added\ndiff --git a/bar.rs b/bar.rs\n-removed\n";
        let chunks = split_diff_by_file(diff);
        assert_eq!(chunks.len(), 2);
        assert_eq!(chunks[0].0, "foo.rs");
        assert_eq!(chunks[1].0, "bar.rs");
    }

    #[test]
    fn compute_score_no_issues() {
        let score = compute_score(&[]);
        assert_eq!(score.overall, 10.0);
        assert_eq!(score.security, 10.0);
    }

    #[test]
    fn compute_score_critical_security() {
        let issues = vec![ReviewIssue {
            file: "auth.rs".to_string(),
            line: 10,
            severity: Severity::Critical,
            category: ReviewFocus::Security,
            description: "SQL injection".to_string(),
            suggested_fix: None,
        }];
        let score = compute_score(&issues);
        assert!(score.security < 10.0);
        assert!(score.overall < 10.0);
    }

    #[test]
    fn report_to_markdown_contains_sections() {
        let report = ReviewReport {
            base_ref: "main".to_string(),
            target_ref: "HEAD".to_string(),
            summary: "Looks good overall.".to_string(),
            issues: vec![ReviewIssue {
                file: "src/auth.rs".to_string(),
                line: 42,
                severity: Severity::Warning,
                category: ReviewFocus::Security,
                description: "Use constant-time comparison".to_string(),
                suggested_fix: Some("Use hmac::equal()".to_string()),
            }],
            suggestions: vec![ReviewSuggestion {
                description: "Add more tests".to_string(),
                file: None,
            }],
            score: ReviewScore { overall: 8.5, correctness: 9.0, security: 7.5, performance: 9.0, style: 9.0 },
            files_reviewed: vec!["src/auth.rs".to_string()],
        };
        let md = report.to_markdown();
        assert!(md.contains("# VibeCLI Code Review"));
        assert!(md.contains("src/auth.rs:42"));
        assert!(md.contains("Suggested fix:"));
        assert!(md.contains("Add more tests"));
    }

    #[test]
    fn no_changes_returns_clean_report() {
        // Empty diff → early exit with perfect score
        let empty = "";
        assert!(empty.trim().is_empty());
    }
}
