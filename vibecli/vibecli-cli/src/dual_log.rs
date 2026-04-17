//! Dual-file session logs: append-only log.jsonl + compacted context.jsonl.
//! Pi-mono gap bridge: Phase B4.
//!
//! Architecture:
//! - `log.jsonl`     — append-only, infinite history, never compacted.
//! - `context.jsonl` — bounded LLM context, compacted when full.
//!
//! Before each agent turn call `sync_context()` to pull any new entries
//! from the full log into the context window.  Use `grep_log()` to search
//! history that has already been evicted from the context window without
//! sending it to the LLM.

use std::collections::HashMap;
use std::path::Path;

// ---------------------------------------------------------------------------
// LogRole
// ---------------------------------------------------------------------------

/// The speaker role for a log entry (mirrors the standard LLM message roles).
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum LogRole {
    User,
    Assistant,
    System,
    Tool,
}

impl LogRole {
    pub fn as_str(&self) -> &str {
        match self {
            LogRole::User => "user",
            LogRole::Assistant => "assistant",
            LogRole::System => "system",
            LogRole::Tool => "tool",
        }
    }

    #[allow(clippy::should_implement_trait)]
    pub fn from_str(s: &str) -> Self {
        match s {
            "assistant" => LogRole::Assistant,
            "system" => LogRole::System,
            "tool" => LogRole::Tool,
            _ => LogRole::User,
        }
    }
}

// ---------------------------------------------------------------------------
// LogEntry
// ---------------------------------------------------------------------------

/// A single entry stored in either the full log or the context window.
#[derive(Debug, Clone)]
pub struct LogEntry {
    pub id: String,
    pub role: LogRole,
    pub content: String,
    pub timestamp_ms: u64,
    /// True when this entry was synthesised during compaction.
    pub is_compacted: bool,
    pub metadata: HashMap<String, String>,
}

impl LogEntry {
    /// Create a normal (non-compacted) entry.
    pub fn new(
        id: impl Into<String>,
        role: LogRole,
        content: impl Into<String>,
        ts: u64,
    ) -> Self {
        Self {
            id: id.into(),
            role,
            content: content.into(),
            timestamp_ms: ts,
            is_compacted: false,
            metadata: HashMap::new(),
        }
    }

    /// Create a synthetic compaction-summary entry.
    pub fn compaction_summary(summary: impl Into<String>, ts: u64) -> Self {
        Self {
            id: format!("compact-{ts}"),
            role: LogRole::System,
            content: summary.into(),
            timestamp_ms: ts,
            is_compacted: true,
            metadata: HashMap::new(),
        }
    }

    // ── Serialisation ────────────────────────────────────────────────────────

    /// Serialise this entry as a single JSON line (no trailing newline).
    pub fn to_jsonl_line(&self) -> String {
        // Build a minimal hand-rolled JSON object to avoid pulling in serde.
        let meta_pairs: Vec<String> = self
            .metadata
            .iter()
            .map(|(k, v)| format!("\"{}\":\"{}\"", escape_json(k), escape_json(v)))
            .collect();
        let meta_json = format!("{{{}}}", meta_pairs.join(","));

        format!(
            "{{\"id\":\"{id}\",\"role\":\"{role}\",\"content\":{content},\
             \"timestamp_ms\":{ts},\"is_compacted\":{compact},\"metadata\":{meta}}}",
            id = escape_json(&self.id),
            role = self.role.as_str(),
            content = json_string(&self.content),
            ts = self.timestamp_ms,
            compact = self.is_compacted,
            meta = meta_json,
        )
    }

    /// Parse a single JSON line back into a `LogEntry`.
    pub fn from_jsonl_line(line: &str) -> Result<Self, String> {
        let line = line.trim();
        if line.is_empty() {
            return Err("empty line".into());
        }

        let id = extract_str_field(line, "id")?;
        let role_str = extract_str_field(line, "role")?;
        let content = extract_str_field(line, "content")?;
        let timestamp_ms = extract_u64_field(line, "timestamp_ms")?;
        let is_compacted = extract_bool_field(line, "is_compacted")?;
        let metadata = extract_metadata(line);

        Ok(Self {
            id,
            role: LogRole::from_str(&role_str),
            content,
            timestamp_ms,
            is_compacted,
            metadata,
        })
    }
}

// ---------------------------------------------------------------------------
// DualLog
// ---------------------------------------------------------------------------

