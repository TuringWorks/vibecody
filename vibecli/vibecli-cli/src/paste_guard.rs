//! Bracketed paste safety guard — collapse large pastes to markers.
//! Pi-mono gap bridge: Phase C5.
//!
//! Terminal bracketed paste mode wraps pasted content in ESC[200~ ... ESC[201~
//! escape sequences. When pasted content exceeds a line threshold, PasteGuard
//! replaces it with a compact `[paste #N +X lines]` marker and stores the full
//! content in a ring buffer. This prevents accidental large context injection
//! and "prompt bombing" attacks.

use std::collections::VecDeque;

/// Terminal escape sequences that bracket pasted text.
pub const BRACKETED_PASTE_START: &str = "\x1b[200~";
pub const BRACKETED_PASTE_END: &str = "\x1b[201~";

/// Enable bracketed paste mode on the terminal.
pub const ENABLE_BRACKETED_PASTE: &str = "\x1b[?2004h";
/// Disable bracketed paste mode on the terminal.
pub const DISABLE_BRACKETED_PASTE: &str = "\x1b[?2004l";

// ── PasteEvent ────────────────────────────────────────────────────────────────

/// A captured paste event.
#[derive(Debug, Clone)]
pub struct PasteEvent {
    /// Monotonically increasing paste number (1-based).
    pub id: u32,
    /// Full pasted content (without bracketed-paste escape sequences).
    pub content: String,
    /// Number of newline-delimited lines in the content.
    pub line_count: usize,
    /// Total character count of the content.
    pub char_count: usize,
}

impl PasteEvent {
    /// Create a new `PasteEvent` from raw content. Counts lines and chars automatically.
    pub fn new(id: u32, content: impl Into<String>) -> Self {
        let content = content.into();
        let line_count = content.lines().count().max(1);
        let char_count = content.chars().count();
        Self {
            id,
            content,
            line_count,
            char_count,
        }
    }

    /// Return `true` when the event's line count exceeds `threshold`.
    pub fn is_large(&self, threshold: usize) -> bool {
        self.line_count > threshold
    }

    /// Return the first `n` lines of the paste content as string slices.
    pub fn preview_lines(&self, n: usize) -> Vec<&str> {
        self.content.lines().take(n).collect()
    }

    /// Format the collapse marker, e.g. `[paste #1 +47 lines]`.
    pub fn marker(&self) -> String {
        format!("[paste #{} +{} lines]", self.id, self.line_count)
    }
}

// ── PasteGuardConfig ──────────────────────────────────────────────────────────

/// Configuration for paste guard behaviour.
#[derive(Debug, Clone)]
pub struct PasteGuardConfig {
    /// Lines at which a paste is considered "large" and collapsed (default: 10).
    pub line_threshold: usize,
    /// Maximum number of paste events to keep in the ring buffer (default: 20).
    pub max_stored_pastes: usize,
    /// Number of preview lines to expose before the collapse marker (default: 3).
    pub show_preview_lines: usize,
    /// When `true`, pastes below the threshold are passed through verbatim (default: true).
    pub auto_expand_under_threshold: bool,
}

impl Default for PasteGuardConfig {
    fn default() -> Self {
        Self {
            line_threshold: 10,
            max_stored_pastes: 20,
            show_preview_lines: 3,
            auto_expand_under_threshold: true,
        }
    }
}

// ── PasteStore ────────────────────────────────────────────────────────────────

/// Ring-buffer store for paste events.
#[derive(Debug)]
pub struct PasteStore {
    pastes: VecDeque<PasteEvent>,
    max_size: usize,
    next_id: u32,
}

impl PasteStore {
    /// Create a new `PasteStore` with the given maximum capacity.
    pub fn new(max_size: usize) -> Self {
        Self {
            pastes: VecDeque::with_capacity(max_size),
            max_size,
            next_id: 1,
        }
    }

    /// Store a paste, evicting the oldest entry when the buffer is full.
    /// Returns the created `PasteEvent`.
    pub fn store(&mut self, content: impl Into<String>) -> PasteEvent {
        let id = self.next_id;
        self.next_id += 1;
        let event = PasteEvent::new(id, content);
        if self.pastes.len() == self.max_size {
            self.pastes.pop_front();
        }
        self.pastes.push_back(event.clone());
        event
    }

    /// Look up a paste by its numeric id.
    pub fn get(&self, id: u32) -> Option<&PasteEvent> {
        self.pastes.iter().find(|e| e.id == id)
    }

    /// Return the most recently stored paste event.
    pub fn latest(&self) -> Option<&PasteEvent> {
        self.pastes.back()
    }

    /// Number of events currently in the store.
    pub fn count(&self) -> usize {
        self.pastes.len()
    }

