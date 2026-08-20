#![allow(dead_code)]
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
use vibe_ai::{retry_async, RetryConfig};

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
            Severity::Error => write!(f, "error"),
            Severity::Warning => write!(f, "warning"),
            Severity::Info => write!(f, "info"),
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
            Severity::Error => "❌",
            Severity::Warning => "⚠️ ",
            Severity::Info => "ℹ️ ",
        }
    }
}

// ── OWASP / CWE static scanner ────────────────────────────────────────────────

/// Pattern-based OWASP/CWE security scan on a unified diff.
///
/// Runs before the LLM analysis so critical issues surface even when the LLM is
/// unavailable.  Each added line (`+`) in the diff is checked against the table
/// below; one finding per line (first matching pattern wins).
pub fn detect_security_patterns(diff: &str) -> Vec<BugReport> {
    use regex::Regex;

    // (regex, CWE id, severity, message, suggestion)
    let raw_patterns: &[(&str, &str, Severity, &str, &str)] = &[
        (
            r"(?i)(execute|query|raw_query|exec)\s*\(\s*[&|]?\s*format!\s*\(",
            "CWE-89",
            Severity::Error,
            "Possible SQL injection: dynamic query constructed with format! macro",
            "Use parameterized queries (e.g. sqlx::query! macro or bound parameters)",
        ),
        (
            r#"(?i)(\.innerHTML\s*=|dangerouslySetInnerHTML\s*=\s*\{\s*\{|document\.write\s*\(|eval\s*\()"#,
            "CWE-79",
            Severity::Error,
            "Possible XSS: unsanitized HTML injection point",
            "Sanitize user content with DOMPurify or use textContent instead of innerHTML",
        ),
        (
            r#"(?i)(File::open|read_to_string|fs::read|std::fs::File::open)\s*\(\s*[^)]*user"#,
            "CWE-22",
            Severity::Error,
            "Possible path traversal: file path derived from user input without canonicalization",
            "Call .canonicalize() and verify the result stays within the allowed directory",
        ),
        (
            r#"(?i)(api_key|apikey|api_secret|password|passwd|secret_key|auth_token)\s*[:=]\s*["'][A-Za-z0-9+/=_\-]{8,}"#,
            "CWE-798",
            Severity::Error,
            "Hardcoded credential detected",
            "Store secrets in environment variables or a secrets manager; never commit them",
        ),
        (
            r"(?i)\bMath\.random\(\)|std::rand::|rand::random\b",
            "CWE-338",
            Severity::Warning,
            "Insecure pseudo-random number generator — may be unsuitable for security use",
            "Use a cryptographically secure RNG: crypto.getRandomValues(), rand::SystemRandom, or secrets.token_bytes()",
        ),
        (
            r#"(?i)(shell\s*=\s*True|subprocess\.call\s*\(|os\.system\s*\(|popen\s*\()|Command::new\s*\(\s*"sh""#,
            "CWE-78",
            Severity::Error,
            "Possible command injection: shell execution with potential user-controlled input",
            "Avoid shell=True; pass arguments as a list and validate all user input before use",
        ),
        (
            r#"(?i)(redirect|location\.href\s*=|Response\.redirect)\s*\(\s*\w*(?:_url|_path|_redirect|_next|url|path|next|redirect)\b"#,
            "CWE-601",
            Severity::Warning,
            "Possible open redirect: redirect target may be user-controlled",
            "Validate redirect URLs against an allowlist of trusted domains before redirecting",
        ),
        // ── Phase 41: Red Team expanded CWE coverage ──────────────────────────
        (
            r#"(?i)(fetch|axios|requests?\.(get|post)|http\.get|urllib)\s*\(\s*[^)]*(?:url|uri|href|endpoint|target|host|addr)"#,
            "CWE-918",
            Severity::Error,
            "Possible SSRF: server-side request with user-controllable URL",
            "Validate and allowlist target URLs; block private IP ranges (127.0.0.0/8, 10.0.0.0/8, 172.16.0.0/12, 192.168.0.0/16)",
        ),
        (
            r#"(?i)(XMLParser|DOMParser|etree\.parse|SAXParser|DocumentBuilder)\s*\("#,
            "CWE-611",
            Severity::Error,
            "Possible XXE: XML parser may process external entities",
            "Disable external entity processing: set XMLConstants.FEATURE_SECURE_PROCESSING or equivalent for your parser",
        ),
        (
            r#"(?i)(pickle\.loads?|yaml\.load\s*\(|yaml\.unsafe_load|marshal\.loads?|unserialize\s*\(|ObjectInputStream)"#,
            "CWE-502",
            Severity::Error,
            "Possible insecure deserialization: untrusted data passed to unsafe deserializer",
            "Never deserialize untrusted data; use yaml.safe_load, JSON, or schema-validated formats",
        ),
        (
            r#"(?i)\$where\s*:|\.find\s*\(\s*\{[^}]*\$(?:regex|where|gt|lt|ne|in)\b"#,
            "CWE-943",
            Severity::Error,
            "Possible NoSQL injection: MongoDB operator in query with potential user input",
            "Sanitize query parameters; use typed schemas; avoid $where and JavaScript execution operators",
        ),
        (
            r#"(?i)(render_template_string|Template\s*\(\s*(?:request|params|query|user|body)|Jinja2\.from_string|\.render\s*\(\s*(?:req|params))"#,
            "CWE-1336",
            Severity::Error,
            "Possible template injection: user-controlled data passed to template engine",
            "Never pass user input directly to template constructors; use pre-compiled templates with variable interpolation",
        ),
        (
            r#"(?i)/(?:api|v\d)/\w+/\d+|(?:findById|get_by_id|find_one)\s*\(\s*(?:req\.|params\.|request\.)"#,
            "CWE-639",
            Severity::Warning,
            "Possible IDOR: resource accessed by sequential ID without apparent authorization check",
            "Verify the requesting user is authorized to access the specific resource; use UUIDs over sequential IDs",
        ),
        (
            r#"(?i)(app\.(post|put|patch|delete)|router\.(post|put|patch|delete))\s*\(\s*["'][^"']+"#,
            "CWE-352",
            Severity::Warning,
            "Possible missing CSRF protection: state-changing endpoint without apparent token validation",
            "Implement CSRF tokens for all state-changing requests; use SameSite=Strict cookie attribute",
        ),
        (
            r#"(?i)["']http://[a-z0-9][\w\.-]+\.(com|io|org|net|dev)/api"#,
            "CWE-319",
            Severity::Warning,
            "Possible cleartext transmission: API endpoint using HTTP instead of HTTPS",
            "Use HTTPS for all API endpoints; enable HSTS; redirect HTTP to HTTPS",
        ),
    ];

    // Compile once (called once per review_diff invocation, not in a tight loop).
    let compiled: Vec<(Regex, &str, Severity, &str, &str)> = raw_patterns
        .iter()
        .filter_map(|(pat, cwe, sev, msg, sug)| {
            Regex::new(pat)
                .ok()
                .map(|re| (re, *cwe, sev.clone(), *msg, *sug))
        })
        .collect();

    let mut reports: Vec<BugReport> = Vec::new();
    let mut current_file = String::new();
    let mut current_new_line: u32 = 0;

    for raw_line in diff.lines() {
        // +++ b/path/to/file.rs  — track filename
        if raw_line.starts_with("+++ ") {
            current_file = raw_line
                .trim_start_matches("+++ ")
                .trim_start_matches("b/")
                .to_string();
            current_new_line = 0;
            continue;
        }
        if raw_line.starts_with("--- ") {
            continue;
        }

        // @@ -old_start[,count] +new_start[,count] @@ — reset line counter
        if raw_line.starts_with("@@") {
            if let Some(plus_part) = raw_line.split('+').nth(1) {
                let num_str = plus_part
                    .split(',')
                    .next()
                    .unwrap_or("0")
                    .split(' ')
                    .next()
                    .unwrap_or("0");
                current_new_line = num_str.parse::<u32>().unwrap_or(1).saturating_sub(1);
            }
            continue;
        }

        if raw_line.starts_with('+') && !raw_line.starts_with("+++") {
            current_new_line += 1;
            let code_line = &raw_line[1..]; // strip leading '+'
            for (re, cwe, sev, msg, sug) in &compiled {
                if re.is_match(code_line) {
                    reports.push(BugReport {
                        file: current_file.clone(),
                        line: current_new_line,
                        severity: sev.clone(),
                        message: format!("[{}] {}", cwe, msg),
                        suggestion: Some(sug.to_string()),
                        fix_command: None,
                        category: Some("security".to_string()),
                    });
                    break; // one finding per line
                }
            }
        } else if raw_line.starts_with(' ') {
            current_new_line += 1;
        }
        // Lines starting with '-' are removed; do not advance the new-file counter.
    }

    reports
}

