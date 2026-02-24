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

    /// Unique identifier for this session (also the stem of the trace file).
    pub fn session_id(&self) -> &str {
        &self.session_id
    }

    /// Absolute path to the JSONL file.
    pub fn path(&self) -> &PathBuf {
        &self.path
    }

    /// Append one entry to the log.
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
            input_summary: input_summary.to_string(),
            output: if output.len() > 600 {
                format!("{}\n…(truncated)", &output[..600])
            } else {
                output.to_string()
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
    pub fn save_messages(&self, messages: &[Message]) -> std::io::Result<()> {
        let path = self.messages_path();
        let json = serde_json::to_string_pretty(messages)
            .map_err(|e| std::io::Error::new(std::io::ErrorKind::Other, e))?;
        fs::write(path, json)
    }

    /// Persist the agent context snapshot for this session.
    pub fn save_context(&self, ctx: &AgentContext) -> std::io::Result<()> {
        let path = self.context_path();
        let json = serde_json::to_string_pretty(ctx)
            .map_err(|e| std::io::Error::new(std::io::ErrorKind::Other, e))?;
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
        .flatten()
        .filter_map(|l| serde_json::from_str(&l).ok())
        .collect()
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