    /// Discard all stored paste events and reset the id counter.
    pub fn clear(&mut self) {
        self.pastes.clear();
        self.next_id = 1;
    }

    /// Return all stored paste ids in insertion order.
    pub fn all_ids(&self) -> Vec<u32> {
        self.pastes.iter().map(|e| e.id).collect()
    }
}

// ── PasteGuard ────────────────────────────────────────────────────────────────

/// The paste guard processor — receives raw terminal input and classifies paste events.
#[derive(Debug)]
pub struct PasteGuard {
    config: PasteGuardConfig,
    store: PasteStore,
    in_paste: bool,
    paste_buffer: String,
}

impl PasteGuard {
    /// Create a new `PasteGuard` with the supplied configuration.
    pub fn new(config: PasteGuardConfig) -> Self {
        let max = config.max_stored_pastes;
        Self {
            config,
            store: PasteStore::new(max),
            in_paste: false,
            paste_buffer: String::new(),
        }
    }

    /// Create a `PasteGuard` with default configuration.
    pub fn with_defaults() -> Self {
        Self::new(PasteGuardConfig::default())
    }

    /// Process a chunk of terminal input.
    ///
    /// Returns a [`ProcessResult`] whose `processed_input` has any large pastes
    /// replaced by their collapse markers, and whose `paste_events` lists every
    /// paste captured during this call.
    pub fn process(&mut self, input: &str) -> ProcessResult {
        let mut output = String::with_capacity(input.len());
        let mut paste_events: Vec<PasteEvent> = Vec::new();
        let mut was_paste = false;
        let mut was_collapsed = false;

        let mut remaining = input;

        while !remaining.is_empty() {
            if self.in_paste {
                // Look for the end sequence.
                if let Some(end_pos) = remaining.find(BRACKETED_PASTE_END) {
                    self.paste_buffer.push_str(&remaining[..end_pos]);
                    remaining = &remaining[end_pos + BRACKETED_PASTE_END.len()..];
                    self.in_paste = false;

                    let content = std::mem::take(&mut self.paste_buffer);
                    let event = self.store.store(&content);
                    was_paste = true;

                    if event.is_large(self.config.line_threshold) {
                        // Collapsed: emit optional preview then marker.
                        was_collapsed = true;
                        if self.config.show_preview_lines > 0 {
                            let preview = event
                                .preview_lines(self.config.show_preview_lines)
                                .join("\n");
                            if !preview.is_empty() {
                                output.push_str(&preview);
                                output.push('\n');
                            }
                        }
                        output.push_str(&event.marker());
                    } else {
                        // Small paste: pass through verbatim.
                        output.push_str(&event.content);
                    }
                    paste_events.push(event);
                } else {
                    // End sequence not yet seen; buffer everything.
                    self.paste_buffer.push_str(remaining);
                    remaining = "";
                }
            } else {
                // Look for the start sequence.
                if let Some(start_pos) = remaining.find(BRACKETED_PASTE_START) {
                    // Emit everything before the sequence as-is.
                    output.push_str(&remaining[..start_pos]);
                    remaining = &remaining[start_pos + BRACKETED_PASTE_START.len()..];
                    self.in_paste = true;
                    self.paste_buffer.clear();
                } else {
                    output.push_str(remaining);
                    remaining = "";
                }
            }
        }

        ProcessResult {
            processed_input: output,
            paste_events,
            was_paste,
            was_collapsed,
        }
    }

    /// Expand a paste marker back to its full content.
    ///
    /// Accepts a string like `"[paste #3 +22 lines]"` and returns the
    /// corresponding stored content, if still available in the ring buffer.
    pub fn expand_marker(&self, marker: &str) -> Option<&str> {
        let id = Self::marker_to_id(marker)?;
        self.store.get(id).map(|e| e.content.as_str())
    }

    /// Extract the paste id from a marker string such as `"[paste #3 +22 lines]"`.
    ///
    /// Returns `None` when the string does not match the expected format.
    pub fn marker_to_id(marker: &str) -> Option<u32> {
        // Expected form: [paste #<id> +<n> lines]
        let marker = marker.trim();
        if !marker.starts_with("[paste #") || !marker.ends_with(']') {
            return None;
        }
        let after_hash = marker.strip_prefix("[paste #")?;
        let space_pos = after_hash.find(' ')?;
        after_hash[..space_pos].parse::<u32>().ok()
    }

    /// Read-only access to the current configuration.
    pub fn config(&self) -> &PasteGuardConfig {
        &self.config
    }

    /// Read-only access to the paste store.
    pub fn store(&self) -> &PasteStore {
        &self.store
    }
}

// ── Free functions ────────────────────────────────────────────────────────────

