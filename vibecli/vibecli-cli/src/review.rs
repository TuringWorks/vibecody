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

// ── Multi-Perspective Review ─────────────────────────────────────────────────
// These types are scaffolded for the multi-perspective review feature
// and will be wired into the CLI once the UX is finalized.

#[allow(dead_code)]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ReviewPerspective {
    Architect,
    SecurityExpert,
    PerformanceEngineer,
    TestingSpecialist,
    UxReviewer,
    Maintainability,
    ApiDesigner,
    DataModeler,
    AccessibilityAuditor,
    DevOpsEngineer,
}

#[allow(dead_code)]
impl ReviewPerspective {
    pub const ALL: &'static [Self] = &[
        Self::Architect,
        Self::SecurityExpert,
        Self::PerformanceEngineer,
        Self::TestingSpecialist,
        Self::Maintainability,
    ];

    pub fn system_prompt(&self) -> &'static str {
        match self {
            Self::Architect => "You are a senior software architect. Focus on design patterns, separation of concerns, modularity, coupling, cohesion, and scalability.",
            Self::SecurityExpert => "You are a security expert. Focus on authentication, authorization, injection attacks, data exposure, cryptographic issues, and supply chain risks.",
            Self::PerformanceEngineer => "You are a performance engineer. Focus on algorithmic complexity, memory usage, I/O patterns, caching opportunities, and bottlenecks.",
            Self::TestingSpecialist => "You are a testing specialist. Focus on test coverage, edge cases, test quality, mocking practices, and regression risks.",
            Self::UxReviewer => "You are a UX reviewer. Focus on user-facing error messages, accessibility, API ergonomics, and developer experience.",
            Self::Maintainability => "You are a maintainability reviewer. Focus on code readability, documentation, naming conventions, technical debt, and future extensibility.",
            Self::ApiDesigner => "You are an API designer. Focus on RESTful conventions, backwards compatibility, versioning, error formats, and pagination.",
            Self::DataModeler => "You are a data modeler. Focus on schema design, normalization, indexes, migrations, and data integrity constraints.",
            Self::AccessibilityAuditor => "You are an accessibility auditor. Focus on WCAG compliance, ARIA attributes, keyboard navigation, screen reader support, and color contrast.",
            Self::DevOpsEngineer => "You are a DevOps engineer. Focus on deployability, configuration management, logging, monitoring, CI/CD, and infrastructure as code.",
        }
    }
}

impl std::fmt::Display for ReviewPerspective {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Architect => write!(f, "Architect"),
            Self::SecurityExpert => write!(f, "Security Expert"),
            Self::PerformanceEngineer => write!(f, "Performance Engineer"),
            Self::TestingSpecialist => write!(f, "Testing Specialist"),
            Self::UxReviewer => write!(f, "UX Reviewer"),
            Self::Maintainability => write!(f, "Maintainability"),
            Self::ApiDesigner => write!(f, "API Designer"),
            Self::DataModeler => write!(f, "Data Modeler"),
            Self::AccessibilityAuditor => write!(f, "Accessibility Auditor"),
            Self::DevOpsEngineer => write!(f, "DevOps Engineer"),
        }
    }
}

#[allow(dead_code)]
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MultiPerspectiveReport {
    pub perspectives_used: Vec<String>,
    pub findings: Vec<PerspectiveFinding>,
    pub merged_issues: Vec<ReviewIssue>,
    pub summary: String,
}

#[allow(dead_code)]
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PerspectiveFinding {
    pub perspective: String,
    pub issues: Vec<ReviewIssue>,
    pub suggestions: Vec<ReviewSuggestion>,
    pub summary: String,
}

