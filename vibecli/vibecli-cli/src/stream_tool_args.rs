//! Streaming partial tool call arguments via toolcall_delta events.
//! Pi-mono gap bridge: Phase B3.
//!
//! During LLM streaming, tool call arguments arrive as incremental JSON
//! fragments.  This module accumulates those fragments, attempts partial JSON
//! parsing after every push, and surfaces UI-friendly hints so that the TUI
//! can display "write_file: editing src/main.rs…" before the full payload
//! has been received.

use std::collections::HashMap;

// ---------------------------------------------------------------------------
// ToolCallDelta
// ---------------------------------------------------------------------------

/// A single streaming delta for one tool call argument stream.
#[derive(Debug, Clone)]
pub struct ToolCallDelta {
    /// Opaque identifier assigned by the LLM runtime (e.g. `"call_abc123"`).
    pub call_id: String,
    /// Name of the tool being called (e.g. `"write_file"`).
    pub tool_name: String,
    /// Raw JSON fragment received in this delta.
    pub args_fragment: String,
    /// `true` when this is the final delta for the call.
    pub is_complete: bool,
    /// Monotonically increasing counter per `call_id`, starting at 0.
    pub sequence: u32,
}

impl ToolCallDelta {
    /// Create an incomplete (intermediate) delta.
    pub fn new(
        call_id: impl Into<String>,
        tool_name: impl Into<String>,
        fragment: impl Into<String>,
        seq: u32,
    ) -> Self {
        Self {
            call_id: call_id.into(),
            tool_name: tool_name.into(),
            args_fragment: fragment.into(),
            is_complete: false,
            sequence: seq,
        }
    }

    /// Create the terminal delta that marks the call as complete.
    pub fn complete(
        call_id: impl Into<String>,
        tool_name: impl Into<String>,
        full_args: impl Into<String>,
        seq: u32,
    ) -> Self {
        Self {
            call_id: call_id.into(),
            tool_name: tool_name.into(),
            args_fragment: full_args.into(),
            is_complete: true,
            sequence: seq,
        }
    }
}

// ---------------------------------------------------------------------------
// PartialHint
// ---------------------------------------------------------------------------

/// A best-guess hint about the semantic content of the partial JSON buffer.
#[derive(Debug, Clone, PartialEq)]
pub enum PartialHint {
    /// A `"path"` or `"file_path"` key was found with a (possibly partial) string value.
    FilePath(String),
    /// A `"command"` key was found; contains the partial value captured so far.
    CommandFragment(String),
    /// A `"content"` key was detected; the value may be large — report its accumulated length.
    ContentLength(usize),
    /// The first key name encountered, when none of the above apply.
    UnknownKey(String),
}

// ---------------------------------------------------------------------------
// PartialParseResult
// ---------------------------------------------------------------------------

/// The result of one attempt to parse the accumulated argument buffer.
#[derive(Debug, Clone)]
pub struct PartialParseResult {
    /// Top-level keys whose values are complete JSON strings (i.e. the closing
    /// `"` has arrived and the value is not still being streamed).
    pub extractable_keys: Vec<String>,
    /// `true` when the full buffer is valid, complete JSON.
    pub is_valid_json: bool,
    /// Best guess at the semantic meaning of what has arrived so far.
    pub hint: Option<PartialHint>,
}

impl PartialParseResult {
    /// Returns `true` when the hint is a `FilePath` variant.
    pub fn has_file_path(&self) -> bool {
        matches!(&self.hint, Some(PartialHint::FilePath(_)))
    }

    /// Returns the file path string if the hint is `FilePath`, otherwise `None`.
    pub fn file_path(&self) -> Option<&str> {
        if let Some(PartialHint::FilePath(p)) = &self.hint {
            Some(p.as_str())
        } else {
            None
        }
    }
}

// ---------------------------------------------------------------------------
// Partial JSON helpers
// ---------------------------------------------------------------------------