// ── Review planning ───────────────────────────────────────────────────────────

/// How much review to buy for one diff.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ReviewPlan {
    /// Orderings of each batch to review. 1 is one look at every file.
    pub passes: usize,
    /// Characters of diff per request.
    pub char_budget: usize,
    /// Hard ceiling on LLM round-trips for the whole review.
    pub max_calls: usize,
}

impl Default for ReviewPlan {
    fn default() -> Self {
        // 8 calls × 8 000 chars covers a ~64 KB diff in full — well past the
        // single 8 000-char request this replaces — without a surprising bill.
        Self {
            passes: 1,
            char_budget: 8_000,
            max_calls: 8,
        }
    }
}

/// What the LLM review actually looked at.
///
/// Reported rather than assumed: "no findings" means something very different
/// when half the diff never reached the model. The static OWASP/CWE scan always
/// covers the whole diff — this describes the model passes only.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct ReviewCoverage {
    pub files_total: usize,
    /// Files whose batch completed at least one successful model pass.
    pub files_reviewed: usize,
    /// Model round-trips attempted.
    pub llm_calls: usize,
    /// Of those, how many the provider failed. A failed call reviews nothing.
    pub llm_calls_failed: usize,
    /// Files whose own section exceeded the per-request budget and was cut.
    pub files_truncated: Vec<String>,
    /// Files dropped entirely because `max_calls` ran out.
    pub files_skipped: Vec<String>,
    /// Files whose every model pass errored — the provider was down, rate
    /// limited, or unconfigured. Only the static scan looked at these.
    pub files_provider_failed: Vec<String>,
}

impl ReviewCoverage {
    /// True when every file in the diff reached the model whole and came back.
    pub fn is_complete(&self) -> bool {
        self.files_skipped.is_empty()
            && self.files_truncated.is_empty()
            && self.files_provider_failed.is_empty()
    }

    /// One line for the terminal / PR body, or `None` when coverage was complete.
    pub fn caveat(&self) -> Option<String> {
        if self.is_complete() {
            return None;
        }
        let mut parts = Vec::new();
        if !self.files_skipped.is_empty() {
            parts.push(format!(
                "{} file(s) not reviewed (call budget)",
                self.files_skipped.len()
            ));
        }
        if !self.files_provider_failed.is_empty() {
            parts.push(format!(
                "{} file(s) not reviewed (provider error)",
                self.files_provider_failed.len()
            ));
        }
        if !self.files_truncated.is_empty() {
            parts.push(format!(
                "{} file(s) truncated to fit the request",
                self.files_truncated.len()
            ));
        }
        Some(parts.join("; "))
    }
}

/// One file's section of a unified diff.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DiffSection {
    pub path: String,
    pub text: String,
}

/// Split a unified diff into per-file sections, preserving order.
///
/// Anything before the first file header (a cover letter, `commit` lines) is
/// dropped — it is not code and only consumes budget.
pub fn split_diff_by_file(diff: &str) -> Vec<DiffSection> {
    let mut sections: Vec<DiffSection> = Vec::new();
    let mut current: Option<DiffSection> = None;

    for line in diff.lines() {
        if let Some(rest) = line.strip_prefix("diff --git ") {
            if let Some(done) = current.take() {
                sections.push(done);
            }
            current = Some(DiffSection {
                path: rest
                    .rsplit_once(" b/")
                    .map(|(_, b)| b.to_string())
                    .unwrap_or_else(|| rest.to_string()),
                text: String::new(),
            });
        }
        if let Some(section) = current.as_mut() {
            section.text.push_str(line);
            section.text.push('\n');
        }
    }
    if let Some(done) = current {
        sections.push(done);
    }

    // A plain `diff -u` with no `diff --git` header is still one reviewable unit.
    if sections.is_empty() && !diff.trim().is_empty() {
        sections.push(DiffSection {
            path: String::new(),
            text: diff.to_string(),
        });
    }
    sections
}