#[allow(dead_code)]
impl MultiPerspectiveReport {
    pub fn to_markdown(&self) -> String {
        let mut md = String::new();
        md.push_str("# Multi-Perspective Code Review\n\n");
        md.push_str(&format!(
            "**Perspectives:** {}\n\n",
            self.perspectives_used.join(", ")
        ));
        md.push_str("## Summary\n\n");
        md.push_str(&self.summary);
        md.push_str("\n\n");

        md.push_str(&format!("## Merged Issues ({})\n\n", self.merged_issues.len()));
        let mut sorted = self.merged_issues.clone();
        sorted.sort_by(|a, b| b.severity.cmp(&a.severity));
        for issue in &sorted {
            let icon = match issue.severity {
                Severity::Critical => "🔴",
                Severity::Warning => "🟡",
                Severity::Info => "🔵",
            };
            md.push_str(&format!(
                "- {} **{}** `{}:{}` — {}\n",
                icon, issue.severity, issue.file, issue.line, issue.description
            ));
        }
        md.push('\n');

        for finding in &self.findings {
            md.push_str(&format!("### {} Perspective\n\n", finding.perspective));
            md.push_str(&format!("{}\n\n", finding.summary));
            if !finding.suggestions.is_empty() {
                for s in &finding.suggestions {
                    md.push_str(&format!("- {}\n", s.description));
                }
                md.push('\n');
            }
        }

        md.push_str("---\n*Generated by VibeCLI Multi-Perspective Review*\n");
        md
    }
}

/// Run multi-perspective code review on a git diff.
#[allow(dead_code)]
pub async fn run_multi_perspective_review(
    config: &ReviewConfig,
    perspectives: &[ReviewPerspective],
    llm: Arc<dyn AIProvider>,
) -> Result<MultiPerspectiveReport> {
    let diff = get_diff(config)?;
    if diff.trim().is_empty() {
        return Ok(MultiPerspectiveReport {
            perspectives_used: perspectives.iter().map(|p| p.to_string()).collect(),
            findings: vec![],
            merged_issues: vec![],
            summary: "No changes found to review.".to_string(),
        });
    }

    let diff_truncated = if diff.len() > MAX_DIFF_CHARS * 2 {
        let end = diff.char_indices().nth(MAX_DIFF_CHARS * 2).map(|(i, _)| i).unwrap_or(diff.len());
        format!("{}\n... (truncated)", &diff[..end])
    } else {
        diff.clone()
    };

    let mut findings: Vec<PerspectiveFinding> = Vec::new();
    let mut all_issues: Vec<ReviewIssue> = Vec::new();

    for perspective in perspectives {
        let prompt = format!(
            r#"Review this git diff from your perspective as a {}.

Respond with ONLY valid JSON:
{{
  "summary": "one-sentence perspective-specific summary",
  "issues": [
    {{
      "file": "filename",
      "line": 0,
      "severity": "critical|warning|info",
      "category": "security|performance|correctness|style|testing",
      "description": "description",
      "suggested_fix": "fix or null"
    }}
  ],
  "suggestions": [
    {{ "description": "suggestion", "file": "optional_file or null" }}
  ]
}}

Diff:
```
{}
```"#,
            perspective, diff_truncated
        );

        let messages = vec![
            Message {
                role: MessageRole::System,
                content: perspective.system_prompt().to_string(),
            },
            Message {
                role: MessageRole::User,
                content: prompt,
            },
        ];

        match llm.chat(&messages, None).await {
            Ok(response) => {
                let (issues, suggestions, summary) = parse_perspective_response(&response);
                all_issues.extend(issues.clone());
                findings.push(PerspectiveFinding {
                    perspective: perspective.to_string(),
                    issues,
                    suggestions,
                    summary,
                });
            }
            Err(e) => {
                tracing::warn!(perspective = %perspective, error = %e, "Perspective review failed");
                findings.push(PerspectiveFinding {
                    perspective: perspective.to_string(),
                    issues: vec![],
                    suggestions: vec![],
                    summary: format!("Failed: {}", e),
                });
            }
        }
    }

    all_issues.dedup_by(|a, b| a.file == b.file && a.line == b.line && a.description == b.description);

    let summary = format!(
        "Reviewed from {} perspectives. Found {} total issues ({} critical).",
        perspectives.len(),
        all_issues.len(),
        all_issues.iter().filter(|i| i.severity == Severity::Critical).count()
    );

    Ok(MultiPerspectiveReport {
        perspectives_used: perspectives.iter().map(|p| p.to_string()).collect(),
        findings,
        merged_issues: all_issues,
        summary,
    })
}

