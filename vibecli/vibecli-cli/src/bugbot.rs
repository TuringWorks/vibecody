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
            Regex::new(pat).ok().map(|re| (re, *cwe, sev.clone(), *msg, *sug))
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
                    .split(',').next().unwrap_or("0")
                    .split(' ').next().unwrap_or("0");
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

        // Run static OWASP/CWE pattern scan first — fast, no LLM required.
        let mut static_reports = detect_security_patterns(diff);

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
            {
                let end = diff.char_indices().nth(8000).map(|(i, _)| i).unwrap_or(diff.len());
                &diff[..end]
            }
        );

        let msgs = vec![Message { role: MessageRole::User, content: prompt }];

        let mut llm_reports = match self.llm.chat(&msgs, None).await {
            Ok(response) => {
                let json_start = response.find('[').unwrap_or(0);
                let json_end = response.rfind(']').map(|i| i + 1).unwrap_or(response.len());
                if json_start < json_end {
                    let json_str = &response[json_start..json_end];
                    serde_json::from_str::<Vec<BugReport>>(json_str).unwrap_or_default()
                } else {
                    vec![]
                }
            }
            Err(_) => vec![],
        };

        // Static reports first (deterministic), then LLM additions.
        static_reports.append(&mut llm_reports);
        static_reports
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
        let client = reqwest::Client::builder()
            .timeout(std::time::Duration::from_secs(30))
            .connect_timeout(std::time::Duration::from_secs(10))
            .build()?;
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

        let client = reqwest::Client::builder()
            .timeout(std::time::Duration::from_secs(30))
            .connect_timeout(std::time::Duration::from_secs(10))
            .build()?;
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
        assert!(!reports.is_empty(), "should detect insecure deserialization");
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
}