/// Strip bracketed paste escape sequences from `s` (for non-paste-mode terminals).
pub fn strip_bracketed_sequences(s: &str) -> String {
    s.replace(BRACKETED_PASTE_START, "")
        .replace(BRACKETED_PASTE_END, "")
}

/// Check whether `s` contains the bracketed paste start sequence.
pub fn has_paste_start(s: &str) -> bool {
    s.contains(BRACKETED_PASTE_START)
}

/// Extract the content between bracketed paste start/end sequences.
///
/// Returns `None` if neither sequence is present or the content between them
/// is empty after stripping the sequences.
pub fn extract_paste_content(s: &str) -> Option<String> {
    let start = s.find(BRACKETED_PASTE_START)?;
    let after_start = start + BRACKETED_PASTE_START.len();
    let end = s[after_start..].find(BRACKETED_PASTE_END)?;
    let content = s[after_start..after_start + end].to_string();
    if content.is_empty() {
        None
    } else {
        Some(content)
    }
}

// ── ProcessResult ─────────────────────────────────────────────────────────────

/// Result of processing a chunk of input through [`PasteGuard`].
#[derive(Debug, Clone)]
pub struct ProcessResult {
    /// Input with large pastes replaced by collapse markers.
    pub processed_input: String,
    /// Every paste event captured during this `process` call.
    pub paste_events: Vec<PasteEvent>,
    /// `true` when at least one paste sequence was detected.
    pub was_paste: bool,
    /// `true` when at least one paste was collapsed to a marker.
    pub was_collapsed: bool,
}

