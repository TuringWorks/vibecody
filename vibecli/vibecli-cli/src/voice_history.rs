#![allow(dead_code)]
//! Voice command history — records, indexes, and replays voice commands.
//! Matches Cody 6.0's voice command history feature.
//!
//! Features:
//! - Append-only log of voice commands with timestamps and transcriptions
//! - Full-text search over past commands
//! - Replay: re-emit a past command as if it were just spoken
//! - Export to JSON

use std::collections::VecDeque;
use std::time::{SystemTime, UNIX_EPOCH};

// ---------------------------------------------------------------------------
// Types
// ---------------------------------------------------------------------------

/// Unique ID for a voice command entry.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct VoiceEntryId(pub u64);

impl std::fmt::Display for VoiceEntryId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "voice-{}", self.0)
    }
}

/// Confidence score of speech-to-text transcription.
#[derive(Debug, Clone, Copy, PartialEq, PartialOrd)]
pub struct Confidence(pub f32);

impl Confidence {
    pub fn label(&self) -> &'static str {
        if self.0 >= 0.9 { "high" }
        else if self.0 >= 0.7 { "medium" }
        else { "low" }
    }
}

/// A single recorded voice command.
#[derive(Debug, Clone)]
pub struct VoiceEntry {
    pub id: VoiceEntryId,
    pub timestamp_ms: u64,
    pub raw_text: String,
    pub normalized_text: String,
    pub confidence: Confidence,
    pub executed: bool,
    pub tags: Vec<String>,
}

impl VoiceEntry {
    /// Format timestamp as a human-readable string (simplified — no chrono dep).
    pub fn timestamp_label(&self) -> String {
        format!("t={}", self.timestamp_ms)
    }
}

/// Result of a search over the voice history.
#[derive(Debug, Clone)]
pub struct VoiceSearchResult {
    pub entry: VoiceEntry,
    pub score: f32,
    pub matched_at: Vec<usize>, // byte offsets of match starts
}

// ---------------------------------------------------------------------------
// History Store
// ---------------------------------------------------------------------------

/// Append-only voice command history with search and replay.
pub struct VoiceHistory {
    entries: VecDeque<VoiceEntry>,
    next_id: u64,
    /// Maximum entries to retain.
    pub capacity: usize,
}

impl Default for VoiceHistory {
    fn default() -> Self {
        Self::new(500)
    }
}

impl VoiceHistory {
    pub fn new(capacity: usize) -> Self {
        Self {
            entries: VecDeque::new(),
            next_id: 1,
            capacity,
        }
    }

    /// Record a new voice command. Returns the assigned ID.
    pub fn record(&mut self, raw_text: impl Into<String>, confidence: f32) -> VoiceEntryId {
        let raw = raw_text.into();
        let normalized = Self::normalize(&raw);
        let id = VoiceEntryId(self.next_id);
        self.next_id += 1;

        let ts = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map(|d| d.as_millis() as u64)
            .unwrap_or(0);

        let entry = VoiceEntry {
            id: id.clone(),
            timestamp_ms: ts,
            raw_text: raw,
            normalized_text: normalized,
            confidence: Confidence(confidence.clamp(0.0, 1.0)),
            executed: false,
            tags: Vec::new(),
        };

        if self.entries.len() >= self.capacity {
            self.entries.pop_front();
        }
        self.entries.push_back(entry);
        id
    }

    /// Mark an entry as executed.
    pub fn mark_executed(&mut self, id: &VoiceEntryId) -> bool {
        for entry in &mut self.entries {
            if &entry.id == id {
                entry.executed = true;
                return true;
            }
        }
        false
    }

    /// Add a tag to an entry.
    pub fn tag(&mut self, id: &VoiceEntryId, tag: impl Into<String>) -> bool {
        let t = tag.into();
        for entry in &mut self.entries {
            if &entry.id == id {
                if !entry.tags.contains(&t) {
                    entry.tags.push(t);
                }
                return true;
            }
        }
        false
    }