/// Pack sections into batches that each fit `budget` characters.
///
/// Returns the batches and the paths of files whose own section exceeded the
/// budget and had to be cut — named, so the caller can say so.
fn pack_into_batches(
    sections: &[DiffSection],
    budget: usize,
) -> (Vec<Vec<DiffSection>>, Vec<String>) {
    let mut batches: Vec<Vec<DiffSection>> = Vec::new();
    let mut current: Vec<DiffSection> = Vec::new();
    let mut used = 0usize;
    let mut truncated = Vec::new();

    for section in sections {
        let section = if section.text.len() > budget {
            truncated.push(section.path.clone());
            DiffSection {
                path: section.path.clone(),
                text: truncate_on_char_boundary(&section.text, budget),
            }
        } else {
            section.clone()
        };

        if !current.is_empty() && used + section.text.len() > budget {
            batches.push(std::mem::take(&mut current));
            used = 0;
        }
        used += section.text.len();
        current.push(section);
    }
    if !current.is_empty() {
        batches.push(current);
    }

    (batches, truncated)
}

/// Cut a string to at most `max` bytes without splitting a character.
fn truncate_on_char_boundary(text: &str, max: usize) -> String {
    if text.len() <= max {
        return text.to_string();
    }
    let end = (0..=max)
        .rev()
        .find(|i| text.is_char_boundary(*i))
        .unwrap_or(0);
    text[..end].to_string()
}

/// Rotate a batch left by `pass` so a different file leads each time.
fn rotate(batch: &[DiffSection], pass: usize) -> Vec<&DiffSection> {
    if batch.is_empty() {
        return Vec::new();
    }
    let offset = pass % batch.len();
    batch[offset..].iter().chain(&batch[..offset]).collect()
}

/// Build the review prompt for one ordered batch.
fn review_prompt(batch: &[&DiffSection]) -> String {
    let body = batch
        .iter()
        .map(|s| s.text.as_str())
        .collect::<Vec<_>>()
        .join("");

    format!(
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

`line` must be a line number from the new side of the diff.
Return an empty array [] if there are no issues.

Diff:
```diff
{}
```
"#,
        body
    )
}

/// Extract the findings array from a model reply.
///
/// Reasoning comes off first. A model that deliberates before answering writes
/// brackets while it does — "[1] the missing null check" — and this scans from
/// the first `[` to the last `]`, so the deliberation could become the payload.
/// What survives is posted verbatim as a PR review comment, where a leaked
/// `<thinking>` tag is not untidy but permanent.
fn parse_reports(response: &str) -> Vec<BugReport> {
    let visible = vibe_ai::tools::strip_thinking(response);
    let response = visible.as_str();
    let Some(start) = response.find('[') else {
        return vec![];
    };
    let Some(end) = response.rfind(']').map(|i| i + 1) else {
        return vec![];
    };
    if start >= end {
        return vec![];
    }
    serde_json::from_str::<Vec<BugReport>>(&response[start..end]).unwrap_or_default()
}

/// Collapse findings that repeated across passes, keeping the highest severity.
///
/// Two passes over the same code phrase the same defect differently, so the key
/// is the location plus a normalised message rather than the message verbatim.
fn dedupe_reports(reports: Vec<BugReport>) -> Vec<BugReport> {
    fn rank(s: &Severity) -> u8 {
        match s {
            Severity::Error => 2,
            Severity::Warning => 1,
            Severity::Info => 0,
        }
    }
    fn key(r: &BugReport) -> (String, u32, String) {
        // Punctuation becomes a separator, not nothing: one pass writes
        // "off-by-one", the next writes "off by one", and they are the same bug.
        let normalised: String = r
            .message
            .to_lowercase()
            .chars()
            .map(|c| if c.is_alphanumeric() { c } else { ' ' })
            .collect::<String>()
            .split_whitespace()
            .take(8)
            .collect::<Vec<_>>()
            .join(" ");
        (r.file.clone(), r.line, normalised)
    }

    let mut best: std::collections::BTreeMap<(String, u32, String), BugReport> =
        std::collections::BTreeMap::new();
    for report in reports {
        match best.entry(key(&report)) {
            std::collections::btree_map::Entry::Vacant(slot) => {
                slot.insert(report);
            }
            std::collections::btree_map::Entry::Occupied(mut slot) => {
                if rank(&report.severity) > rank(&slot.get().severity) {
                    slot.insert(report);
                }
            }
        }
    }

    let mut out: Vec<BugReport> = best.into_values().collect();
    out.sort_by(|a, b| {
        rank(&b.severity)
            .cmp(&rank(&a.severity))
            .then_with(|| a.file.cmp(&b.file))
            .then_with(|| a.line.cmp(&b.line))
    });
    out
}

// ── BugBot ────────────────────────────────────────────────────────────────────

pub struct BugBot {
    pub llm: Arc<dyn LLMProvider>,
    pub gh_token: Option<String>,
}

impl BugBot {
    pub fn new(llm: Arc<dyn LLMProvider>) -> Self {
        // Route through the canonical resolver so ProfileStore wins
        // over env. AGENTS.md → Zero-Config First.
        Self {
            llm,
            gh_token: crate::github_app::resolve_github_token(),
        }
    }

    pub fn with_gh_token(mut self, token: impl Into<String>) -> Self {
        self.gh_token = Some(token.into());
        self
    }

