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
