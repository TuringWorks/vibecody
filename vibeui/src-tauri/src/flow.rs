//! Flow Awareness Engine — tracks developer activity and provides context to AI.
//!
//! Records: file opens, file edits (debounced), terminal commands run.
//! Exposed to the AI via `get_flow_context()` which formats recent events into
//! a human-readable summary suitable for injecting into the system prompt.

use std::collections::VecDeque;
use std::time::{SystemTime, UNIX_EPOCH};

/// Maximum events to retain in the ring buffer.
const MAX_EVENTS: usize = 100;

/// A single tracked developer action.
#[derive(Debug, Clone)]
pub struct FlowEvent {
    /// Category: "file_open", "file_edit", "terminal_cmd", "file_save"
    pub kind: String,
    /// Payload: path, command, etc.
    pub data: String,
    /// Unix timestamp (seconds).
    #[allow(dead_code)]
    pub timestamp: u64,
}

/// Ring-buffer based tracker for recent developer activity.
pub struct FlowTracker {
    events: VecDeque<FlowEvent>,
}

impl FlowTracker {
    pub fn new() -> Self {
        Self {
            events: VecDeque::with_capacity(MAX_EVENTS),
        }
    }

    /// Record a new flow event.
    pub fn record(&mut self, kind: &str, data: &str) {
        let ts = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map(|d| d.as_secs())
            .unwrap_or(0);
        if self.events.len() >= MAX_EVENTS {
            self.events.pop_front();
        }
        self.events.push_back(FlowEvent {
            kind: kind.to_string(),
            data: data.to_string(),
            timestamp: ts,
        });
    }

    /// Return a formatted context string summarising the most recent `limit` events.
    pub fn context_string(&self, limit: usize) -> String {
        if self.events.is_empty() {
            return String::new();
        }

        let recent: Vec<&FlowEvent> = self.events.iter().rev().take(limit).collect::<Vec<_>>().into_iter().rev().collect();

        // Summarise by category for brevity
        let opens: Vec<&str> = recent
            .iter()
            .filter(|e| e.kind == "file_open")
            .map(|e| e.data.as_str())
            .collect();
        let edits: Vec<&str> = recent
            .iter()
            .filter(|e| e.kind == "file_edit" || e.kind == "file_save")
            .map(|e| e.data.as_str())
            .collect();
        let cmds: Vec<&str> = recent
            .iter()
            .filter(|e| e.kind == "terminal_cmd")
            .map(|e| e.data.as_str())
            .collect();

        let mut ctx = String::from("## Recent Activity\n");
        if !opens.is_empty() {
            // Deduplicate, keep last 5
            let unique: Vec<&str> = {
                let mut seen = std::collections::HashSet::new();
                opens.iter().rev().filter(|&&p| seen.insert(p)).take(5).copied().collect::<Vec<_>>().into_iter().rev().collect()
            };
            ctx.push_str(&format!("Recently opened files: {}\n", unique.join(", ")));
        }
        if !edits.is_empty() {
            let unique: Vec<&str> = {
                let mut seen = std::collections::HashSet::new();
                edits.iter().rev().filter(|&&p| seen.insert(p)).take(5).copied().collect::<Vec<_>>().into_iter().rev().collect()
            };
            ctx.push_str(&format!("Recently edited files: {}\n", unique.join(", ")));
        }
        if !cmds.is_empty() {
            let last: Vec<&str> = cmds.iter().rev().take(5).copied().collect::<Vec<_>>().into_iter().rev().collect();
            ctx.push_str(&format!("Recent terminal commands: {}\n", last.join("; ")));
        }
        ctx
    }
}

impl Default for FlowTracker {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn new_tracker_is_empty() {
        let tracker = FlowTracker::new();
        assert!(tracker.events.is_empty());
        assert_eq!(tracker.context_string(10), "");
    }

    #[test]
    fn default_is_same_as_new() {
        let tracker = FlowTracker::default();
        assert!(tracker.events.is_empty());
    }

    #[test]
    fn record_adds_event() {
        let mut tracker = FlowTracker::new();
        tracker.record("file_open", "src/main.rs");
        assert_eq!(tracker.events.len(), 1);
        assert_eq!(tracker.events[0].kind, "file_open");
        assert_eq!(tracker.events[0].data, "src/main.rs");
        assert!(tracker.events[0].timestamp > 0);
    }

    #[test]
    fn record_multiple_events() {
        let mut tracker = FlowTracker::new();
        tracker.record("file_open", "a.rs");
        tracker.record("file_edit", "a.rs");
        tracker.record("terminal_cmd", "cargo build");
        assert_eq!(tracker.events.len(), 3);
    }

