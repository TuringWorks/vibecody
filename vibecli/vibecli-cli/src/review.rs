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
            current_file = line.split(" b/").nth(1).unwrap_or("unknown").to_string();
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
        let end = diff.char_indices().nth(MAX_DIFF_CHARS).map(|(i, _)| i).unwrap_or(diff.len());
        format!("{}\n... (truncated)", &diff[..end])
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

    // ── split_diff_by_file tests ─────────────────────────────────────────────

    #[test]
    fn split_diff_by_file_empty_diff() {
        let chunks = split_diff_by_file("");
        assert!(chunks.is_empty());
    }

    #[test]
    fn split_diff_by_file_no_git_header() {
        // Diff without "diff --git" header falls back to single "(all changes)" chunk.
        let diff = "+++ b/foo.rs\n-old line\n+new line\n";
        let chunks = split_diff_by_file(diff);
        assert_eq!(chunks.len(), 1);
        assert_eq!(chunks[0].0, "(all changes)");
        assert!(chunks[0].1.contains("+new line"));
    }

    #[test]
    fn split_diff_by_file_single_file() {
        let diff = "diff --git a/src/main.rs b/src/main.rs\nindex abc..def 100644\n--- a/src/main.rs\n+++ b/src/main.rs\n@@ -1 +1 @@\n-old\n+new\n";
        let chunks = split_diff_by_file(diff);
        assert_eq!(chunks.len(), 1);
        assert_eq!(chunks[0].0, "src/main.rs");
        assert!(chunks[0].1.contains("-old"));
        assert!(chunks[0].1.contains("+new"));
    }

    #[test]
    fn split_diff_by_file_three_files() {
        let diff = "\
diff --git a/a.rs b/a.rs\n+a content\n\
diff --git a/b.rs b/b.rs\n+b content\n\
diff --git a/c.rs b/c.rs\n+c content\n";
        let chunks = split_diff_by_file(diff);
        assert_eq!(chunks.len(), 3);
        assert_eq!(chunks[0].0, "a.rs");
        assert_eq!(chunks[1].0, "b.rs");
        assert_eq!(chunks[2].0, "c.rs");
    }

    // ── compute_score tests ──────────────────────────────────────────────────

    #[test]
    fn compute_score_all_critical_clamps_to_zero() {
        // 6 critical security issues = 6 * 2.0 = 12.0 deduction, clamped to 0.
        let issues: Vec<ReviewIssue> = (0..6)
            .map(|i| ReviewIssue {
                file: "vuln.rs".into(),
                line: i,
                severity: Severity::Critical,
                category: ReviewFocus::Security,
                description: format!("critical issue {}", i),
                suggested_fix: None,
            })
            .collect();
        let score = compute_score(&issues);
        assert_eq!(score.security, 0.0);
        // Other dimensions untouched
        assert_eq!(score.correctness, 10.0);
        assert_eq!(score.performance, 10.0);
        assert_eq!(score.style, 10.0);
        // Overall = (10 + 0 + 10 + 10) / 4 = 7.5
        assert!((score.overall - 7.5).abs() < 0.01);
    }

    #[test]
    fn compute_score_mix_of_severities() {
        let issues = vec![
            ReviewIssue {
                file: "a.rs".into(), line: 1,
                severity: Severity::Critical, category: ReviewFocus::Correctness,
                description: "bug".into(), suggested_fix: None,
            },
            ReviewIssue {
                file: "b.rs".into(), line: 2,
                severity: Severity::Warning, category: ReviewFocus::Style,
                description: "naming".into(), suggested_fix: None,
            },
            ReviewIssue {
                file: "c.rs".into(), line: 3,
                severity: Severity::Info, category: ReviewFocus::Performance,
                description: "minor".into(), suggested_fix: None,
            },
        ];
        let score = compute_score(&issues);
        assert!((score.correctness - 8.0).abs() < 0.01); // 10 - 2.0
        assert!((score.style - 9.5).abs() < 0.01);       // 10 - 0.5
        assert!((score.performance - 9.9).abs() < 0.01);  // 10 - 0.1
        assert_eq!(score.security, 10.0);                 // untouched
    }

    #[test]
    fn compute_score_single_warning() {
        let issues = vec![ReviewIssue {
            file: "x.rs".into(), line: 5,
            severity: Severity::Warning, category: ReviewFocus::Performance,
            description: "slow".into(), suggested_fix: None,
        }];
        let score = compute_score(&issues);
        assert!((score.performance - 9.5).abs() < 0.01);
        assert_eq!(score.correctness, 10.0);
        assert_eq!(score.security, 10.0);
        assert_eq!(score.style, 10.0);
    }

    // ── exit_code tests ──────────────────────────────────────────────────────

    #[test]
    fn exit_code_zero_when_no_critical() {
        let report = ReviewReport {
            base_ref: String::new(), target_ref: String::new(),
            summary: "ok".into(),
            issues: vec![
                ReviewIssue {
                    file: "a.rs".into(), line: 1,
                    severity: Severity::Warning, category: ReviewFocus::Style,
                    description: "meh".into(), suggested_fix: None,
                },
                ReviewIssue {
                    file: "b.rs".into(), line: 2,
                    severity: Severity::Info, category: ReviewFocus::Correctness,
                    description: "note".into(), suggested_fix: None,
                },
            ],
            suggestions: vec![],
            score: ReviewScore { overall: 9.0, correctness: 9.0, security: 10.0, performance: 10.0, style: 9.0 },
            files_reviewed: vec![],
        };
        assert_eq!(report.exit_code(), 0);
    }

    #[test]
    fn exit_code_one_when_critical_present() {
        let report = ReviewReport {
            base_ref: String::new(), target_ref: String::new(),
            summary: "bad".into(),
            issues: vec![ReviewIssue {
                file: "a.rs".into(), line: 1,
                severity: Severity::Critical, category: ReviewFocus::Security,
                description: "vuln".into(), suggested_fix: None,
            }],
            suggestions: vec![],
            score: ReviewScore { overall: 5.0, correctness: 10.0, security: 5.0, performance: 10.0, style: 10.0 },
            files_reviewed: vec![],
        };
        assert_eq!(report.exit_code(), 1);
    }

    // ── to_markdown tests ────────────────────────────────────────────────────

    #[test]
    fn to_markdown_contains_expected_sections() {
        let report = ReviewReport {
            base_ref: "develop".into(), target_ref: "feature".into(),
            summary: "Overall the change is good.".into(),
            issues: vec![],
            suggestions: vec![ReviewSuggestion { description: "Add docs".into(), file: Some("lib.rs".into()) }],
            score: ReviewScore { overall: 10.0, correctness: 10.0, security: 10.0, performance: 10.0, style: 10.0 },
            files_reviewed: vec!["lib.rs".into(), "main.rs".into()],
        };
        let md = report.to_markdown();
        assert!(md.contains("# VibeCLI Code Review"));
        assert!(md.contains("**Diff:** `develop..feature`"));
        assert!(md.contains("lib.rs, main.rs") || md.contains("main.rs, lib.rs"));
        assert!(md.contains("## Summary"));
        assert!(md.contains("## Scores"));
        assert!(md.contains("10.0/10"));
        assert!(md.contains("## Suggestions"));
        assert!(md.contains("Add docs (`lib.rs`)"));
        assert!(md.contains("*Generated by VibeCLI*"));
        // No Issues section when there are no issues
        assert!(!md.contains("## Issues"));
    }

    // ── extract_files_from_diff tests ────────────────────────────────────────

    #[test]
    fn extract_files_from_diff_ignores_dev_null() {
        let diff = "+++ b/new_file.rs\n+++ b//dev/null\n";
        let files = extract_files_from_diff(diff);
        assert!(files.contains(&"new_file.rs".to_string()));
        // /dev/null is filtered out
        assert!(!files.iter().any(|f| f.contains("dev/null")));
    }

    #[test]
    fn extract_files_from_diff_deduplicates() {
        let diff = "+++ b/src/foo.rs\n+++ b/src/foo.rs\n+++ b/src/bar.rs\n";
        let files = extract_files_from_diff(diff);
        let foo_count = files.iter().filter(|f| *f == "src/foo.rs").count();
        assert_eq!(foo_count, 1);
        assert_eq!(files.len(), 2);
    }

    #[test]
    fn extract_files_from_diff_empty() {
        let files = extract_files_from_diff("");
        assert!(files.is_empty());
    }

    // ── Display trait tests ──────────────────────────────────────────────────

    #[test]
    fn review_focus_display_all_variants() {
        assert_eq!(format!("{}", ReviewFocus::Security), "Security");
        assert_eq!(format!("{}", ReviewFocus::Performance), "Performance");
        assert_eq!(format!("{}", ReviewFocus::Correctness), "Correctness");
        assert_eq!(format!("{}", ReviewFocus::Style), "Style");
        assert_eq!(format!("{}", ReviewFocus::Testing), "Testing");
    }

    #[test]
    fn severity_display_and_ordering() {
        assert_eq!(format!("{}", Severity::Info), "Info");
        assert_eq!(format!("{}", Severity::Warning), "Warning");
        assert_eq!(format!("{}", Severity::Critical), "Critical");
        // Ordering: Info < Warning < Critical
        assert!(Severity::Info < Severity::Warning);
        assert!(Severity::Warning < Severity::Critical);
        assert!(Severity::Info < Severity::Critical);
    }

    // ── ReviewConfig default tests ───────────────────────────────────────────

    #[test]
    fn review_config_default_values() {
        let cfg = ReviewConfig::default();
        assert_eq!(cfg.base_ref, "");
        assert_eq!(cfg.target_ref, "");
        assert!(!cfg.post_to_github);
        assert!(cfg.github_pr.is_none());
        assert_eq!(cfg.focus.len(), 3);
        assert!(cfg.focus.contains(&ReviewFocus::Correctness));
        assert!(cfg.focus.contains(&ReviewFocus::Security));
        assert!(cfg.focus.contains(&ReviewFocus::Performance));
        // Style and Testing are NOT in default focus
        assert!(!cfg.focus.contains(&ReviewFocus::Style));
        assert!(!cfg.focus.contains(&ReviewFocus::Testing));
    }

    // ── Serde roundtrip tests ────────────────────────────────────────────────

    #[test]
    fn review_issue_serde_roundtrip() {
        let issue = ReviewIssue {
            file: "src/auth.rs".into(),
            line: 42,
            severity: Severity::Critical,
            category: ReviewFocus::Security,
            description: "SQL injection vulnerability".into(),
            suggested_fix: Some("Use parameterized queries".into()),
        };
        let json = serde_json::to_string(&issue).unwrap();
        let deserialized: ReviewIssue = serde_json::from_str(&json).unwrap();
        assert_eq!(deserialized.file, "src/auth.rs");
        assert_eq!(deserialized.line, 42);
        assert_eq!(deserialized.severity, Severity::Critical);
        assert_eq!(deserialized.category, ReviewFocus::Security);
        assert_eq!(deserialized.description, "SQL injection vulnerability");
        assert_eq!(deserialized.suggested_fix.as_deref(), Some("Use parameterized queries"));
    }

    #[test]
    fn review_suggestion_serde_roundtrip() {
        let suggestion = ReviewSuggestion {
            description: "Consider adding integration tests".into(),
            file: Some("tests/".into()),
        };
        let json = serde_json::to_string(&suggestion).unwrap();
        let deserialized: ReviewSuggestion = serde_json::from_str(&json).unwrap();
        assert_eq!(deserialized.description, "Consider adding integration tests");
        assert_eq!(deserialized.file.as_deref(), Some("tests/"));

        // Also test with None file
        let suggestion_no_file = ReviewSuggestion {
            description: "Use clippy".into(),
            file: None,
        };
        let json2 = serde_json::to_string(&suggestion_no_file).unwrap();
        let deserialized2: ReviewSuggestion = serde_json::from_str(&json2).unwrap();
        assert!(deserialized2.file.is_none());
    }

    #[test]
    fn review_score_serde_roundtrip() {
        let score = ReviewScore {
            overall: 8.5,
            correctness: 9.0,
            security: 7.5,
            performance: 9.0,
            style: 8.0,
        };
        let json = serde_json::to_string(&score).unwrap();
        let deserialized: ReviewScore = serde_json::from_str(&json).unwrap();
        assert!((deserialized.overall - 8.5).abs() < f32::EPSILON);
        assert!((deserialized.correctness - 9.0).abs() < f32::EPSILON);
        assert!((deserialized.security - 7.5).abs() < f32::EPSILON);
        assert!((deserialized.performance - 9.0).abs() < f32::EPSILON);
        assert!((deserialized.style - 8.0).abs() < f32::EPSILON);
    }
}