/// Attempt to extract top-level string values from a potentially incomplete
/// JSON object.  We only return keys whose string values are fully closed
/// (i.e. a complete `"key": "value"` pair has been received).
///
/// Strategy: parse as valid JSON first; if that succeeds, return all string
/// keys.  Otherwise, walk the raw text looking for `"key": "value"` patterns
/// where the closing quote of the value has arrived.
fn extract_complete_string_keys(buffer: &str) -> Vec<String> {
    // Fast path — valid JSON.
    if let Ok(v) = serde_json::from_str::<serde_json::Value>(buffer) {
        if let Some(obj) = v.as_object() {
            return obj
                .iter()
                .filter(|(_, v)| v.is_string())
                .map(|(k, _)| k.clone())
                .collect();
        }
    }

    // Slow path — scan for completed `"key": "value"` pairs.
    let mut keys = Vec::new();
    let bytes = buffer.as_bytes();
    let len = bytes.len();
    let mut i = 0;

    while i < len {
        // Find the opening `"` of a key.
        if bytes[i] != b'"' {
            i += 1;
            continue;
        }
        i += 1; // skip opening quote

        // Read the key name.
        let key_start = i;
        while i < len && bytes[i] != b'"' {
            if bytes[i] == b'\\' {
                i += 1; // skip escaped char
            }
            i += 1;
        }
        if i >= len {
            break;
        }
        let key = &buffer[key_start..i];
        i += 1; // skip closing key quote

        // Skip whitespace and colon.
        while i < len && (bytes[i] == b' ' || bytes[i] == b'\t' || bytes[i] == b'\n' || bytes[i] == b'\r') {
            i += 1;
        }
        if i >= len || bytes[i] != b':' {
            continue;
        }
        i += 1; // skip ':'

        // Skip whitespace.
        while i < len && (bytes[i] == b' ' || bytes[i] == b'\t' || bytes[i] == b'\n' || bytes[i] == b'\r') {
            i += 1;
        }

        // Check if the value is a string.
        if i >= len || bytes[i] != b'"' {
            // Value is not a string — skip to the next top-level key.
            continue;
        }
        i += 1; // skip opening value quote

        // Scan to the closing quote of the value.
        let val_start = i;
        let mut closed = false;
        while i < len {
            if bytes[i] == b'\\' {
                i += 2;
                continue;
            }
            if bytes[i] == b'"' {
                closed = true;
                i += 1;
                break;
            }
            i += 1;
        }
        if closed {
            let _ = &buffer[val_start..i - 1]; // validate slice
            keys.push(key.to_string());
        }
    }

    keys
}

/// Extract the string value for a specific key from a partial JSON buffer.
/// Returns `None` if the key is not present or its value has not closed yet.
fn extract_string_value<'a>(buffer: &'a str, key: &str) -> Option<&'a str> {
    // Try valid JSON first.
    if let Ok(v) = serde_json::from_str::<serde_json::Value>(buffer) {
        if let Some(s) = v.get(key).and_then(|v| v.as_str()) {
            // Return owned — we need to return a &'a str pointing into buffer.
            // Since the JSON string may differ from the raw slice, fall through
            // to the raw scan below when the value contains escapes.
            let _ = s; // we'll use the raw scan instead for lifetime reasons
        }
    }

    // Raw scan for `"<key>": "<value>"`.
    let needle = format!("\"{}\"", key);
    let mut pos = 0;
    while let Some(rel) = buffer[pos..].find(needle.as_str()) {
        let after_key = pos + rel + needle.len();
        let rest = buffer[after_key..].trim_start();
        if !rest.starts_with(':') {
            pos += rel + 1;
            continue;
        }
        let after_colon = rest[1..].trim_start();
        if !after_colon.starts_with('"') {
            pos += rel + 1;
            continue;
        }
        // Step past the opening quote.
        let val_bytes = &after_colon.as_bytes()[1..];
        let mut end = 0;
        let mut closed = false;
        while end < val_bytes.len() {
            if val_bytes[end] == b'\\' {
                end += 2;
                continue;
            }
            if val_bytes[end] == b'"' {
                closed = true;
                break;
            }
            end += 1;
        }
        if closed {
            return Some(&after_colon[1..end + 1]); // slice inside original buffer
        }
        return None; // value not yet closed
    }
    None
}