// ── Unit tests ────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    fn make_paste(lines: usize) -> String {
        (0..lines)
            .map(|i| format!("line {}", i + 1))
            .collect::<Vec<_>>()
            .join("\n")
    }

    fn bracketed(content: &str) -> String {
        format!(
            "{}{}{}",
            BRACKETED_PASTE_START, content, BRACKETED_PASTE_END
        )
    }

    // ── PasteEvent ────────────────────────────────────────────────────────────

    #[test]
    fn paste_event_marker_format() {
        let e = PasteEvent::new(1, make_paste(47));
        assert_eq!(e.marker(), "[paste #1 +47 lines]");
    }

    #[test]
    fn paste_event_marker_uses_correct_id() {
        let e = PasteEvent::new(7, make_paste(3));
        assert_eq!(e.marker(), "[paste #7 +3 lines]");
    }

    #[test]
    fn paste_event_is_large_threshold() {
        let e = PasteEvent::new(1, make_paste(10));
        assert!(!e.is_large(10), "exactly at threshold is not large");
        assert!(e.is_large(9), "above threshold is large");
    }

    #[test]
    fn paste_event_preview_lines() {
        let e = PasteEvent::new(1, make_paste(20));
        let preview = e.preview_lines(3);
        assert_eq!(preview.len(), 3);
        assert_eq!(preview[0], "line 1");
        assert_eq!(preview[2], "line 3");
    }

    #[test]
    fn paste_event_char_count() {
        let content = "abc";
        let e = PasteEvent::new(1, content);
        assert_eq!(e.char_count, 3);
    }

    // ── PasteStore ────────────────────────────────────────────────────────────

    #[test]
    fn paste_store_stores_and_retrieves() {
        let mut store = PasteStore::new(5);
        let event = store.store("hello world");
        assert_eq!(event.id, 1);
        assert!(store.get(1).is_some());
        assert_eq!(store.count(), 1);
    }

    #[test]
    fn paste_store_ring_buffer_eviction() {
        let mut store = PasteStore::new(3);
        store.store("a");
        store.store("b");
        store.store("c");
        // Buffer is full; next store evicts id=1.
        store.store("d");
        assert_eq!(store.count(), 3);
        assert!(store.get(1).is_none(), "oldest entry should be evicted");
        assert!(store.get(2).is_some());
        assert!(store.get(4).is_some());
    }

    #[test]
    fn paste_store_latest() {
        let mut store = PasteStore::new(5);
        store.store("first");
        store.store("second");
        assert_eq!(store.latest().unwrap().content, "second");
    }

    #[test]
    fn paste_store_all_ids_in_order() {
        let mut store = PasteStore::new(5);
        store.store("x");
        store.store("y");
        store.store("z");
        assert_eq!(store.all_ids(), vec![1, 2, 3]);
    }

    #[test]
    fn paste_store_clear_resets() {
        let mut store = PasteStore::new(5);
        store.store("a");
        store.store("b");
        store.clear();
        assert_eq!(store.count(), 0);
        // After clear, next id restarts at 1.
        let e = store.store("fresh");
        assert_eq!(e.id, 1);
    }

    // ── PasteGuard ────────────────────────────────────────────────────────────

    #[test]
    fn paste_guard_small_paste_passes_inline() {
        let mut guard = PasteGuard::with_defaults();
        let small = bracketed("hello\nworld");
        let result = guard.process(&small);
        assert!(result.was_paste);
        assert!(!result.was_collapsed);
        assert!(result.processed_input.contains("hello"));
        assert!(result.processed_input.contains("world"));
    }

    #[test]
    fn paste_guard_collapses_large_paste() {
        let mut guard = PasteGuard::with_defaults();
        let large = bracketed(&make_paste(15));
        let result = guard.process(&large);
        assert!(result.was_paste);
        assert!(result.was_collapsed);
        assert!(
            result.processed_input.contains("[paste #1 +15 lines]"),
            "output: {}",
            result.processed_input
        );
    }

    #[test]
    fn paste_guard_collapse_includes_preview_lines() {
        let config = PasteGuardConfig {
            line_threshold: 5,
            show_preview_lines: 2,
            ..Default::default()
        };
        let mut guard = PasteGuard::new(config);
        let large = bracketed(&make_paste(10));
        let result = guard.process(&large);
        assert!(result.was_collapsed);
        assert!(result.processed_input.contains("line 1"));
        assert!(result.processed_input.contains("line 2"));
        // line 3 is beyond preview_lines=2 so should only appear inside stored content.
    }

    #[test]
    fn paste_guard_expand_marker_retrieves_content() {
        let mut guard = PasteGuard::with_defaults();
        let content = make_paste(20);
        let input = bracketed(&content);
        let result = guard.process(&input);
        assert!(result.was_collapsed);
        let marker = result
            .processed_input
            .lines()
            .find(|l| l.starts_with("[paste #"))
            .expect("marker not found in output");
        let expanded = guard
            .expand_marker(marker)
            .expect("expand_marker returned None");
        assert_eq!(expanded, content);
    }

    #[test]
    fn paste_guard_marker_to_id_parsing() {
        assert_eq!(PasteGuard::marker_to_id("[paste #1 +47 lines]"), Some(1));
        assert_eq!(PasteGuard::marker_to_id("[paste #99 +3 lines]"), Some(99));
        assert_eq!(PasteGuard::marker_to_id("not a marker"), None);
        assert_eq!(PasteGuard::marker_to_id("[paste #abc +1 lines]"), None);
        assert_eq!(PasteGuard::marker_to_id(""), None);
    }

    #[test]
    fn paste_guard_non_paste_input_unchanged() {
        let mut guard = PasteGuard::with_defaults();
        let result = guard.process("normal input without escapes");
        assert!(!result.was_paste);
        assert_eq!(result.processed_input, "normal input without escapes");
    }

    #[test]
    fn paste_guard_text_before_and_after_paste() {
        let mut guard = PasteGuard::with_defaults();
        let input = format!(
            "before{}hello{}after",
            BRACKETED_PASTE_START, BRACKETED_PASTE_END
        );
        let result = guard.process(&input);
        assert!(result.was_paste);
        assert!(result.processed_input.starts_with("before"));
        assert!(result.processed_input.ends_with("after"));
    }

    // ── Free functions ────────────────────────────────────────────────────────

    #[test]
    fn strip_bracketed_sequences_removes_both() {
        let s = format!("{}content{}", BRACKETED_PASTE_START, BRACKETED_PASTE_END);
        assert_eq!(strip_bracketed_sequences(&s), "content");
    }

    #[test]
    fn strip_bracketed_sequences_no_op_on_clean_input() {
        assert_eq!(strip_bracketed_sequences("plain text"), "plain text");
    }

    #[test]
    fn has_paste_start_detects_sequence() {
        assert!(has_paste_start(&format!("{}rest", BRACKETED_PASTE_START)));
        assert!(!has_paste_start("plain"));
    }

    #[test]
    fn extract_paste_content_returns_inner() {
        let raw = format!(
            "{}hello world{}",
            BRACKETED_PASTE_START, BRACKETED_PASTE_END
        );
        assert_eq!(extract_paste_content(&raw), Some("hello world".to_string()));
    }

    #[test]
    fn extract_paste_content_none_without_end() {
        let raw = format!("{}no end here", BRACKETED_PASTE_START);
        assert_eq!(extract_paste_content(&raw), None);
    }

    #[test]
    fn extract_paste_content_none_on_plain_input() {
        assert_eq!(extract_paste_content("no escape sequences"), None);
    }

    #[test]
    fn extract_paste_content_none_for_empty_paste() {
        let raw = format!("{}{}", BRACKETED_PASTE_START, BRACKETED_PASTE_END);
        assert_eq!(extract_paste_content(&raw), None);
    }
}