/// Dual-log manager for a single session/channel.
///
/// `full_log` is the append-only record of every message ever sent.
/// `context` is the bounded slice that is actually handed to the LLM.
#[derive(Debug)]
pub struct DualLog {
    /// All entries ever appended — the append-only log.
    full_log: Vec<LogEntry>,
    /// Entries in the active LLM context window.
    context: Vec<LogEntry>,
    /// Index into `full_log`: everything below this index has already been
    /// copied into `context`.
    sync_watermark: usize,
    /// Hard cap on the number of entries kept in `context`.
    max_context_entries: usize,
}

impl DualLog {
    /// Create a new, empty dual-log.
    pub fn new(max_context_entries: usize) -> Self {
        Self {
            full_log: Vec::new(),
            context: Vec::new(),
            sync_watermark: 0,
            max_context_entries,
        }
    }

    // ── Mutation ─────────────────────────────────────────────────────────────

    /// Append a new entry to the full log (does not automatically sync context).
    pub fn append(&mut self, entry: LogEntry) {
        self.full_log.push(entry);
    }

    /// Pull any new full-log entries (since the last sync) into the context.
    ///
    /// When the context would exceed `max_context_entries` the oldest entries
    /// are dropped to make room (they remain safe in `full_log`).
    /// Call this before every agent turn.
    pub fn sync_context(&mut self) {
        let new_entries = &self.full_log[self.sync_watermark..];
        for entry in new_entries {
            if self.context.len() >= self.max_context_entries {
                // Evict the oldest entry from context to make room.
                self.context.remove(0);
            }
            self.context.push(entry.clone());
        }
        self.sync_watermark = self.full_log.len();
    }

    /// Compact the context: replace the oldest `(len - keep_recent)` entries
    /// with a single summary entry.  The full log is *never* modified.
    pub fn compact(&mut self, summary: &str, keep_recent: usize) {
        if self.context.len() <= keep_recent {
            return;
        }
        let ts = self
            .context
            .last()
            .map(|e| e.timestamp_ms)
            .unwrap_or(0);
        let summary_entry = LogEntry::compaction_summary(summary, ts);
        let tail_start = self.context.len() - keep_recent;
        let tail: Vec<LogEntry> = self.context.drain(tail_start..).collect();
        self.context.clear();
        self.context.push(summary_entry);
        self.context.extend(tail);
    }

    // ── Queries ──────────────────────────────────────────────────────────────

    /// Search the full log (including history evicted from context) for entries
    /// whose `content` contains `pattern` (case-sensitive substring match).
    /// Use this to retrieve historical entries without adding them to the LLM
    /// context window.
    pub fn grep_log(&self, pattern: &str) -> Vec<&LogEntry> {
        self.full_log
            .iter()
            .filter(|e| e.content.contains(pattern))
            .collect()
    }

    /// All entries currently in the LLM context window.
    pub fn context_entries(&self) -> &[LogEntry] {
        &self.context
    }

    /// All entries ever appended (the append-only full log).
    pub fn full_log_entries(&self) -> &[LogEntry] {
        &self.full_log
    }

    /// Total number of entries in the full log.
    pub fn full_log_count(&self) -> usize {
        self.full_log.len()
    }

    /// Number of entries currently in the context window.
    pub fn context_count(&self) -> usize {
        self.context.len()
    }

    /// Entries that have been appended to the full log but not yet synced into
    /// the context window.
    pub fn unsynced_count(&self) -> usize {
        self.full_log.len().saturating_sub(self.sync_watermark)
    }

    /// True when the context window is at its maximum capacity.
    pub fn is_context_full(&self) -> bool {
        self.context.len() >= self.max_context_entries
    }

    // ── Serialisation ────────────────────────────────────────────────────────

    /// Render the full log as a JSONL string (one entry per line).
    pub fn serialize_full_log(&self) -> String {
        self.full_log
            .iter()
            .map(|e| e.to_jsonl_line())
            .collect::<Vec<_>>()
            .join("\n")
    }

    /// Render the context window as a JSONL string (one entry per line).
    pub fn serialize_context(&self) -> String {
        self.context
            .iter()
            .map(|e| e.to_jsonl_line())
            .collect::<Vec<_>>()
            .join("\n")
    }

    /// Restore a `DualLog` from previously serialised JSONL strings.
    ///
    /// The watermark is set to `full_log.len()` so that a subsequent
    /// `sync_context()` call will be a no-op (context is already loaded).
    pub fn load(
        full_log_jsonl: &str,
        context_jsonl: &str,
        max_context: usize,
    ) -> Result<Self, String> {
        let full_log = parse_jsonl(full_log_jsonl)?;
        let context = parse_jsonl(context_jsonl)?;
        let watermark = full_log.len();
        Ok(Self {
            full_log,
            context,
            sync_watermark: watermark,
            max_context_entries: max_context,
        })
    }