/// Detect whether a `"content"` key has started streaming in the buffer and
/// return the number of characters received so far for its value (even if
/// unclosed).
fn detect_content_length(buffer: &str) -> Option<usize> {
    let needle = "\"content\"";
    let pos = buffer.find(needle)?;
    let rest = buffer[pos + needle.len()..].trim_start();
    if !rest.starts_with(':') {
        return None;
    }
    let after_colon = rest[1..].trim_start();
    if !after_colon.starts_with('"') {
        return None;
    }
    let val = &after_colon[1..];
    // Count until closing quote (or end of buffer if still streaming).
    let mut len = 0;
    let mut chars = val.char_indices();
    let mut found_close = false;
    while let Some((_, ch)) = chars.next() {
        if ch == '\\' {
            chars.next(); // skip escaped char
            len += 1;
        } else if ch == '"' {
            found_close = true;
            break;
        } else {
            len += 1;
        }
    }
    if found_close || !val.is_empty() {
        Some(len)
    } else {
        None
    }
}

/// Build a `PartialParseResult` from a raw buffer.
fn parse_partial(buffer: &str) -> PartialParseResult {
    let is_valid_json = serde_json::from_str::<serde_json::Value>(buffer).is_ok();
    let extractable_keys = extract_complete_string_keys(buffer);

    // Determine the best hint.
    let hint = if let Some(path) = extract_string_value(buffer, "path")
        .or_else(|| extract_string_value(buffer, "file_path"))
    {
        Some(PartialHint::FilePath(path.to_string()))
    } else if let Some(cmd) = extract_string_value(buffer, "command") {
        Some(PartialHint::CommandFragment(cmd.to_string()))
    } else if let Some(content_len) = detect_content_length(buffer) {
        Some(PartialHint::ContentLength(content_len))
    } else if !extractable_keys.is_empty() {
        Some(PartialHint::UnknownKey(extractable_keys[0].clone()))
    } else {
        // Scan for the first key name even if its value is incomplete.
        extract_first_key_name(buffer).map(PartialHint::UnknownKey)
    };

    PartialParseResult {
        extractable_keys,
        is_valid_json,
        hint,
    }
}

/// Return the name of the first key encountered in a partial JSON object,
/// regardless of whether its value has arrived.
fn extract_first_key_name(buffer: &str) -> Option<String> {
    let trimmed = buffer.trim_start();
    if !trimmed.starts_with('{') {
        return None;
    }
    let inner = &trimmed[1..].trim_start();
    if !inner.starts_with('"') {
        return None;
    }
    let key_bytes = &inner.as_bytes()[1..];
    let mut end = 0;
    while end < key_bytes.len() {
        if key_bytes[end] == b'\\' {
            end += 2;
            continue;
        }
        if key_bytes[end] == b'"' {
            return Some(inner[1..end + 1].to_string());
        }
        end += 1;
    }
    None
}

// ---------------------------------------------------------------------------
// ToolArgAccumulator
// ---------------------------------------------------------------------------

/// Accumulates JSON fragments for a single streaming tool call and exposes
/// partial parsing after every push.
#[derive(Debug)]
pub struct ToolArgAccumulator {
    call_id: String,
    tool_name: String,
    buffer: String,
    sequence: u32,
}

impl ToolArgAccumulator {
    /// Create a new accumulator for the given call.
    pub fn new(call_id: impl Into<String>, tool_name: impl Into<String>) -> Self {
        Self {
            call_id: call_id.into(),
            tool_name: tool_name.into(),
            buffer: String::new(),
            sequence: 0,
        }
    }

    /// Append a fragment to the internal buffer, increment the sequence
    /// counter, and return a `PartialParseResult`.
    pub fn push(&mut self, fragment: &str) -> PartialParseResult {
        self.buffer.push_str(fragment);
        self.sequence += 1;
        parse_partial(&self.buffer)
    }