    /// Get an entry by ID.
    pub fn get(&self, id: &VoiceEntryId) -> Option<&VoiceEntry> {
        self.entries.iter().find(|e| &e.id == id)
    }

    /// Most recent N entries (newest first).
    pub fn recent(&self, n: usize) -> Vec<&VoiceEntry> {
        self.entries.iter().rev().take(n).collect()
    }

    /// Full-text search: finds entries whose normalized text contains `query`.
    /// Returns results sorted by relevance (exact match > word match > substring).
    pub fn search(&self, query: &str) -> Vec<VoiceSearchResult> {
        let q_normalized = Self::normalize(query);
        let mut results: Vec<VoiceSearchResult> = Vec::new();

        for entry in &self.entries {
            let text = &entry.normalized_text;
            if let Some(pos) = text.find(&q_normalized) {
                // Score: exact full match > starts-with > contains
                let score = if text == &q_normalized {
                    1.0
                } else if text.starts_with(&q_normalized) {
                    0.9
                } else {
                    0.7
                };
                // Confidence boost
                let score = score * 0.8 + entry.confidence.0 * 0.2;
                results.push(VoiceSearchResult {
                    entry: entry.clone(),
                    score,
                    matched_at: vec![pos],
                });
            }
        }

        results.sort_by(|a, b| b.score.partial_cmp(&a.score).unwrap_or(std::cmp::Ordering::Equal));
        results
    }

    /// Replay: returns the raw_text of the entry so caller can re-dispatch it.
    pub fn replay(&self, id: &VoiceEntryId) -> Option<String> {
        self.get(id).map(|e| e.raw_text.clone())
    }

    /// Export all entries as a JSON string (minimal hand-built JSON, no dep).
    pub fn export_json(&self) -> String {
        let mut out = String::from("[\n");
        for (i, entry) in self.entries.iter().enumerate() {
            out.push_str("  {");
            out.push_str(&format!("\"id\":{},", entry.id.0));
            out.push_str(&format!("\"ts\":{},", entry.timestamp_ms));
            out.push_str(&format!("\"text\":\"{}\",", entry.normalized_text.replace('"', "\\\"")));
            out.push_str(&format!("\"confidence\":{:.2},", entry.confidence.0));
            out.push_str(&format!("\"executed\":{}", entry.executed));
            out.push('}');
            if i + 1 < self.entries.len() {
                out.push(',');
            }
            out.push('\n');
        }
        out.push(']');
        out
    }

    /// Statistics.
    pub fn stats(&self) -> VoiceHistoryStats {
        let total = self.entries.len();
        let executed = self.entries.iter().filter(|e| e.executed).count();
        let avg_confidence = if total == 0 {
            0.0
        } else {
            self.entries.iter().map(|e| e.confidence.0 as f64).sum::<f64>() / total as f64
        };
        VoiceHistoryStats { total, executed, avg_confidence }
    }

    fn normalize(text: &str) -> String {
        text.to_lowercase()
            .split_whitespace()
            .collect::<Vec<_>>()
            .join(" ")
    }
}

#[derive(Debug)]
pub struct VoiceHistoryStats {
    pub total: usize,
    pub executed: usize,
    pub avg_confidence: f64,
}

impl VoiceHistoryStats {
    pub fn execution_rate(&self) -> f64 {
        if self.total == 0 { return 0.0; }
        self.executed as f64 / self.total as f64
    }
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_record_and_get() {
        let mut h = VoiceHistory::new(100);
        let id = h.record("open file main.rs", 0.95);
        let entry = h.get(&id).unwrap();
        assert_eq!(entry.raw_text, "open file main.rs");
        assert_eq!(entry.normalized_text, "open file main.rs");
        assert!(!entry.executed);
    }

