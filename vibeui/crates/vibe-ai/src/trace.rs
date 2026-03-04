//! Structured JSONL trace/audit log for agent sessions.
//!
//! Each agent run writes entries to a configurable directory
//! (VibeCLI uses `~/.vibecli/traces/<unix_secs>.jsonl`).
//! Entries capture every tool call, result, timing, and approval source.
//!
//! # Session Resume
//!
//! `TraceWriter::save_messages()` persists the full message history to
//! `<session_id>-messages.json` in the same directory. `load_session()` can
//! then restore a prior conversation for `vibecli --resume <session-id>`.

use crate::agent::AgentContext;
use crate::provider::Message;
use serde::{Deserialize, Serialize};
use std::fs::{self, File, OpenOptions};
use std::io::{BufRead, BufReader, Write};
use std::path::{Path, PathBuf};
use std::time::{SystemTime, UNIX_EPOCH};

// ── TraceEntry ────────────────────────────────────────────────────────────────

/// A single trace entry — one JSONL row.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TraceEntry {
    /// Unix timestamp (seconds).
    pub timestamp: u64,
    /// Identifies the session this entry belongs to.
    pub session_id: String,
    /// 0-based step index within the agent loop.
    pub step: usize,
    /// Tool name ("read_file", "bash", "write_file", …).
    pub tool: String,
    /// Short human-readable summary of the tool call input.
    pub input_summary: String,
    /// Tool output (truncated to 600 chars if longer).
    pub output: String,
    /// Whether the tool call succeeded.
    pub success: bool,
    /// Elapsed time to execute this step.
    pub duration_ms: u64,
    /// Who approved the action: `"user"` | `"auto"` | `"ci-auto"` | `"rejected"`.
    pub approved_by: String,
}

// ── TraceWriter ───────────────────────────────────────────────────────────────

/// Appends [`TraceEntry`] records to a JSONL file in `dir`.
pub struct TraceWriter {
    session_id: String,
    path: PathBuf,
}

impl TraceWriter {
    /// Create a new writer.  The file is created lazily on the first
    /// [`record`] call, so construction never fails.
    pub fn new(dir: PathBuf) -> Self {
        let _ = fs::create_dir_all(&dir);
        let session_id = format!("{}", now_secs());
        let path = dir.join(format!("{}.jsonl", &session_id));
        Self { session_id, path }
    }

    /// Like [`new`] but prefixes the session ID with a human-readable name.
    /// Useful for `--session-name` so traces are easy to identify.
    /// Example: `new_named(dir, "my-task")` → `"my-task-1700000000.jsonl"`
    pub fn new_named(dir: PathBuf, name: &str) -> Self {
        let _ = fs::create_dir_all(&dir);
        let session_id = format!("{}-{}", name, now_secs());
        let path = dir.join(format!("{}.jsonl", &session_id));
        Self { session_id, path }
    }

    /// Unique identifier for this session (also the stem of the trace file).
    pub fn session_id(&self) -> &str {
        &self.session_id
    }

    /// Absolute path to the JSONL file.
    pub fn path(&self) -> &PathBuf {
        &self.path
    }

    /// Append one entry to the log.
    #[allow(clippy::too_many_arguments)]
    pub fn record(
        &self,
        step: usize,
        tool: &str,
        input_summary: &str,
        output: &str,
        success: bool,
        duration_ms: u64,
        approved_by: &str,
    ) {
        let entry = TraceEntry {
            timestamp: now_secs(),
            session_id: self.session_id.clone(),
            step,
            tool: tool.to_string(),
            input_summary: redact_secrets(input_summary),
            output: {
                let truncated = if output.len() > 600 {
                    let safe_end = output.char_indices().nth(600).map(|(i,_)| i).unwrap_or(output.len());
                    format!("{}\n…(truncated)", &output[..safe_end])
                } else {
                    output.to_string()
                };
                redact_secrets(&truncated)
            },
            success,
            duration_ms,
            approved_by: approved_by.to_string(),
        };
        if let Ok(line) = serde_json::to_string(&entry) {
            if let Ok(mut f) = OpenOptions::new().create(true).append(true).open(&self.path) {
                let _ = writeln!(f, "{}", line);
            }
        }
    }

    /// Persist the full message history for this session.
    /// Saved as `<session_id>-messages.json` alongside the JSONL trace.
    /// Secrets are scrubbed before writing.
    pub fn save_messages(&self, messages: &[Message]) -> std::io::Result<()> {
        let path = self.messages_path();
        let json = serde_json::to_string_pretty(messages)
            .map_err(std::io::Error::other)?;
        fs::write(path, redact_secrets(&json))
    }

    /// Persist the agent context snapshot for this session.
    pub fn save_context(&self, ctx: &AgentContext) -> std::io::Result<()> {
        let path = self.context_path();
        let json = serde_json::to_string_pretty(ctx)
            .map_err(std::io::Error::other)?;
        fs::write(path, json)
    }

    fn messages_path(&self) -> PathBuf {
        self.path
            .parent()
            .unwrap_or_else(|| std::path::Path::new("."))
            .join(format!("{}-messages.json", self.session_id))
    }