    /// Attempt to parse the complete accumulated buffer as JSON.
    ///
    /// Returns `Err` with a human-readable message when the buffer is not
    /// valid JSON (e.g. the LLM was interrupted mid-stream).
    pub fn finalize(&self) -> Result<serde_json::Value, String> {
        serde_json::from_str(&self.buffer)
            .map_err(|e| format!("finalize failed for call '{}': {}", self.call_id, e))
    }

    /// The raw accumulated buffer.
    pub fn buffer(&self) -> &str {
        &self.buffer
    }

    /// The call identifier.
    pub fn call_id(&self) -> &str {
        &self.call_id
    }

    /// The tool name.
    pub fn tool_name(&self) -> &str {
        &self.tool_name
    }

    /// The current sequence number (number of pushes made so far).
    pub fn sequence(&self) -> u32 {
        self.sequence
    }
}

// ---------------------------------------------------------------------------
// StreamingToolCallManager
// ---------------------------------------------------------------------------

/// Manages `ToolArgAccumulator` instances for multiple concurrent streaming
/// tool calls that may arrive interleaved during a single generation.
#[derive(Debug)]
pub struct StreamingToolCallManager {
    accumulators: HashMap<String, ToolArgAccumulator>,
}

impl Default for StreamingToolCallManager {
    fn default() -> Self {
        Self::new()
    }
}

impl StreamingToolCallManager {
    /// Create a new, empty manager.
    pub fn new() -> Self {
        Self {
            accumulators: HashMap::new(),
        }
    }

    /// Feed a `ToolCallDelta` into the appropriate accumulator (creating one
    /// if this is the first delta for `call_id`).  Returns the
    /// `PartialParseResult` after appending the fragment.
    pub fn on_delta(&mut self, delta: ToolCallDelta) -> PartialParseResult {
        let acc = self
            .accumulators
            .entry(delta.call_id.clone())
            .or_insert_with(|| {
                ToolArgAccumulator::new(delta.call_id.clone(), delta.tool_name.clone())
            });
        acc.push(&delta.args_fragment)
    }

    /// Finalize the call with the given `call_id` and return its parsed JSON
    /// value.  The accumulator is retained (not removed) so that callers can
    /// still inspect the buffer.
    pub fn on_complete(&mut self, call_id: &str) -> Result<serde_json::Value, String> {
        match self.accumulators.get(call_id) {
            Some(acc) => acc.finalize(),
            None => Err(format!("no accumulator found for call_id '{}'", call_id)),
        }
    }

    /// Return the list of active (registered) calls as `(call_id, tool_name)`
    /// pairs.  Order is unspecified.
    pub fn active_calls(&self) -> Vec<(&str, &str)> {
        self.accumulators
            .values()
            .map(|a| (a.call_id(), a.tool_name()))
            .collect()
    }

    /// Finalize every registered call and drain the manager.  Returns a
    /// `Vec<(call_id, Result<Value, String>)>`.
    pub fn finalize_all(&mut self) -> Vec<(String, Result<serde_json::Value, String>)> {
        self.accumulators
            .drain()
            .map(|(id, acc)| (id, acc.finalize()))
            .collect()
    }

    /// Remove all accumulators.
    pub fn clear(&mut self) {
        self.accumulators.clear();
    }
}

// ---------------------------------------------------------------------------
// TUI renderer
// ---------------------------------------------------------------------------