    /// Write both JSONL files atomically (write to tmp then rename).
    pub fn persist(&self, log_path: &Path, context_path: &Path) -> Result<(), String> {
        write_file(log_path, &self.serialize_full_log())?;
        write_file(context_path, &self.serialize_context())?;
        Ok(())
    }
}

// ---------------------------------------------------------------------------
// Private helpers
// ---------------------------------------------------------------------------

fn parse_jsonl(src: &str) -> Result<Vec<LogEntry>, String> {
    src.lines()
        .filter(|l| !l.trim().is_empty())
        .map(LogEntry::from_jsonl_line)
        .collect()
}

fn write_file(path: &Path, content: &str) -> Result<(), String> {
    use std::io::Write;
    let tmp = path.with_extension("tmp");
    let mut f = std::fs::File::create(&tmp)
        .map_err(|e| format!("create {}: {e}", tmp.display()))?;
    f.write_all(content.as_bytes())
        .map_err(|e| format!("write {}: {e}", tmp.display()))?;
    std::fs::rename(&tmp, path)
        .map_err(|e| format!("rename {} -> {}: {e}", tmp.display(), path.display()))?;
    Ok(())
}

/// Escape a string value for embedding inside a JSON string literal.
fn escape_json(s: &str) -> String {
    s.replace('\\', "\\\\")
        .replace('"', "\\\"")
        .replace('\n', "\\n")
        .replace('\r', "\\r")
        .replace('\t', "\\t")
}

/// Return a JSON-encoded string (with surrounding double quotes).
fn json_string(s: &str) -> String {
    format!("\"{}\"", escape_json(s))
}

/// Pull the value of a JSON string field by name using simple text scanning.
fn extract_str_field(json: &str, field: &str) -> Result<String, String> {
    let needle = format!("\"{}\":", field);
    let start = json
        .find(&needle)
        .ok_or_else(|| format!("field '{field}' not found"))?;
    let after_colon = &json[start + needle.len()..].trim_start();
    if !after_colon.starts_with('"') {
        return Err(format!("field '{field}' is not a string"));
    }
    // Walk char-by-char to handle escape sequences.
    let chars: Vec<char> = after_colon[1..].chars().collect();
    let mut result = String::new();
    let mut i = 0;
    while i < chars.len() {
        match chars[i] {
            '"' => break,
            '\\' if i + 1 < chars.len() => {
                i += 1;
                match chars[i] {
                    'n' => result.push('\n'),
                    'r' => result.push('\r'),
                    't' => result.push('\t'),
                    c => result.push(c),
                }
            }
            c => result.push(c),
        }
        i += 1;
    }
    Ok(result)
}

/// Pull the value of a JSON `u64` field.
fn extract_u64_field(json: &str, field: &str) -> Result<u64, String> {
    let needle = format!("\"{}\":", field);
    let start = json
        .find(&needle)
        .ok_or_else(|| format!("field '{field}' not found"))?;
    let after = json[start + needle.len()..].trim_start();
    let end = after
        .find(|c: char| !c.is_ascii_digit())
        .unwrap_or(after.len());
    after[..end]
        .parse::<u64>()
        .map_err(|e| format!("field '{field}' parse error: {e}"))
}

/// Pull the value of a JSON boolean field.
fn extract_bool_field(json: &str, field: &str) -> Result<bool, String> {
    let needle = format!("\"{}\":", field);
    let start = json
        .find(&needle)
        .ok_or_else(|| format!("field '{field}' not found"))?;
    let after = json[start + needle.len()..].trim_start();
    if after.starts_with("true") {
        Ok(true)
    } else if after.starts_with("false") {
        Ok(false)
    } else {
        Err(format!("field '{field}' is not a boolean"))
    }
}

/// Extract the `metadata` object as a flat `HashMap<String,String>`.
fn extract_metadata(json: &str) -> HashMap<String, String> {
    let needle = "\"metadata\":";
    let mut map = HashMap::new();
    let start = match json.find(needle) {
        Some(s) => s + needle.len(),
        None => return map,
    };
    let after = json[start..].trim_start();
    if !after.starts_with('{') {
        return map;
    }
    let inner_start = start + after.find('{').unwrap_or(0) + 1;
    if let Some(inner_end) = json[inner_start..].find('}') {
        let inner = &json[inner_start..inner_start + inner_end];
        // Very simple key:value extraction for string-only maps.
        let mut rest = inner;
        while let Some(k_start) = rest.find('"') {
            rest = &rest[k_start + 1..];
            let k_end = match rest.find('"') {
                Some(e) => e,
                None => break,
            };
            let key = rest[..k_end].to_string();
            rest = &rest[k_end + 1..];
            // skip ":"
            if let Some(colon) = rest.find(':') {
                rest = rest[colon + 1..].trim_start();
            } else {
                break;
            }
            if rest.starts_with('"') {
                rest = &rest[1..];
                let v_end = match rest.find('"') {
                    Some(e) => e,
                    None => break,
                };
                let val = rest[..v_end].to_string();
                map.insert(key, val);
                rest = &rest[v_end + 1..];
            } else {
                break;
            }
        }
    }
    map
}