    fn context_path(&self) -> PathBuf {
        self.path
            .parent()
            .unwrap_or_else(|| std::path::Path::new("."))
            .join(format!("{}-context.json", self.session_id))
    }
}

// ── SessionSnapshot ───────────────────────────────────────────────────────────

/// All the data needed to resume a previous agent session.
#[derive(Debug, Clone)]
pub struct SessionSnapshot {
    pub session_id: String,
    pub messages: Vec<Message>,
    pub context: Option<AgentContext>,
    pub trace: Vec<TraceEntry>,
}

/// Load a full session snapshot by ID for `vibecli --resume <session_id>`.
pub fn load_session(session_id: &str, dir: &Path) -> Option<SessionSnapshot> {
    let jsonl_path = dir.join(format!("{}.jsonl", session_id));
    if !jsonl_path.exists() {
        return None;
    }

    let trace = load_trace(&jsonl_path);

    let messages_path = dir.join(format!("{}-messages.json", session_id));
    let messages: Vec<Message> = if messages_path.exists() {
        fs::read_to_string(&messages_path)
            .ok()
            .and_then(|s| serde_json::from_str(&s).ok())
            .unwrap_or_default()
    } else {
        vec![]
    };

    let context_path = dir.join(format!("{}-context.json", session_id));
    let context: Option<AgentContext> = if context_path.exists() {
        fs::read_to_string(&context_path)
            .ok()
            .and_then(|s| serde_json::from_str(&s).ok())
    } else {
        None
    };

    Some(SessionSnapshot {
        session_id: session_id.to_string(),
        messages,
        context,
        trace,
    })
}

// ── Listing / Loading ─────────────────────────────────────────────────────────

/// Summary of a past trace session.
#[derive(Debug, Clone)]
pub struct TraceSession {
    /// Session ID (unix seconds as string).
    pub session_id: String,
    /// Creation timestamp (unix seconds).
    pub timestamp: u64,
    /// Path to the JSONL file.
    pub path: PathBuf,
    /// Number of trace entries recorded.
    pub step_count: usize,
}

/// List all trace sessions in `dir`, sorted newest-first.
pub fn list_traces(dir: &Path) -> Vec<TraceSession> {
    let Ok(entries) = fs::read_dir(dir) else {
        return vec![];
    };
    let mut sessions: Vec<TraceSession> = entries
        .flatten()
        .filter_map(|e| {
            let name = e.file_name().to_string_lossy().to_string();
            name.strip_suffix(".jsonl").map(|id| {
                let ts: u64 = id.parse().unwrap_or(0);
                let count = count_lines(&e.path()).unwrap_or(0);
                TraceSession {
                    session_id: id.to_string(),
                    timestamp: ts,
                    path: e.path(),
                    step_count: count,
                }
            })
        })
        .collect();
    sessions.sort_by(|a, b| b.timestamp.cmp(&a.timestamp));
    sessions
}

/// Load all [`TraceEntry`] records from a single JSONL file.
pub fn load_trace(path: &Path) -> Vec<TraceEntry> {
    let Ok(f) = File::open(path) else {
        return vec![];
    };
    BufReader::new(f)
        .lines()
        .map_while(Result::ok)
        .filter_map(|l| serde_json::from_str(&l).ok())
        .collect()
}

// ── Secrets scrubbing ────────────────────────────────────────────────────────