/// Render a `PartialParseResult` as a one-line status string suitable for
/// display in the TUI status bar.
///
/// # Examples
/// ```text
/// "write_file: editing src/main.rs…"
/// "bash: running echo hello…"
/// "read_file: receiving content (42 chars)…"
/// "some_tool: reading args…"
/// ```
pub fn render_partial_hint(result: &PartialParseResult, tool_name: &str) -> String {
    match &result.hint {
        Some(PartialHint::FilePath(path)) => {
            format!("{}: editing {}...", tool_name, path)
        }
        Some(PartialHint::CommandFragment(cmd)) => {
            if cmd.is_empty() {
                format!("{}: running command...", tool_name)
            } else {
                format!("{}: running {}...", tool_name, cmd)
            }
        }
        Some(PartialHint::ContentLength(n)) => {
            format!("{}: receiving content ({} chars)...", tool_name, n)
        }
        Some(PartialHint::UnknownKey(key)) => {
            format!("{}: reading {} arg...", tool_name, key)
        }
        None => {
            if result.is_valid_json {
                format!("{}: args ready", tool_name)
            } else {
                format!("{}: reading args...", tool_name)
            }
        }
    }
}

// ---------------------------------------------------------------------------
// Unit tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    // -- ToolArgAccumulator --------------------------------------------------

    #[test]
    fn accumulator_push_increments_sequence() {
        let mut acc = ToolArgAccumulator::new("c1", "write_file");
        assert_eq!(acc.sequence(), 0);
        acc.push("{\"path\":");
        assert_eq!(acc.sequence(), 1);
        acc.push("\"src/main.rs\"");
        assert_eq!(acc.sequence(), 2);
        acc.push("}");
        assert_eq!(acc.sequence(), 3);
    }

    #[test]
    fn accumulator_buffer_grows() {
        let mut acc = ToolArgAccumulator::new("c2", "bash");
        acc.push("{\"command\": \"ec");
        acc.push("ho hi\"}");
        assert_eq!(acc.buffer(), "{\"command\": \"echo hi\"}");
    }

    // -- Partial JSON key extraction -----------------------------------------

    #[test]
    fn extract_complete_string_keys_valid_json() {
        let buf = r#"{"path": "src/lib.rs", "content": "hello"}"#;
        let keys = extract_complete_string_keys(buf);
        assert!(keys.contains(&"path".to_string()));
        assert!(keys.contains(&"content".to_string()));
    }

    #[test]
    fn extract_complete_string_keys_partial_json() {
        // "content" value is still open — only "path" should be extractable.
        let buf = r#"{"path": "src/lib.rs", "content": "hel"#;
        let keys = extract_complete_string_keys(buf);
        assert!(keys.contains(&"path".to_string()), "keys = {:?}", keys);
        assert!(!keys.contains(&"content".to_string()), "content should not be extractable yet");
    }

    // -- Finalize valid JSON -------------------------------------------------

    #[test]
    fn finalize_returns_parsed_value() {
        let mut acc = ToolArgAccumulator::new("c3", "write_file");
        acc.push(r#"{"path": "foo.rs", "content": "fn main(){}"}"#);
        let val = acc.finalize().expect("should parse");
        assert_eq!(val["path"], "foo.rs");
        assert_eq!(val["content"], "fn main(){}");
    }

    #[test]
    fn finalize_errors_on_incomplete_json() {
        let mut acc = ToolArgAccumulator::new("c4", "write_file");
        acc.push(r#"{"path": "foo.rs", "content": "#);
        assert!(acc.finalize().is_err());
    }

    // -- FilePath hint detection ---------------------------------------------

    #[test]
    fn file_path_hint_detected_path_key() {
        let mut acc = ToolArgAccumulator::new("c5", "write_file");
        let result = acc.push(r#"{"path": "src/main.rs", "content": "..."}"#);
        assert!(result.has_file_path(), "expected FilePath hint");
        assert_eq!(result.file_path(), Some("src/main.rs"));
    }

    #[test]
    fn file_path_hint_detected_file_path_key() {
        let mut acc = ToolArgAccumulator::new("c6", "edit_file");
        let result = acc.push(r#"{"file_path": "src/lib.rs"}"#);
        assert!(result.has_file_path());
        assert_eq!(result.file_path(), Some("src/lib.rs"));
    }

    #[test]
    fn command_hint_detected() {
        let mut acc = ToolArgAccumulator::new("c7", "bash");
        let result = acc.push(r#"{"command": "cargo build"}"#);
        assert!(
            matches!(&result.hint, Some(PartialHint::CommandFragment(cmd)) if cmd == "cargo build"),
            "got {:?}",
            result.hint
        );
    }

    #[test]
    fn content_length_hint_detected() {
        let mut acc = ToolArgAccumulator::new("c8", "write_file");
        // content value is still open
        let result = acc.push(r#"{"content": "hello wor"#);
        assert!(
            matches!(&result.hint, Some(PartialHint::ContentLength(n)) if *n == 9),
            "got {:?}",
            result.hint
        );
    }

    // -- StreamingToolCallManager -------------------------------------------

    #[test]
    fn manager_multi_call_tracking() {
        let mut mgr = StreamingToolCallManager::new();

        mgr.on_delta(ToolCallDelta::new("call_1", "write_file", r#"{"path": "a.rs"}"#, 0));
        mgr.on_delta(ToolCallDelta::new("call_2", "bash", r#"{"command": "ls"}"#, 0));

        let calls = mgr.active_calls();
        assert_eq!(calls.len(), 2);
        let ids: Vec<&str> = calls.iter().map(|(id, _)| *id).collect();
        assert!(ids.contains(&"call_1"));
        assert!(ids.contains(&"call_2"));
    }

    #[test]
    fn manager_on_complete_returns_value() {
        let mut mgr = StreamingToolCallManager::new();
        mgr.on_delta(ToolCallDelta::new("call_3", "bash", r#"{"command":"ls -la"}"#, 0));
        let val = mgr.on_complete("call_3").expect("should parse");
        assert_eq!(val["command"], "ls -la");
    }

    #[test]
    fn manager_finalize_all_drains() {
        let mut mgr = StreamingToolCallManager::new();
        mgr.on_delta(ToolCallDelta::new("call_a", "bash", r#"{"command":"pwd"}"#, 0));
        mgr.on_delta(ToolCallDelta::new("call_b", "read_file", r#"{"path":"x.rs"}"#, 0));
        let results = mgr.finalize_all();
        assert_eq!(results.len(), 2);
        assert_eq!(mgr.active_calls().len(), 0, "should be empty after finalize_all");
    }

    #[test]
    fn manager_clear_removes_all() {
        let mut mgr = StreamingToolCallManager::new();
        mgr.on_delta(ToolCallDelta::new("call_x", "tool", "{}", 0));
        mgr.clear();
        assert!(mgr.active_calls().is_empty());
    }

    // -- render_partial_hint ------------------------------------------------

    #[test]
    fn render_file_path_hint() {
        let result = PartialParseResult {
            extractable_keys: vec!["path".into()],
            is_valid_json: false,
            hint: Some(PartialHint::FilePath("src/main.rs".into())),
        };
        let s = render_partial_hint(&result, "write_file");
        assert!(s.contains("write_file"), "{}", s);
        assert!(s.contains("src/main.rs"), "{}", s);
    }

    #[test]
    fn render_command_hint() {
        let result = PartialParseResult {
            extractable_keys: vec![],
            is_valid_json: false,
            hint: Some(PartialHint::CommandFragment("cargo test".into())),
        };
        let s = render_partial_hint(&result, "bash");
        assert!(s.contains("bash"), "{}", s);
        assert!(s.contains("cargo test"), "{}", s);
    }

    #[test]
    fn render_no_hint_incomplete() {
        let result = PartialParseResult {
            extractable_keys: vec![],
            is_valid_json: false,
            hint: None,
        };
        let s = render_partial_hint(&result, "unknown_tool");
        assert!(s.contains("unknown_tool"), "{}", s);
        assert!(s.contains("reading args"), "{}", s);
    }

    #[test]
    fn render_no_hint_complete() {
        let result = PartialParseResult {
            extractable_keys: vec![],
            is_valid_json: true,
            hint: None,
        };
        let s = render_partial_hint(&result, "my_tool");
        assert!(s.contains("my_tool"), "{}", s);
        assert!(s.contains("ready"), "{}", s);
    }
}