    #[test]
    fn ring_buffer_evicts_oldest_at_capacity() {
        let mut tracker = FlowTracker::new();
        for i in 0..MAX_EVENTS {
            tracker.record("file_open", &format!("file_{}.rs", i));
        }
        assert_eq!(tracker.events.len(), MAX_EVENTS);

        // Add one more — oldest (file_0.rs) should be evicted
        tracker.record("file_open", "overflow.rs");
        assert_eq!(tracker.events.len(), MAX_EVENTS);
        assert_eq!(tracker.events.front().unwrap().data, "file_1.rs");
        assert_eq!(tracker.events.back().unwrap().data, "overflow.rs");
    }

    #[test]
    fn context_string_empty_tracker() {
        let tracker = FlowTracker::new();
        assert_eq!(tracker.context_string(50), "");
    }

    #[test]
    fn context_string_file_opens() {
        let mut tracker = FlowTracker::new();
        tracker.record("file_open", "src/main.rs");
        tracker.record("file_open", "src/lib.rs");
        let ctx = tracker.context_string(10);
        assert!(ctx.contains("## Recent Activity"));
        assert!(ctx.contains("Recently opened files:"));
        assert!(ctx.contains("src/main.rs"));
        assert!(ctx.contains("src/lib.rs"));
    }

    #[test]
    fn context_string_file_edits_and_saves() {
        let mut tracker = FlowTracker::new();
        tracker.record("file_edit", "a.rs");
        tracker.record("file_save", "b.rs");
        let ctx = tracker.context_string(10);
        assert!(ctx.contains("Recently edited files:"));
        assert!(ctx.contains("a.rs"));
        assert!(ctx.contains("b.rs"));
    }

    #[test]
    fn context_string_terminal_commands() {
        let mut tracker = FlowTracker::new();
        tracker.record("terminal_cmd", "cargo build");
        tracker.record("terminal_cmd", "cargo test");
        let ctx = tracker.context_string(10);
        assert!(ctx.contains("Recent terminal commands:"));
        assert!(ctx.contains("cargo build"));
        assert!(ctx.contains("cargo test"));
        // Commands are joined with "; "
        assert!(ctx.contains("; "));
    }

    #[test]
    fn context_string_deduplicates_opens() {
        let mut tracker = FlowTracker::new();
        tracker.record("file_open", "src/main.rs");
        tracker.record("file_open", "src/main.rs");
        tracker.record("file_open", "src/main.rs");
        let ctx = tracker.context_string(10);
        // Should only appear once after dedup
        let count = ctx.matches("src/main.rs").count();
        assert_eq!(count, 1);
    }

    #[test]
    fn context_string_deduplicates_edits() {
        let mut tracker = FlowTracker::new();
        tracker.record("file_edit", "a.rs");
        tracker.record("file_edit", "a.rs");
        tracker.record("file_edit", "b.rs");
        let ctx = tracker.context_string(10);
        let count = ctx.matches("a.rs").count();
        assert_eq!(count, 1);
    }

    #[test]
    fn context_string_limits_opens_to_five() {
        let mut tracker = FlowTracker::new();
        for i in 0..10 {
            tracker.record("file_open", &format!("file_{}.rs", i));
        }
        let ctx = tracker.context_string(100);
        // Only last 5 unique opens should appear
        assert!(!ctx.contains("file_0.rs"));
        assert!(ctx.contains("file_9.rs"));
    }

    #[test]
    fn context_string_limit_parameter() {
        let mut tracker = FlowTracker::new();
        tracker.record("file_open", "a.rs");
        tracker.record("terminal_cmd", "cargo build");
        tracker.record("file_open", "b.rs");
        // limit=1 should only show the most recent event (b.rs)
        let ctx = tracker.context_string(1);
        assert!(ctx.contains("b.rs"));
        assert!(!ctx.contains("cargo build"));
    }

    #[test]
    fn context_string_only_terminal_no_files() {
        let mut tracker = FlowTracker::new();
        tracker.record("terminal_cmd", "ls -la");
        let ctx = tracker.context_string(10);
        assert!(ctx.contains("Recent terminal commands:"));
        assert!(!ctx.contains("opened"));
        assert!(!ctx.contains("edited"));
    }

    #[test]
    fn context_string_mixed_categories() {
        let mut tracker = FlowTracker::new();
        tracker.record("file_open", "main.rs");
        tracker.record("file_edit", "lib.rs");
        tracker.record("terminal_cmd", "cargo test");
        let ctx = tracker.context_string(10);
        assert!(ctx.contains("Recently opened files:"));
        assert!(ctx.contains("Recently edited files:"));
        assert!(ctx.contains("Recent terminal commands:"));
    }

    #[test]
    fn context_string_unknown_kind_is_ignored() {
        let mut tracker = FlowTracker::new();
        tracker.record("unknown_kind", "something");
        let ctx = tracker.context_string(10);
        // The header appears since events is non-empty, but no category section
        assert!(ctx.contains("## Recent Activity"));
        assert!(!ctx.contains("something"));
    }
}