// ---------------------------------------------------------------------------
// Unit tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use tempfile::TempDir;

    fn make_entry(id: &str, role: LogRole, content: &str, ts: u64) -> LogEntry {
        LogEntry::new(id, role, content, ts)
    }

    // ── append ───────────────────────────────────────────────────────────────

    #[test]
    fn append_grows_full_log() {
        let mut dl = DualLog::new(10);
        assert_eq!(dl.full_log_count(), 0);
        dl.append(make_entry("1", LogRole::User, "hello", 1));
        dl.append(make_entry("2", LogRole::Assistant, "hi", 2));
        assert_eq!(dl.full_log_count(), 2);
        assert_eq!(dl.context_count(), 0); // not yet synced
    }

    // ── sync_context ─────────────────────────────────────────────────────────

    #[test]
    fn sync_context_moves_entries() {
        let mut dl = DualLog::new(10);
        dl.append(make_entry("1", LogRole::User, "a", 1));
        dl.append(make_entry("2", LogRole::User, "b", 2));
        assert_eq!(dl.unsynced_count(), 2);
        dl.sync_context();
        assert_eq!(dl.context_count(), 2);
        assert_eq!(dl.unsynced_count(), 0);
    }

    #[test]
    fn sync_context_respects_max_entries() {
        let mut dl = DualLog::new(3);
        for i in 0..5u64 {
            dl.append(make_entry(&i.to_string(), LogRole::User, "x", i));
        }
        dl.sync_context();
        // Context capped at 3; full log still has 5.
        assert_eq!(dl.context_count(), 3);
        assert_eq!(dl.full_log_count(), 5);
    }

    #[test]
    fn sync_context_idempotent_when_no_new_entries() {
        let mut dl = DualLog::new(10);
        dl.append(make_entry("1", LogRole::User, "a", 1));
        dl.sync_context();
        dl.sync_context(); // second call should be no-op
        assert_eq!(dl.context_count(), 1);
    }

    // ── compact ──────────────────────────────────────────────────────────────

    #[test]
    fn compact_replaces_old_entries_keeps_recent() {
        let mut dl = DualLog::new(20);
        for i in 0..10u64 {
            dl.append(make_entry(&i.to_string(), LogRole::User, &format!("msg {i}"), i));
        }
        dl.sync_context();
        assert_eq!(dl.context_count(), 10);

        dl.compact("summary of first 7", 3);
        // 1 summary + 3 recent = 4 entries in context
        assert_eq!(dl.context_count(), 4);
        assert!(dl.context_entries()[0].is_compacted);
        assert_eq!(dl.context_entries()[0].content, "summary of first 7");
        // Full log untouched
        assert_eq!(dl.full_log_count(), 10);
    }

    #[test]
    fn compact_noop_when_context_le_keep_recent() {
        let mut dl = DualLog::new(20);
        dl.append(make_entry("1", LogRole::User, "only", 1));
        dl.sync_context();
        dl.compact("should not appear", 5);
        assert_eq!(dl.context_count(), 1);
        assert!(!dl.context_entries()[0].is_compacted);
    }

    // ── grep_log ─────────────────────────────────────────────────────────────

    #[test]
    fn grep_log_finds_historical_entry_not_in_context() {
        let mut dl = DualLog::new(2);
        // Fill full log with 4 entries — only 2 fit in context
        dl.append(make_entry("1", LogRole::User, "ancient history", 1));
        dl.append(make_entry("2", LogRole::User, "more history", 2));
        dl.append(make_entry("3", LogRole::User, "recent a", 3));
        dl.append(make_entry("4", LogRole::User, "recent b", 4));
        dl.sync_context();

        // Context holds entries 3 and 4 only.
        assert_eq!(dl.context_count(), 2);
        let in_context: Vec<_> = dl.context_entries().iter().map(|e| e.id.as_str()).collect();
        assert!(!in_context.contains(&"1"));

        // grep_log can still find "ancient"
        let hits = dl.grep_log("ancient");
        assert_eq!(hits.len(), 1);
        assert_eq!(hits[0].id, "1");
    }

    #[test]
    fn grep_log_returns_empty_when_no_match() {
        let mut dl = DualLog::new(10);
        dl.append(make_entry("1", LogRole::User, "hello world", 1));
        let hits = dl.grep_log("xyzzy");
        assert!(hits.is_empty());
    }

    // ── serialise / deserialise ───────────────────────────────────────────────

    #[test]
    fn jsonl_roundtrip_for_entry() {
        let mut entry = LogEntry::new("abc-123", LogRole::Assistant, "Hello\nWorld", 999);
        entry.metadata.insert("model".into(), "gpt-4o".into());

        let line = entry.to_jsonl_line();
        let parsed = LogEntry::from_jsonl_line(&line).expect("parse failed");

        assert_eq!(parsed.id, entry.id);
        assert_eq!(parsed.role, entry.role);
        assert_eq!(parsed.content, entry.content);
        assert_eq!(parsed.timestamp_ms, entry.timestamp_ms);
        assert_eq!(parsed.is_compacted, entry.is_compacted);
    }

    #[test]
    fn serialize_deserialize_roundtrip() {
        let mut dl = DualLog::new(10);
        dl.append(make_entry("1", LogRole::System, "init", 0));
        dl.append(make_entry("2", LogRole::User, "hello", 1));
        dl.append(make_entry("3", LogRole::Assistant, "hi", 2));
        dl.sync_context();

        let full_jsonl = dl.serialize_full_log();
        let ctx_jsonl = dl.serialize_context();

        let restored = DualLog::load(&full_jsonl, &ctx_jsonl, 10).expect("load failed");
        assert_eq!(restored.full_log_count(), 3);
        assert_eq!(restored.context_count(), 3);
        assert_eq!(restored.unsynced_count(), 0);
        assert_eq!(restored.full_log_entries()[1].content, "hello");
    }

    // ── unsynced_count ───────────────────────────────────────────────────────

    #[test]
    fn unsynced_count_tracks_pending_entries() {
        let mut dl = DualLog::new(10);
        assert_eq!(dl.unsynced_count(), 0);
        dl.append(make_entry("1", LogRole::User, "a", 1));
        assert_eq!(dl.unsynced_count(), 1);
        dl.append(make_entry("2", LogRole::User, "b", 2));
        assert_eq!(dl.unsynced_count(), 2);
        dl.sync_context();
        assert_eq!(dl.unsynced_count(), 0);
    }

    // ── is_context_full ──────────────────────────────────────────────────────

    #[test]
    fn is_context_full_triggers_at_max() {
        let mut dl = DualLog::new(2);
        dl.append(make_entry("1", LogRole::User, "a", 1));
        dl.append(make_entry("2", LogRole::User, "b", 2));
        dl.sync_context();
        assert!(dl.is_context_full());
    }

    // ── persist + load ───────────────────────────────────────────────────────

    #[test]
    fn persist_and_load_round_trip() {
        let tmp = TempDir::new().unwrap();
        let log_path = tmp.path().join("log.jsonl");
        let ctx_path = tmp.path().join("context.jsonl");

        let mut dl = DualLog::new(5);
        dl.append(make_entry("u1", LogRole::User, "question", 100));
        dl.append(make_entry("a1", LogRole::Assistant, "answer", 200));
        dl.sync_context();
        dl.persist(&log_path, &ctx_path).expect("persist failed");

        assert!(log_path.exists());
        assert!(ctx_path.exists());

        let full_src = fs::read_to_string(&log_path).unwrap();
        let ctx_src = fs::read_to_string(&ctx_path).unwrap();
        let restored = DualLog::load(&full_src, &ctx_src, 5).expect("load failed");

        assert_eq!(restored.full_log_count(), 2);
        assert_eq!(restored.context_count(), 2);
        assert_eq!(restored.full_log_entries()[0].id, "u1");
        assert_eq!(restored.full_log_entries()[1].content, "answer");
    }

    // ── compaction_summary constructor ───────────────────────────────────────

    #[test]
    fn compaction_summary_entry_fields() {
        let entry = LogEntry::compaction_summary("compact text", 42);
        assert!(entry.is_compacted);
        assert_eq!(entry.role, LogRole::System);
        assert_eq!(entry.content, "compact text");
        assert_eq!(entry.timestamp_ms, 42);
        assert!(entry.id.starts_with("compact-"));
    }
}