/// Redact common secret patterns from a string before persisting to traces.
///
/// Matches API keys (sk-*, ghp_*, Bearer tokens, etc.),
/// AWS credentials, private keys, and passwords in common config formats.
pub fn redact_secrets(input: &str) -> String {
    use std::sync::OnceLock;

    static RE: OnceLock<Vec<(regex::Regex, &'static str)>> = OnceLock::new();
    let patterns = RE.get_or_init(|| {
        [
            // OpenAI / Anthropic / generic sk- keys
            (r"sk-[a-zA-Z0-9_-]{20,}", "[REDACTED_API_KEY]"),
            // GitHub tokens (ghp_, gho_, ghs_, ghr_, github_pat_)
            (r"gh[psohr]_[a-zA-Z0-9_]{20,}", "[REDACTED_GITHUB_TOKEN]"),
            (r"github_pat_[a-zA-Z0-9_]{20,}", "[REDACTED_GITHUB_TOKEN]"),
            // Bearer tokens in headers
            (r"(?i)(Bearer\s+)[a-zA-Z0-9_.+/=-]{20,}", "${1}[REDACTED]"),
            // AWS access keys (AKIA...)
            (r"AKIA[A-Z0-9]{16}", "[REDACTED_AWS_KEY]"),
            // AWS secret keys (40 char base64-like after known prefixes)
            (r"(?i)(aws_secret_access_key\s*[=:]\s*)[A-Za-z0-9/+=]{30,}", "${1}[REDACTED]"),
            // Generic password/secret/token in config-like lines (exclude '[' to avoid re-redacting)
            (r#"(?i)((?:password|secret|token|api_key|apikey|api-key)\s*[=:]\s*["']?)[^\s"'\[]{8,}"#, "${1}[REDACTED]"),
            // API key in URL query param (?key=...)
            (r"(?i)([?&]key=)[a-zA-Z0-9_-]{20,}", "${1}[REDACTED]"),
            // Private key blocks
            (r"(?s)(-----BEGIN (?:RSA |EC |DSA |OPENSSH )?PRIVATE KEY-----).*?(-----END (?:RSA |EC |DSA |OPENSSH )?PRIVATE KEY-----)", "$1\n[REDACTED]\n$2"),
        ]
        .iter()
        .filter_map(|(pat, replacement)| {
            regex::Regex::new(pat).ok().map(|re| (re, *replacement))
        })
        .collect()
    });

    let mut result = input.to_string();
    for (re, replacement) in patterns {
        result = re.replace_all(&result, *replacement).to_string();
    }
    result
}

// ── Helpers ───────────────────────────────────────────────────────────────────

fn now_secs() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs()
}

fn count_lines(path: &Path) -> std::io::Result<usize> {
    let f = File::open(path)?;
    Ok(BufReader::new(f).lines().count())
}

// ── Tests ─────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;
    use std::env::temp_dir;

    #[test]
    fn long_output_truncated() {
        let dir = temp_dir().join(format!("vibe_trace_trunc_{}", now_secs()));
        let writer = TraceWriter::new(dir.clone());
        let long = "x".repeat(800);
        writer.record(0, "bash", "bash(cmd)", &long, true, 1, "auto");

        let sessions = list_traces(&dir);
        let entries = load_trace(&sessions[0].path);
        assert!(entries[0].output.len() < 700, "output should be truncated");
        assert!(entries[0].output.contains("truncated"));

        let _ = fs::remove_dir_all(&dir);
    }

    #[test]
    fn redact_openai_key() {
        let input = "Using API key sk-abcdefghij1234567890abcdefghij to call GPT";
        let result = redact_secrets(input);
        assert!(!result.contains("sk-abcdefghij"), "OpenAI key should be redacted");
        assert!(result.contains("[REDACTED_API_KEY]"));
    }

    #[test]
    fn redact_github_token() {
        let input = "token=ghp_xyzABCDEFGHIJ1234567890abcdef";
        let result = redact_secrets(input);
        assert!(!result.contains("ghp_xyz"), "GitHub token should be redacted");
        assert!(result.contains("[REDACTED_GITHUB_TOKEN]"));
    }

    #[test]
    fn redact_bearer_token() {
        let input = "Authorization: Bearer eyJhbGciOiJIUzI1NiIsInR5cCI6IkpXVCJ9.eyJzdWIiOiIxMjM0NTY3ODkwIn0";
        let result = redact_secrets(input);
        assert!(!result.contains("eyJhbGci"), "Bearer token should be redacted");
        assert!(result.contains("Bearer [REDACTED]"));
    }

    #[test]
    fn redact_aws_key() {
        let input = "AWS_ACCESS_KEY_ID=AKIAIOSFODNN7EXAMPLE";
        let result = redact_secrets(input);
        assert!(!result.contains("AKIAIOSFODNN7EXAMPLE"), "AWS key should be redacted");
    }

    #[test]
    fn redact_api_key_in_url() {
        let input = "https://api.example.com/v1/generate?key=AIzaSyDGHJKL1234567890abcdef";
        let result = redact_secrets(input);
        assert!(!result.contains("AIzaSy"), "URL API key should be redacted");
        assert!(result.contains("?key=[REDACTED]"));
    }

    #[test]
    fn redact_password_in_config() {
        let input = r#"password = "superSecretPass123!""#;
        let result = redact_secrets(input);
        assert!(!result.contains("superSecretPass"), "Password should be redacted");
    }

    #[test]
    fn no_false_positive_on_short_values() {
        let input = "Using model gpt-4 with temperature 0.7";
        let result = redact_secrets(input);
        assert_eq!(input, result, "Short normal values should not be redacted");
    }

    #[test]
    fn empty_dir_returns_empty() {
        let dir = temp_dir().join(format!("vibe_trace_empty_{}", now_secs()));
        let sessions = list_traces(&dir);
        assert!(sessions.is_empty());
    }

    #[test]
    fn write_and_load() {
        let dir = temp_dir().join(format!("vibe_trace_test_{}", now_secs()));
        let writer = TraceWriter::new(dir.clone());
        writer.record(0, "read_file", "read_file(src/main.rs)", "fn main() {}", true, 5, "auto");
        writer.record(1, "bash", "bash(cargo build)", "[exit 0]", true, 1200, "user");

        let sessions = list_traces(&dir);
        assert_eq!(sessions.len(), 1);
        assert_eq!(sessions[0].step_count, 2);

        let entries = load_trace(&sessions[0].path);
        assert_eq!(entries.len(), 2);
        assert_eq!(entries[0].tool, "read_file");
        assert_eq!(entries[1].tool, "bash");
        assert_eq!(entries[1].approved_by, "user");

        // Clean up
        let _ = fs::remove_dir_all(&dir);
    }
}