    /// Analyze a unified diff and return bug reports.
    ///
    /// Uses [`ReviewPlan::default`], which covers the whole diff rather than its
    /// first 8 000 characters. A small diff still costs exactly one request; a
    /// large one costs up to `max_calls`. Use
    /// [`review_diff_planned`](Self::review_diff_planned) when you need to know
    /// what coverage the review actually achieved.
    pub async fn review_diff(&self, diff: &str) -> Vec<BugReport> {
        if diff.trim().is_empty() {
            return vec![];
        }
        self.review_diff_planned(diff, ReviewPlan::default())
            .await
            .0
    }

    /// Review a diff with full file coverage and optional repeated passes.
    ///
    /// [`review_diff`](Self::review_diff) sends the first `8000` characters of the
    /// diff and nothing else — on any PR past a few files, everything after the
    /// cutoff is silently unreviewed. This splits the diff per file, packs the
    /// files into batches that each fit the budget, and reviews every batch, so
    /// coverage is a property of the plan rather than of how the diff happened to
    /// be ordered.
    ///
    /// `passes > 1` reviews each batch again with the files rotated. A model's
    /// attention is not uniform across a long prompt, so a finding in the last
    /// file of a batch is likelier to be missed than one in the first; rotating
    /// gives every file a turn at the front. Rotation is deterministic, so two
    /// runs over the same diff issue the same requests.
    ///
    /// Returns findings deduplicated across passes, plus a [`ReviewCoverage`]
    /// stating what was actually reviewed.
    pub async fn review_diff_planned(
        &self,
        diff: &str,
        plan: ReviewPlan,
    ) -> (Vec<BugReport>, ReviewCoverage) {
        let static_reports = detect_security_patterns(diff);

        let files = split_diff_by_file(diff);
        if files.is_empty() {
            return (static_reports, ReviewCoverage::default());
        }

        let budget = plan.char_budget.max(1);
        let (batches, truncated) = pack_into_batches(&files, budget);

        // Pass-major, so the first `batches.len()` requests are one complete look
        // at every file. The call ceiling therefore costs extra passes before it
        // ever costs coverage — and when it does cost coverage, the tail it drops
        // is exactly `batches[allowed..]`, which is named rather than lost.
        let passes = plan.passes.max(1);
        let requests: Vec<(usize, usize)> = (0..passes)
            .flat_map(|pass| (0..batches.len()).map(move |batch| (batch, pass)))
            .collect();
        let allowed = requests.len().min(plan.max_calls.max(1));
        let skipped_batches: Vec<usize> = (allowed.min(batches.len())..batches.len()).collect();

        let futures = requests[..allowed].iter().map(|&(batch, pass)| {
            let prompt = review_prompt(&rotate(&batches[batch], pass));
            async move { (batch, self.review_once(prompt).await) }
        });

        let per_request = futures::future::join_all(futures).await;

        // A failed call reviewed nothing. Counting it as coverage is the exact
        // shape of bug this struct exists to prevent: with the provider down,
        // "0 findings, 1/1 files reviewed" is a clean bill of health nobody gave.
        let mut succeeded: std::collections::BTreeSet<usize> = std::collections::BTreeSet::new();
        let mut llm_calls_failed = 0usize;
        let mut all = static_reports;
        for (batch, outcome) in per_request {
            match outcome {
                Some(reports) => {
                    succeeded.insert(batch);
                    all.extend(reports);
                }
                None => llm_calls_failed += 1,
            }
        }

        let paths_of = |batch: usize| batches[batch].iter().map(|f| f.path.clone());
        let files_skipped: Vec<String> =
            skipped_batches.iter().copied().flat_map(paths_of).collect();
        let files_provider_failed: Vec<String> = (0..batches.len())
            .filter(|b| !succeeded.contains(b) && !skipped_batches.contains(b))
            .flat_map(paths_of)
            .collect();

        let coverage = ReviewCoverage {
            files_total: files.len(),
            files_reviewed: files
                .len()
                .saturating_sub(files_skipped.len() + files_provider_failed.len()),
            llm_calls: allowed,
            llm_calls_failed,
            files_truncated: truncated,
            files_skipped,
            files_provider_failed,
        };

        (dedupe_reports(all), coverage)
    }