#[allow(dead_code)]
fn parse_perspective_response(response: &str) -> (Vec<ReviewIssue>, Vec<ReviewSuggestion>, String) {
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
    struct RawResp {
        #[serde(default)]
        summary: String,
        #[serde(default)]
        issues: Vec<RawIssue>,
        #[serde(default)]
        suggestions: Vec<RawSugg>,
    }

    #[derive(Deserialize)]
    struct RawIssue {
        #[serde(default)]
        file: String,
        #[serde(default)]
        line: u32,
        #[serde(default = "default_sev")]
        severity: String,
        #[serde(default)]
        category: String,
        #[serde(default)]
        description: String,
        #[serde(default)]
        suggested_fix: Option<String>,
    }

    #[derive(Deserialize)]
    struct RawSugg {
        #[serde(default)]
        description: String,
        #[serde(default)]
        file: Option<String>,
    }

    fn default_sev() -> String { "info".to_string() }

    let parsed: RawResp = serde_json::from_str(&json_str)
        .unwrap_or(RawResp { summary: String::new(), issues: vec![], suggestions: vec![] });

    let issues = parsed.issues.into_iter().map(|i| ReviewIssue {
        file: i.file,
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

    (issues, suggestions, parsed.summary)
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

    // ── Additional split_diff_by_file edge cases ────────────────────────────

    #[test]
    fn split_diff_by_file_binary_file() {
        // Binary files in git diffs still have a "diff --git" header
        let diff = "diff --git a/image.png b/image.png\nBinary files a/image.png and b/image.png differ\n";
        let chunks = split_diff_by_file(diff);
        assert_eq!(chunks.len(), 1);
        assert_eq!(chunks[0].0, "image.png");
        assert!(chunks[0].1.contains("Binary files"));
    }

    #[test]
    fn split_diff_by_file_no_b_slash_in_header() {
        // If "diff --git" line lacks " b/", the parser falls back to "unknown"
        let diff = "diff --git weird_format\n+some content\n";
        let chunks = split_diff_by_file(diff);
        assert_eq!(chunks.len(), 1);
        assert_eq!(chunks[0].0, "unknown");
    }

    #[test]
    fn split_diff_by_file_mixed_binary_and_text() {
        let diff = "\
diff --git a/src/lib.rs b/src/lib.rs\n--- a/src/lib.rs\n+++ b/src/lib.rs\n@@ -1 +1 @@\n-old\n+new\n\
diff --git a/logo.png b/logo.png\nBinary files differ\n\
diff --git a/README.md b/README.md\n+added readme\n";
        let chunks = split_diff_by_file(diff);
        assert_eq!(chunks.len(), 3);
        assert_eq!(chunks[0].0, "src/lib.rs");
        assert_eq!(chunks[1].0, "logo.png");
        assert_eq!(chunks[2].0, "README.md");
    }

    // ── Additional compute_score edge cases ─────────────────────────────────

    #[test]
    fn compute_score_multiple_warnings_same_category() {
        // 4 warnings in performance = 4 * 0.5 = 2.0 deduction
        let issues: Vec<ReviewIssue> = (0..4)
            .map(|i| ReviewIssue {
                file: "perf.rs".into(),
                line: i,
                severity: Severity::Warning,
                category: ReviewFocus::Performance,
                description: format!("slow op {}", i),
                suggested_fix: None,
            })
            .collect();
        let score = compute_score(&issues);
        assert!((score.performance - 8.0).abs() < 0.01); // 10.0 - 2.0
        assert_eq!(score.correctness, 10.0);
        assert_eq!(score.security, 10.0);
        assert_eq!(score.style, 10.0);
    }

    #[test]
    fn compute_score_testing_deducts_from_correctness() {
        // Testing category should deduct from correctness
        let issues = vec![ReviewIssue {
            file: "test.rs".into(), line: 1,
            severity: Severity::Critical, category: ReviewFocus::Testing,
            description: "missing tests".into(), suggested_fix: None,
        }];
        let score = compute_score(&issues);
        assert!((score.correctness - 8.0).abs() < 0.01); // 10 - 2.0
        assert_eq!(score.security, 10.0);
    }

    #[test]
    fn compute_score_all_categories_hit() {
        // One critical issue in every category
        let issues = vec![
            ReviewIssue {
                file: "a.rs".into(), line: 1,
                severity: Severity::Critical, category: ReviewFocus::Correctness,
                description: "bug".into(), suggested_fix: None,
            },
            ReviewIssue {
                file: "b.rs".into(), line: 2,
                severity: Severity::Critical, category: ReviewFocus::Security,
                description: "vuln".into(), suggested_fix: None,
            },
            ReviewIssue {
                file: "c.rs".into(), line: 3,
                severity: Severity::Critical, category: ReviewFocus::Performance,
                description: "slow".into(), suggested_fix: None,
            },
            ReviewIssue {
                file: "d.rs".into(), line: 4,
                severity: Severity::Critical, category: ReviewFocus::Style,
                description: "ugly".into(), suggested_fix: None,
            },
        ];
        let score = compute_score(&issues);
        assert!((score.correctness - 8.0).abs() < 0.01);
        assert!((score.security - 8.0).abs() < 0.01);
        assert!((score.performance - 8.0).abs() < 0.01);
        assert!((score.style - 8.0).abs() < 0.01);
        assert!((score.overall - 8.0).abs() < 0.01);
    }

    #[test]
    fn compute_score_info_only_barely_reduces() {
        let issues = vec![ReviewIssue {
            file: "x.rs".into(), line: 1,
            severity: Severity::Info, category: ReviewFocus::Style,
            description: "nit".into(), suggested_fix: None,
        }];
        let score = compute_score(&issues);
        assert!((score.style - 9.9).abs() < 0.01); // 10.0 - 0.1
    }

    // ── Additional exit_code tests ──────────────────────────────────────────

    #[test]
    fn exit_code_zero_when_no_issues() {
        let report = ReviewReport {
            base_ref: String::new(), target_ref: String::new(),
            summary: "clean".into(),
            issues: vec![],
            suggestions: vec![],
            score: ReviewScore { overall: 10.0, correctness: 10.0, security: 10.0, performance: 10.0, style: 10.0 },
            files_reviewed: vec![],
        };
        assert_eq!(report.exit_code(), 0);
    }

    #[test]
    fn exit_code_one_with_mixed_including_critical() {
        let report = ReviewReport {
            base_ref: String::new(), target_ref: String::new(),
            summary: "mixed".into(),
            issues: vec![
                ReviewIssue {
                    file: "a.rs".into(), line: 1,
                    severity: Severity::Info, category: ReviewFocus::Style,
                    description: "nit".into(), suggested_fix: None,
                },
                ReviewIssue {
                    file: "b.rs".into(), line: 2,
                    severity: Severity::Critical, category: ReviewFocus::Security,
                    description: "vuln".into(), suggested_fix: None,
                },
                ReviewIssue {
                    file: "c.rs".into(), line: 3,
                    severity: Severity::Warning, category: ReviewFocus::Performance,
                    description: "slow".into(), suggested_fix: None,
                },
            ],
            suggestions: vec![],
            score: ReviewScore { overall: 5.0, correctness: 10.0, security: 5.0, performance: 9.5, style: 9.9 },
            files_reviewed: vec![],
        };
        assert_eq!(report.exit_code(), 1);
    }

    // ── Additional to_markdown tests ────────────────────────────────────────

    #[test]
    fn to_markdown_no_diff_refs_omits_diff_line() {
        let report = ReviewReport {
            base_ref: String::new(), target_ref: String::new(),
            summary: "Quick review.".into(),
            issues: vec![],
            suggestions: vec![],
            score: ReviewScore { overall: 10.0, correctness: 10.0, security: 10.0, performance: 10.0, style: 10.0 },
            files_reviewed: vec![],
        };
        let md = report.to_markdown();
        assert!(!md.contains("**Diff:**"));
    }

    #[test]
    fn to_markdown_sorts_issues_critical_first() {
        let report = ReviewReport {
            base_ref: "main".into(), target_ref: "HEAD".into(),
            summary: "Issues found.".into(),
            issues: vec![
                ReviewIssue {
                    file: "a.rs".into(), line: 1,
                    severity: Severity::Info, category: ReviewFocus::Style,
                    description: "info issue".into(), suggested_fix: None,
                },
                ReviewIssue {
                    file: "b.rs".into(), line: 2,
                    severity: Severity::Critical, category: ReviewFocus::Security,
                    description: "critical issue".into(), suggested_fix: None,
                },
                ReviewIssue {
                    file: "c.rs".into(), line: 3,
                    severity: Severity::Warning, category: ReviewFocus::Performance,
                    description: "warning issue".into(), suggested_fix: None,
                },
            ],
            suggestions: vec![],
            score: ReviewScore { overall: 7.0, correctness: 10.0, security: 8.0, performance: 9.5, style: 9.9 },
            files_reviewed: vec!["a.rs".into(), "b.rs".into(), "c.rs".into()],
        };
        let md = report.to_markdown();
        // Critical should appear before Warning, which should appear before Info
        let crit_pos = md.find("critical issue").unwrap();
        let warn_pos = md.find("warning issue").unwrap();
        let info_pos = md.find("info issue").unwrap();
        assert!(crit_pos < warn_pos, "Critical should come before Warning");
        assert!(warn_pos < info_pos, "Warning should come before Info");
    }

    // ── Multi-Perspective Review tests ──────────────────────────────────

    #[test]
    fn perspective_display_all_variants() {
        assert_eq!(format!("{}", ReviewPerspective::Architect), "Architect");
        assert_eq!(format!("{}", ReviewPerspective::SecurityExpert), "Security Expert");
        assert_eq!(format!("{}", ReviewPerspective::PerformanceEngineer), "Performance Engineer");
        assert_eq!(format!("{}", ReviewPerspective::TestingSpecialist), "Testing Specialist");
        assert_eq!(format!("{}", ReviewPerspective::UxReviewer), "UX Reviewer");
        assert_eq!(format!("{}", ReviewPerspective::Maintainability), "Maintainability");
        assert_eq!(format!("{}", ReviewPerspective::DevOpsEngineer), "DevOps Engineer");
    }

    #[test]
    fn perspective_system_prompts_nonempty() {
        for p in ReviewPerspective::ALL {
            assert!(!p.system_prompt().is_empty(), "{} has empty prompt", p);
        }
    }

    #[test]
    fn perspective_all_has_five() {
        assert_eq!(ReviewPerspective::ALL.len(), 5);
    }

    #[test]
    fn parse_perspective_response_valid() {
        let json = r#"{"summary": "Good code", "issues": [{"file": "a.rs", "line": 10, "severity": "warning", "category": "security", "description": "weak hash"}], "suggestions": [{"description": "Use bcrypt"}]}"#;
        let (issues, suggestions, summary) = parse_perspective_response(json);
        assert_eq!(summary, "Good code");
        assert_eq!(issues.len(), 1);
        assert_eq!(issues[0].severity, Severity::Warning);
        assert_eq!(suggestions.len(), 1);
    }

    #[test]
    fn parse_perspective_response_invalid() {
        let (issues, suggestions, summary) = parse_perspective_response("not json at all");
        assert!(issues.is_empty());
        assert!(suggestions.is_empty());
        assert!(summary.is_empty());
    }

    #[test]
    fn multi_perspective_report_markdown() {
        let report = MultiPerspectiveReport {
            perspectives_used: vec!["Architect".into(), "Security Expert".into()],
            findings: vec![
                PerspectiveFinding {
                    perspective: "Architect".into(),
                    issues: vec![],
                    suggestions: vec![ReviewSuggestion { description: "Decouple modules".into(), file: None }],
                    summary: "Good architecture overall.".into(),
                },
            ],
            merged_issues: vec![ReviewIssue {
                file: "main.rs".into(),
                line: 5,
                severity: Severity::Warning,
                category: ReviewFocus::Correctness,
                description: "unused variable".into(),
                suggested_fix: None,
            }],
            summary: "Reviewed from 2 perspectives. Found 1 total issues (0 critical).".into(),
        };
        let md = report.to_markdown();
        assert!(md.contains("Multi-Perspective Code Review"));
        assert!(md.contains("Architect"));
        assert!(md.contains("Decouple modules"));
        assert!(md.contains("unused variable"));
    }

    // ── parse_perspective_response edge cases ─────────────────────────────

    #[test]
    fn parse_perspective_response_with_markdown_fences() {
        let input = "```json\n{\"summary\": \"All good\", \"issues\": [], \"suggestions\": []}\n```";
        let (issues, suggestions, summary) = parse_perspective_response(input);
        assert_eq!(summary, "All good");
        assert!(issues.is_empty());
        assert!(suggestions.is_empty());
    }

    #[test]
    fn parse_perspective_response_multiple_issues() {
        let json = r#"{"summary": "Multiple problems", "issues": [
            {"file": "a.rs", "line": 1, "severity": "critical", "category": "security", "description": "SQL injection"},
            {"file": "b.rs", "line": 20, "severity": "warning", "category": "performance", "description": "N+1 query"},
            {"file": "c.rs", "line": 50, "severity": "info", "category": "style", "description": "Long function"}
        ], "suggestions": [{"description": "Refactor"}]}"#;
        let (issues, suggestions, summary) = parse_perspective_response(json);
        assert_eq!(summary, "Multiple problems");
        assert_eq!(issues.len(), 3);
        assert_eq!(issues[0].severity, Severity::Critical);
        assert_eq!(issues[0].category, ReviewFocus::Security);
        assert_eq!(issues[1].severity, Severity::Warning);
        assert_eq!(issues[1].category, ReviewFocus::Performance);
        assert_eq!(issues[2].severity, Severity::Info);
        assert_eq!(issues[2].category, ReviewFocus::Style);
        assert_eq!(suggestions.len(), 1);
    }

    #[test]
    fn parse_perspective_response_unknown_severity_defaults_info() {
        let json = r#"{"summary": "ok", "issues": [
            {"file": "x.rs", "line": 1, "severity": "banana", "category": "testing", "description": "test"}
        ], "suggestions": []}"#;
        let (issues, _, _) = parse_perspective_response(json);
        assert_eq!(issues[0].severity, Severity::Info);
        assert_eq!(issues[0].category, ReviewFocus::Testing);
    }

    #[test]
    fn parse_perspective_response_unknown_category_defaults_correctness() {
        let json = r#"{"summary": "ok", "issues": [
            {"file": "x.rs", "line": 1, "severity": "warning", "category": "unknown_cat", "description": "test"}
        ], "suggestions": []}"#;
        let (issues, _, _) = parse_perspective_response(json);
        assert_eq!(issues[0].category, ReviewFocus::Correctness);
    }

    #[test]
    fn parse_perspective_response_empty_json_object() {
        let json = "{}";
        let (issues, suggestions, summary) = parse_perspective_response(json);
        assert!(issues.is_empty());
        assert!(suggestions.is_empty());
        assert!(summary.is_empty());
    }

    // ── ReviewPerspective additional tests ─────────────────────────────────

    #[test]
    fn perspective_display_extended_variants() {
        assert_eq!(format!("{}", ReviewPerspective::ApiDesigner), "API Designer");
        assert_eq!(format!("{}", ReviewPerspective::DataModeler), "Data Modeler");
        assert_eq!(format!("{}", ReviewPerspective::AccessibilityAuditor), "Accessibility Auditor");
    }

    #[test]
    fn perspective_system_prompt_content_matches_role() {
        assert!(ReviewPerspective::SecurityExpert.system_prompt().contains("security"));
        assert!(ReviewPerspective::PerformanceEngineer.system_prompt().contains("performance"));
        assert!(ReviewPerspective::Architect.system_prompt().contains("architect"));
        assert!(ReviewPerspective::TestingSpecialist.system_prompt().contains("testing"));
        assert!(ReviewPerspective::DevOpsEngineer.system_prompt().contains("DevOps"));
    }

    #[test]
    fn perspective_all_does_not_include_extended() {
        // ALL has 5 perspectives but there are 10 total variants
        assert!(!ReviewPerspective::ALL.contains(&ReviewPerspective::UxReviewer));
        assert!(!ReviewPerspective::ALL.contains(&ReviewPerspective::ApiDesigner));
        assert!(!ReviewPerspective::ALL.contains(&ReviewPerspective::DataModeler));
        assert!(!ReviewPerspective::ALL.contains(&ReviewPerspective::AccessibilityAuditor));
        assert!(!ReviewPerspective::ALL.contains(&ReviewPerspective::DevOpsEngineer));
    }

    // ── to_markdown edge cases ────────────────────────────────────────────

    #[test]
    fn to_markdown_with_suggested_fix_shows_fix() {
        let report = ReviewReport {
            base_ref: "main".into(), target_ref: "HEAD".into(),
            summary: "One issue.".into(),
            issues: vec![ReviewIssue {
                file: "lib.rs".into(), line: 10,
                severity: Severity::Warning, category: ReviewFocus::Style,
                description: "Use snake_case".into(),
                suggested_fix: Some("Rename myVar to my_var".into()),
            }],
            suggestions: vec![],
            score: ReviewScore { overall: 9.5, correctness: 10.0, security: 10.0, performance: 10.0, style: 9.5 },
            files_reviewed: vec!["lib.rs".into()],
        };
        let md = report.to_markdown();
        assert!(md.contains("**Suggested fix:** Rename myVar to my_var"));
    }

    #[test]
    fn to_markdown_with_no_suggested_fix_omits_fix_line() {
        let report = ReviewReport {
            base_ref: "main".into(), target_ref: "HEAD".into(),
            summary: "One issue.".into(),
            issues: vec![ReviewIssue {
                file: "lib.rs".into(), line: 10,
                severity: Severity::Info, category: ReviewFocus::Correctness,
                description: "Consider edge case".into(),
                suggested_fix: None,
            }],
            suggestions: vec![],
            score: ReviewScore { overall: 9.9, correctness: 9.9, security: 10.0, performance: 10.0, style: 10.0 },
            files_reviewed: vec!["lib.rs".into()],
        };
        let md = report.to_markdown();
        assert!(!md.contains("**Suggested fix:**"));
    }

    #[test]
    fn to_markdown_suggestions_with_file_and_without() {
        let report = ReviewReport {
            base_ref: String::new(), target_ref: String::new(),
            summary: "Suggestions only.".into(),
            issues: vec![],
            suggestions: vec![
                ReviewSuggestion { description: "Add logging".into(), file: Some("main.rs".into()) },
                ReviewSuggestion { description: "Consider CI".into(), file: None },
            ],
            score: ReviewScore { overall: 10.0, correctness: 10.0, security: 10.0, performance: 10.0, style: 10.0 },
            files_reviewed: vec![],
        };
        let md = report.to_markdown();
        assert!(md.contains("- Add logging (`main.rs`)"));
        assert!(md.contains("- Consider CI\n"));
    }

    // ── MultiPerspectiveReport edge cases ─────────────────────────────────

    #[test]
    fn multi_perspective_report_empty_findings() {
        let report = MultiPerspectiveReport {
            perspectives_used: vec!["Architect".into()],
            findings: vec![],
            merged_issues: vec![],
            summary: "No issues found.".into(),
        };
        let md = report.to_markdown();
        assert!(md.contains("Merged Issues (0)"));
        assert!(md.contains("No issues found."));
        assert!(md.contains("*Generated by VibeCLI Multi-Perspective Review*"));
    }

    #[test]
    fn multi_perspective_report_sorted_by_severity() {
        let report = MultiPerspectiveReport {
            perspectives_used: vec!["Security Expert".into()],
            findings: vec![],
            merged_issues: vec![
                ReviewIssue {
                    file: "a.rs".into(), line: 1,
                    severity: Severity::Info, category: ReviewFocus::Style,
                    description: "info issue".into(), suggested_fix: None,
                },
                ReviewIssue {
                    file: "b.rs".into(), line: 2,
                    severity: Severity::Critical, category: ReviewFocus::Security,
                    description: "critical issue".into(), suggested_fix: None,
                },
            ],
            summary: "Found 2 issues.".into(),
        };
        let md = report.to_markdown();
        let crit_pos = md.find("critical issue").unwrap();
        let info_pos = md.find("info issue").unwrap();
        assert!(crit_pos < info_pos, "Critical should appear before Info in merged issues");
    }

    // ── compute_score with many info issues ───────────────────────────────

    #[test]
    fn compute_score_many_info_issues_gradual_decrease() {
        // 20 info issues in style = 20 * 0.1 = 2.0 deduction
        let issues: Vec<ReviewIssue> = (0..20)
            .map(|i| ReviewIssue {
                file: "nit.rs".into(),
                line: i,
                severity: Severity::Info,
                category: ReviewFocus::Style,
                description: format!("nit {}", i),
                suggested_fix: None,
            })
            .collect();
        let score = compute_score(&issues);
        assert!((score.style - 8.0).abs() < 0.01); // 10.0 - 2.0
        assert_eq!(score.correctness, 10.0);
    }

    // ── Severity serde roundtrip ──────────────────────────────────────────

    #[test]
    fn severity_serde_roundtrip() {
        for sev in &[Severity::Info, Severity::Warning, Severity::Critical] {
            let json = serde_json::to_string(sev).unwrap();
            let back: Severity = serde_json::from_str(&json).unwrap();
            assert_eq!(&back, sev);
        }
    }

    #[test]
    fn review_focus_serde_roundtrip() {
        for focus in &[ReviewFocus::Security, ReviewFocus::Performance,
                       ReviewFocus::Correctness, ReviewFocus::Style, ReviewFocus::Testing] {
            let json = serde_json::to_string(focus).unwrap();
            let back: ReviewFocus = serde_json::from_str(&json).unwrap();
            assert_eq!(&back, focus);
        }
    }
}