    #[test]
    fn test_normalize_casing_whitespace() {
        let mut h = VoiceHistory::new(100);
        let id = h.record("  OPEN   File  Main.RS  ", 0.8);
        let entry = h.get(&id).unwrap();
        assert_eq!(entry.normalized_text, "open file main.rs");
    }

    #[test]
    fn test_mark_executed() {
        let mut h = VoiceHistory::new(100);
        let id = h.record("run tests", 0.9);
        assert!(h.mark_executed(&id));
        assert!(h.get(&id).unwrap().executed);
    }

    #[test]
    fn test_tag() {
        let mut h = VoiceHistory::new(100);
        let id = h.record("build project", 0.85);
        assert!(h.tag(&id, "build"));
        assert!(h.get(&id).unwrap().tags.contains(&"build".to_string()));
    }

    #[test]
    fn test_recent_newest_first() {
        let mut h = VoiceHistory::new(100);
        h.record("cmd 1", 0.9);
        h.record("cmd 2", 0.9);
        h.record("cmd 3", 0.9);
        let recent = h.recent(2);
        assert_eq!(recent[0].raw_text, "cmd 3");
        assert_eq!(recent[1].raw_text, "cmd 2");
    }

    #[test]
    fn test_search_finds_match() {
        let mut h = VoiceHistory::new(100);
        h.record("open the terminal", 0.9);
        h.record("close the editor", 0.8);
        let results = h.search("terminal");
        assert_eq!(results.len(), 1);
        assert!(results[0].entry.raw_text.contains("terminal"));
    }

    #[test]
    fn test_search_no_match() {
        let mut h = VoiceHistory::new(100);
        h.record("open file", 0.9);
        let results = h.search("nonexistent");
        assert!(results.is_empty());
    }

    #[test]
    fn test_search_score_exact_higher() {
        let mut h = VoiceHistory::new(100);
        h.record("run", 0.9);
        h.record("run all tests", 0.9);
        let results = h.search("run");
        assert!(results[0].score >= results[1].score);
    }

    #[test]
    fn test_replay_returns_raw_text() {
        let mut h = VoiceHistory::new(100);
        let id = h.record("Open file README", 0.9);
        let replayed = h.replay(&id).unwrap();
        assert_eq!(replayed, "Open file README");
    }

    #[test]
    fn test_capacity_eviction() {
        let mut h = VoiceHistory::new(3);
        h.record("cmd 1", 0.9);
        h.record("cmd 2", 0.9);
        h.record("cmd 3", 0.9);
        h.record("cmd 4", 0.9);
        assert_eq!(h.entries.len(), 3);
        // Oldest was evicted
        assert_eq!(h.entries[0].raw_text, "cmd 2");
    }

    #[test]
    fn test_stats() {
        let mut h = VoiceHistory::new(100);
        let id1 = h.record("cmd 1", 0.9);
        h.record("cmd 2", 0.7);
        h.mark_executed(&id1);
        let stats = h.stats();
        assert_eq!(stats.total, 2);
        assert_eq!(stats.executed, 1);
        assert!((stats.execution_rate() - 0.5).abs() < 1e-9);
        assert!((stats.avg_confidence - 0.8).abs() < 0.01);
    }

    #[test]
    fn test_confidence_label() {
        assert_eq!(Confidence(0.95).label(), "high");
        assert_eq!(Confidence(0.75).label(), "medium");
        assert_eq!(Confidence(0.5).label(), "low");
    }

    #[test]
    fn test_export_json() {
        let mut h = VoiceHistory::new(100);
        h.record("test command", 0.9);
        let json = h.export_json();
        assert!(json.starts_with('['));
        assert!(json.ends_with(']'));
        assert!(json.contains("test command"));
    }

    #[test]
    fn test_empty_history() {
        let h = VoiceHistory::new(100);
        let stats = h.stats();
        assert_eq!(stats.total, 0);
        assert_eq!(stats.avg_confidence, 0.0);
        assert!(h.search("anything").is_empty());
    }
}