    /// One review round-trip.
    ///
    /// `None` means the provider failed — distinct from `Some(vec![])`, which
    /// means the model looked and found nothing. Collapsing the two is what
    /// lets an outage read as a clean review.
    async fn review_once(&self, prompt: String) -> Option<Vec<BugReport>> {
        let msgs = vec![Message {
            role: MessageRole::User,
            content: prompt,
        }];
        match self.llm.chat(&msgs, None).await {
            Ok(response) => Some(parse_reports(&response)),
            Err(e) => {
                tracing::debug!(
                    target: "vibecody::bugbot",
                    error = %e,
                    "review pass failed — its files are reported as unreviewed"
                );
                None
            }
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
        let url = format!(
            "https://api.github.com/repos/{}/{}/pulls/{}",
            owner, repo, pr_number
        );
        let client = reqwest::Client::builder()
            .timeout(std::time::Duration::from_secs(30))
            .connect_timeout(std::time::Duration::from_secs(10))
            .build()?;
        let gh_token = self.gh_token.clone();
        let resp = retry_async(&RetryConfig::default(), "bugbot-fetch-pr-diff", || {
            let client = client.clone();
            let url = url.clone();
            let gh_token = gh_token.clone();
            async move {
                let mut req = client
                    .get(&url)
                    .header("Accept", "application/vnd.github.v3.diff")
                    .header("User-Agent", "vibecli-bugbot/1.0");
                if let Some(token) = &gh_token {
                    req = req.header("Authorization", format!("Bearer {}", token));
                }
                req.send().await.map_err(Into::into)
            }
        })
        .await?;
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
        self.post_github_review_with_fixes(
            owner,
            repo,
            pr_number,
            reports,
            &std::collections::HashMap::new(),
            commit_sha,
        )
        .await
    }

    /// Post inline review comments, attaching a committable ```` ```suggestion ````
    /// block to every finding that has an anchored fix.
    ///
    /// `fixes` is keyed by index into `reports`. A finding without an entry is
    /// posted as prose, exactly as before — a missing fix is never faked.
    pub async fn post_github_review_with_fixes(
        &self,
        owner: &str,
        repo: &str,
        pr_number: u64,
        reports: &[BugReport],
        fixes: &std::collections::HashMap<usize, crate::bugbot_autofix::FixProposal>,
        commit_sha: &str,
    ) -> Result<()> {
        let token = self
            .gh_token
            .as_ref()
            .ok_or_else(|| anyhow::anyhow!("GITHUB_TOKEN not set"))?;

        let client = reqwest::Client::builder()
            .timeout(std::time::Duration::from_secs(30))
            .connect_timeout(std::time::Duration::from_secs(10))
            .build()?;
        let url = format!(
            "https://api.github.com/repos/{}/{}/pulls/{}/reviews",
            owner, repo, pr_number
        );

        let comments: Vec<serde_json::Value> = reports
            .iter()
            .enumerate()
            .filter(|(_, r)| r.severity == Severity::Error || r.severity == Severity::Warning)
            .map(|(i, r)| match fixes.get(&i) {
                Some(fix) => fix.review_comment_json(r),
                None => {
                    let mut body = format!("**{}** {}: {}", r.icon(), r.severity, r.message);
                    if let Some(sug) = &r.suggestion {
                        body.push_str(&format!("\n\n💡 **Suggestion:** {}", sug));
                    }
                    serde_json::json!({
                        "path": r.file,
                        "line": r.line,
                        "body": body,
                    })
                }
            })
            .collect();

        if comments.is_empty() {
            return Ok(());
        }

        let fix_count = fixes.len();
        let body_text = match (
            reports.iter().any(|r| r.severity == Severity::Error),
            fix_count,
        ) {
            (_, n) if n > 0 => format!(
                "🤖 **BugBot** found issues and proposed {} committable fix{}. \
                 Commit a suggestion to apply it — the anchors were verified against this diff, \
                 but the fixes have not been compiled or tested.",
                n,
                if n == 1 { "" } else { "es" }
            ),
            (true, _) => {
                "🤖 **BugBot** found issues that need attention. Please review the inline comments."
                    .to_string()
            }
            (false, _) => "🤖 **BugBot** found some warnings. See inline comments.".to_string(),
        };

        let payload = serde_json::json!({
            "commit_id": commit_sha,
            "body": body_text,
            "event": "COMMENT",
            "comments": comments,
        });

        let resp = retry_async(&RetryConfig::default(), "bugbot-post-review", || {
            let client = client.clone();
            let url = url.clone();
            let token = token.clone();
            let payload = payload.clone();
            async move {
                client
                    .post(&url)
                    .header("Authorization", format!("Bearer {}", token))
                    .header("Accept", "application/vnd.github.v3+json")
                    .header("User-Agent", "vibecli-bugbot/1.0")
                    .json(&payload)
                    .send()
                    .await
                    .map_err(Into::into)
            }
        })
        .await?;

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

        let errors = reports
            .iter()
            .filter(|r| r.severity == Severity::Error)
            .count();
        let warnings = reports
            .iter()
            .filter(|r| r.severity == Severity::Warning)
            .count();
        let infos = reports
            .iter()
            .filter(|r| r.severity == Severity::Info)
            .count();

        let mut out = format!(
            "\n🤖 BugBot Review: {} errors, {} warnings, {} info\n{}\n",
            errors,
            warnings,
            infos,
            "─".repeat(50)
        );

        for r in reports {
            out.push_str(&format!(
                "\n{} [{}] {}:{}\n   {}\n",
                r.icon(),
                r.severity,
                r.file,
                r.line,
                r.message
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

    fn section(path: &str, body: &str) -> String {
        format!(
            "diff --git a/{p} b/{p}\n--- a/{p}\n+++ b/{p}\n@@ -1,1 +1,1 @@\n+{body}\n",
            p = path
        )
    }

    fn finding(file: &str, line: u32, severity: Severity, message: &str) -> BugReport {
        BugReport {
            file: file.into(),
            line,
            severity,
            message: message.into(),
            suggestion: None,
            fix_command: None,
            category: None,
        }
    }

    // ── split_diff_by_file ───────────────────────────────────────────────────

    #[test]
    fn splits_a_multi_file_diff_into_sections() {
        let diff = format!("{}{}", section("a.rs", "one"), section("b/c.rs", "two"));
        let sections = split_diff_by_file(&diff);
        assert_eq!(sections.len(), 2);
        assert_eq!(sections[0].path, "a.rs");
        assert_eq!(sections[1].path, "b/c.rs");
        assert!(sections[0].text.contains("+one"));
        assert!(!sections[0].text.contains("+two"));
    }

    #[test]
    fn drops_a_preamble_before_the_first_file_header() {
        let diff = format!("commit abc123\nAuthor: me\n\n{}", section("a.rs", "one"));
        let sections = split_diff_by_file(&diff);
        assert_eq!(sections.len(), 1);
        assert!(!sections[0].text.contains("Author"));
    }

    #[test]
    fn a_headerless_diff_is_still_one_reviewable_section() {
        let sections = split_diff_by_file("--- a/x\n+++ b/x\n@@ -1 +1 @@\n+y\n");
        assert_eq!(sections.len(), 1);
        assert!(sections[0].path.is_empty());
    }

    #[test]
    fn an_empty_diff_yields_no_sections() {
        assert!(split_diff_by_file("").is_empty());
        assert!(split_diff_by_file("   \n\n").is_empty());
    }

    // ── pack_into_batches ────────────────────────────────────────────────────

    #[test]
    fn packs_every_file_into_some_batch() {
        let sections = split_diff_by_file(&format!(
            "{}{}{}",
            section("a.rs", "one"),
            section("b.rs", "two"),
            section("c.rs", "three")
        ));
        let (batches, truncated) = pack_into_batches(&sections, 100);
        assert!(truncated.is_empty());
        let packed: usize = batches.iter().map(Vec::len).sum();
        assert_eq!(packed, 3, "no file may be dropped by packing");
        assert!(batches.len() > 1, "a 100-char budget cannot hold all three");
    }

    #[test]
    fn a_single_oversized_file_is_truncated_and_named() {
        let big = section("huge.rs", &"x".repeat(500));
        let sections = split_diff_by_file(&big);
        let (batches, truncated) = pack_into_batches(&sections, 120);
        assert_eq!(truncated, vec!["huge.rs".to_string()]);
        assert_eq!(batches.len(), 1);
        assert!(batches[0][0].text.len() <= 120);
    }

    #[test]
    fn truncation_never_splits_a_character() {
        // Each `é` is two bytes; a byte-slice at an odd offset would panic.
        let text = "é".repeat(50);
        let cut = truncate_on_char_boundary(&text, 25);
        assert!(cut.len() <= 25);
        assert_eq!(cut.chars().count(), 12);
    }

    #[test]
    fn one_batch_when_everything_fits() {
        let sections =
            split_diff_by_file(&format!("{}{}", section("a.rs", "1"), section("b.rs", "2")));
        let (batches, _) = pack_into_batches(&sections, 100_000);
        assert_eq!(batches.len(), 1);
        assert_eq!(batches[0].len(), 2);
    }

    // ── rotate ───────────────────────────────────────────────────────────────

    #[test]
    fn rotation_gives_each_file_a_turn_at_the_front() {
        let sections = split_diff_by_file(&format!(
            "{}{}{}",
            section("a.rs", "1"),
            section("b.rs", "2"),
            section("c.rs", "3")
        ));
        let leads: Vec<&str> = (0..3)
            .map(|pass| rotate(&sections, pass)[0].path.as_str())
            .collect();
        assert_eq!(leads, vec!["a.rs", "b.rs", "c.rs"]);
    }

    #[test]
    fn rotation_preserves_every_file() {
        let sections =
            split_diff_by_file(&format!("{}{}", section("a.rs", "1"), section("b.rs", "2")));
        let rotated = rotate(&sections, 1);
        assert_eq!(rotated.len(), 2);
    }

    #[test]
    fn rotation_is_deterministic_across_calls() {
        let sections =
            split_diff_by_file(&format!("{}{}", section("a.rs", "1"), section("b.rs", "2")));
        let first: Vec<&str> = rotate(&sections, 7)
            .iter()
            .map(|s| s.path.as_str())
            .collect();
        let second: Vec<&str> = rotate(&sections, 7)
            .iter()
            .map(|s| s.path.as_str())
            .collect();
        assert_eq!(first, second);
    }

    #[test]
    fn rotating_an_empty_batch_is_empty() {
        assert!(rotate(&[], 3).is_empty());
    }

    // ── dedupe_reports ───────────────────────────────────────────────────────

    #[test]
    fn collapses_the_same_finding_reported_by_two_passes() {
        let reports = vec![
            finding(
                "a.rs",
                10,
                Severity::Warning,
                "Off-by-one in the loop bound",
            ),
            finding(
                "a.rs",
                10,
                Severity::Warning,
                "off by one in the loop bound!",
            ),
        ];
        assert_eq!(dedupe_reports(reports).len(), 1);
    }

    #[test]
    fn keeps_the_highest_severity_of_a_duplicate() {
        let reports = vec![
            finding("a.rs", 10, Severity::Info, "Off by one in the loop bound"),
            finding("a.rs", 10, Severity::Error, "Off by one in the loop bound"),
        ];
        let deduped = dedupe_reports(reports);
        assert_eq!(deduped.len(), 1);
        assert_eq!(deduped[0].severity, Severity::Error);
    }

    #[test]
    fn distinct_findings_at_the_same_line_both_survive() {
        let reports = vec![
            finding("a.rs", 10, Severity::Error, "Division by zero"),
            finding(
                "a.rs",
                10,
                Severity::Error,
                "Unvalidated user input reaches the query",
            ),
        ];
        assert_eq!(dedupe_reports(reports).len(), 2);
    }

    #[test]
    fn the_same_message_in_two_files_is_not_a_duplicate() {
        let reports = vec![
            finding("a.rs", 10, Severity::Error, "Division by zero"),
            finding("b.rs", 10, Severity::Error, "Division by zero"),
        ];
        assert_eq!(dedupe_reports(reports).len(), 2);
    }

    #[test]
    fn errors_sort_before_warnings() {
        let reports = vec![
            finding("z.rs", 1, Severity::Info, "note"),
            finding("a.rs", 1, Severity::Error, "boom"),
            finding("m.rs", 1, Severity::Warning, "hmm"),
        ];
        let deduped = dedupe_reports(reports);
        assert_eq!(deduped[0].severity, Severity::Error);
        assert_eq!(deduped[2].severity, Severity::Info);
    }

    // ── ReviewCoverage ───────────────────────────────────────────────────────

    #[test]
    fn complete_coverage_has_no_caveat() {
        let coverage = ReviewCoverage {
            files_total: 3,
            files_reviewed: 3,
            llm_calls: 1,
            ..Default::default()
        };
        assert!(coverage.is_complete());
        assert_eq!(coverage.caveat(), None);
    }

    #[test]
    fn skipped_and_truncated_files_both_produce_a_caveat() {
        let coverage = ReviewCoverage {
            files_total: 5,
            files_reviewed: 3,
            llm_calls: 8,
            files_truncated: vec!["big.rs".into()],
            files_skipped: vec!["x.rs".into(), "y.rs".into()],
            ..Default::default()
        };
        assert!(!coverage.is_complete());
        let caveat = coverage.caveat().expect("coverage was incomplete");
        assert!(caveat.contains("2 file(s) not reviewed (call budget)"));
        assert!(caveat.contains("1 file(s) truncated"));
    }

    #[test]
    fn a_provider_failure_is_not_coverage() {
        // With the provider down, the static scan still runs — but claiming the
        // file was reviewed turns an outage into a clean bill of health.
        let coverage = ReviewCoverage {
            files_total: 1,
            files_reviewed: 0,
            llm_calls: 1,
            llm_calls_failed: 1,
            files_provider_failed: vec!["src/math.py".into()],
            ..Default::default()
        };
        assert!(!coverage.is_complete());
        let caveat = coverage.caveat().expect("a failed call is not coverage");
        assert!(caveat.contains("1 file(s) not reviewed (provider error)"));
    }

    #[test]
    fn default_plan_covers_far_more_than_one_request() {
        let plan = ReviewPlan::default();
        assert_eq!(plan.passes, 1);
        assert!(plan.char_budget * plan.max_calls >= 64_000);
    }

    // ── review_diff_planned (provider outcomes) ──────────────────────────────

    /// A provider whose every call fails, standing in for an outage, a rate
    /// limit, or a missing API key.
    struct FailingProvider;

    #[async_trait::async_trait]
    impl LLMProvider for FailingProvider {
        fn name(&self) -> &str {
            "failing"
        }
        async fn is_available(&self) -> bool {
            false
        }
        async fn complete(
            &self,
            _ctx: &vibe_ai::provider::CodeContext,
        ) -> Result<vibe_ai::provider::CompletionResponse> {
            anyhow::bail!("provider is down")
        }
        async fn stream_complete(
            &self,
            _ctx: &vibe_ai::provider::CodeContext,
        ) -> Result<vibe_ai::provider::CompletionStream> {
            anyhow::bail!("provider is down")
        }
        async fn chat(&self, _m: &[Message], _c: Option<String>) -> Result<String> {
            anyhow::bail!("provider is down")
        }
        async fn stream_chat(&self, _m: &[Message]) -> Result<vibe_ai::provider::CompletionStream> {
            anyhow::bail!("provider is down")
        }
    }

    #[tokio::test]
    async fn a_failed_model_call_reports_the_file_as_unreviewed() {
        // Regression: coverage used to print "1/1 file(s) reviewed" after the
        // only model call errored, so an outage read as a clean review.
        let bot = BugBot {
            llm: Arc::new(FailingProvider),
            gh_token: None,
        };
        let diff = section("src/math.py", "x = 1");
        let (_reports, coverage) = bot.review_diff_planned(&diff, ReviewPlan::default()).await;

        assert_eq!(coverage.files_total, 1);
        assert_eq!(coverage.files_reviewed, 0);
        assert_eq!(coverage.llm_calls, 1);
        assert_eq!(coverage.llm_calls_failed, 1);
        assert_eq!(
            coverage.files_provider_failed,
            vec!["src/math.py".to_string()]
        );
        assert!(!coverage.is_complete());
    }

    #[tokio::test]
    async fn the_static_scan_still_runs_when_the_provider_is_down() {
        let bot = BugBot {
            llm: Arc::new(FailingProvider),
            gh_token: None,
        };
        let diff = section(
            "app.py",
            "API_KEY = \"sk-live-abcdef0123456789abcdef0123456789\"",
        );
        let (reports, coverage) = bot.review_diff_planned(&diff, ReviewPlan::default()).await;

        assert!(
            !reports.is_empty(),
            "the deterministic scan does not depend on the provider"
        );
        assert!(
            !coverage.is_complete(),
            "but the review is still not complete"
        );
    }

    #[tokio::test]
    async fn an_empty_diff_costs_no_model_calls() {
        let bot = BugBot {
            llm: Arc::new(FailingProvider),
            gh_token: None,
        };
        let (reports, coverage) = bot.review_diff_planned("", ReviewPlan::default()).await;
        assert!(reports.is_empty());
        assert_eq!(coverage.llm_calls, 0);
        assert!(coverage.is_complete());
    }

    // ── parse_reports ────────────────────────────────────────────────────────

    #[test]
    fn parses_a_findings_array_out_of_prose() {
        let reply = "Here you go:\n[{\"file\":\"a.rs\",\"line\":1,\"severity\":\"error\",\"message\":\"m\"}]\nDone";
        let reports = parse_reports(reply);
        assert_eq!(reports.len(), 1);
        assert_eq!(reports[0].file, "a.rs");
    }

    #[test]
    fn a_reply_with_no_array_yields_no_findings() {
        assert!(parse_reports("I found nothing.").is_empty());
        assert!(parse_reports("").is_empty());
    }

    #[test]
    fn reasoning_is_not_mistaken_for_the_findings_array() {
        // The reasoning numbers its candidates, so it holds a `[` earlier than
        // the real array and a `]` the scan would otherwise stop at.
        let reply = "<thinking>Candidates: [1] the null check, [2] the retry.</thinking>\n\
                     [{\"file\":\"a.rs\",\"line\":1,\"severity\":\"error\",\"message\":\"m\"}]";
        let reports = parse_reports(reply);
        assert_eq!(reports.len(), 1);
        assert_eq!(reports[0].file, "a.rs");
        assert!(!reports[0].message.contains("thinking"));
    }

    #[test]
    fn an_unclosed_reasoning_block_yields_no_findings() {
        // Cut off mid-reasoning: there is no answer in here, and posting the
        // deliberation as a review comment would be worse than posting nothing.
        assert!(parse_reports("<thinking>Looking at [1] the null check").is_empty());
    }

    #[test]
    fn malformed_json_yields_no_findings_rather_than_a_panic() {
        assert!(parse_reports("[{\"file\": }]").is_empty());
    }

    // ── review_prompt ────────────────────────────────────────────────────────

    #[test]
    fn the_prompt_contains_every_file_in_the_batch() {
        let sections = split_diff_by_file(&format!(
            "{}{}",
            section("a.rs", "one"),
            section("b.rs", "two")
        ));
        let prompt = review_prompt(&rotate(&sections, 0));
        assert!(prompt.contains("+one"));
        assert!(prompt.contains("+two"));
        assert!(prompt.contains("new side of the diff"));
    }

    #[test]
    fn format_empty_reports() {
        let output = BugBot::format_reports(&[]);
        assert!(output.contains("no issues"));
    }

    #[test]
    fn format_reports_with_issues() {
        let reports = vec![BugReport {
            file: "src/main.rs".to_string(),
            line: 42,
            severity: Severity::Error,
            message: "Division by zero".to_string(),
            suggestion: Some("Add guard".to_string()),
            fix_command: None,
            category: Some("logic".to_string()),
        }];
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

    #[test]
    fn detect_hardcoded_secret() {
        let diff = r#"diff --git a/src/config.rs b/src/config.rs
--- a/src/config.rs
+++ b/src/config.rs
@@ -1,3 +1,4 @@
 fn setup() {
+    let api_key = "sk-abc123def456ghij";
 }
"#;
        let reports = detect_security_patterns(diff);
        assert!(!reports.is_empty(), "should detect hardcoded secret");
        assert!(reports[0].message.contains("CWE-798"));
        assert_eq!(reports[0].file, "src/config.rs");
        assert_eq!(reports[0].line, 2);
    }

    #[test]
    fn detect_xss_pattern() {
        let diff = r#"diff --git a/src/ui.ts b/src/ui.ts
--- a/src/ui.ts
+++ b/src/ui.ts
@@ -5,3 +5,4 @@
 function render(data: string) {
+    el.innerHTML = data;
 }
"#;
        let reports = detect_security_patterns(diff);
        assert!(!reports.is_empty());
        assert!(reports[0].message.contains("CWE-79"));
    }

    #[test]
    fn clean_diff_has_no_static_findings() {
        let diff = r#"diff --git a/src/main.rs b/src/main.rs
--- a/src/main.rs
+++ b/src/main.rs
@@ -1,3 +1,4 @@
 fn main() {
+    println!("hello, world");
 }
"#;
        let reports = detect_security_patterns(diff);
        assert!(reports.is_empty(), "clean diff should yield no findings");
    }

    #[test]
    fn detect_ssrf_pattern() {
        let diff = r#"diff --git a/src/proxy.py b/src/proxy.py
--- a/src/proxy.py
+++ b/src/proxy.py
@@ -1,3 +1,4 @@
 def proxy(request):
+    resp = requests.get(url=request.params.target_url)
 }
"#;
        let reports = detect_security_patterns(diff);
        assert!(!reports.is_empty(), "should detect SSRF");
        assert!(reports[0].message.contains("CWE-918"));
    }

    #[test]
    fn detect_insecure_deserialization() {
        let diff = r#"diff --git a/src/handler.py b/src/handler.py
--- a/src/handler.py
+++ b/src/handler.py
@@ -1,3 +1,4 @@
 def load(data):
+    obj = pickle.loads(data)
 }
"#;
        let reports = detect_security_patterns(diff);
        assert!(
            !reports.is_empty(),
            "should detect insecure deserialization"
        );
        assert!(reports[0].message.contains("CWE-502"));
    }

    #[test]
    fn detect_nosql_injection() {
        let diff = r#"diff --git a/src/users.js b/src/users.js
--- a/src/users.js
+++ b/src/users.js
@@ -1,3 +1,4 @@
 function getUser(req) {
+    db.users.find({ $where: "this.name == '" + req.body.name + "'" })
 }
"#;
        let reports = detect_security_patterns(diff);
        assert!(!reports.is_empty(), "should detect NoSQL injection");
        assert!(reports[0].message.contains("CWE-943"));
    }

    #[test]
    fn detect_cleartext_api() {
        let diff = r#"diff --git a/src/config.ts b/src/config.ts
--- a/src/config.ts
+++ b/src/config.ts
@@ -1,3 +1,4 @@
 const config = {
+    apiUrl: "http://payments.example.com/api/charge",
 }
"#;
        let reports = detect_security_patterns(diff);
        assert!(!reports.is_empty(), "should detect cleartext HTTP API");
        assert!(reports[0].message.contains("CWE-319"));
    }

    #[test]
    fn line_counter_accuracy_with_context_lines() {
        // @@ -1,0 +1,5 @$ header means new-file starts at line 1.
        // 3 context lines + 1 added line with a secret → the secret is on line 4.
        let diff = r#"diff --git a/src/app.rs b/src/app.rs
--- a/src/app.rs
+++ b/src/app.rs
@@ -1,0 +1,5 @@
 line one
 line two
 line three
+    let api_key = "sk-AAAA1234BBBB5678";
 line five
"#;
        let reports = detect_security_patterns(diff);
        assert_eq!(reports.len(), 1, "exactly one finding expected");
        assert!(reports[0].message.contains("CWE-798"));
        assert_eq!(reports[0].file, "src/app.rs");
        // 3 context lines advance counter to 3, then the added line increments to 4.
        assert_eq!(reports[0].line, 4, "secret should be reported at line 4");
    }

    #[test]
    fn removed_lines_not_reported() {
        // A line prefixed with `-` that contains a secret should NOT generate a finding
        // because it is being removed, not added.
        let diff = r#"diff --git a/src/old.rs b/src/old.rs
--- a/src/old.rs
+++ b/src/old.rs
@@ -1,4 +1,3 @@
 fn main() {
-    let api_key = "sk-REMOVEDKEY12345678";
     println!("clean now");
 }
"#;
        let reports = detect_security_patterns(diff);
        assert!(
            reports.is_empty(),
            "removed lines with secrets should not be reported"
        );
    }

    #[test]
    fn no_security_issues_returns_empty() {
        let diff = r#"diff --git a/src/lib.rs b/src/lib.rs
--- a/src/lib.rs
+++ b/src/lib.rs
@@ -10,6 +10,8 @@
 use std::collections::HashMap;

+fn add(a: i32, b: i32) -> i32 {
+    a + b
+}

 fn existing() {}
"#;
        let reports = detect_security_patterns(diff);
        assert!(
            reports.is_empty(),
            "diff with no security issues should return empty vec"
        );
    }

    #[test]
    fn bugreport_icon_all_variants() {
        let make_report = |severity: Severity| BugReport {
            file: "f.rs".to_string(),
            line: 1,
            severity,
            message: "test".to_string(),
            suggestion: None,
            fix_command: None,
            category: None,
        };

        let error_report = make_report(Severity::Error);
        assert_eq!(error_report.icon(), "❌");

        let warning_report = make_report(Severity::Warning);
        assert_eq!(warning_report.icon(), "⚠️ ");

        let info_report = make_report(Severity::Info);
        assert_eq!(info_report.icon(), "ℹ️ ");
    }
}
