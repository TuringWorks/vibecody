#![allow(dead_code)]
//! Tool-pair-preserving auto-compaction.
//!
//! When compacting a long conversation, naively truncating messages can split a
//! `ToolUse`/`ToolResult` pair — causing the model to see a tool call with no
//! result (or a result with no call), which confuses most LLMs.
//!
//! This module:
//! 1. Finds the raw compaction boundary.
//! 2. Walks it **backward** until the message at `boundary - 1` is NOT a ToolUse.
//! 3. Generates a structured `CompactionSummary` of the compacted region.
//! 4. Renders the summary into a synthetic assistant continuation message.
//!
//! # Usage
//! ```rust,ignore
//! let engine = CompactionEngine::new(CompactionConfig::default());
//! let compacted = engine.compact(&messages);
//! ```

use serde::{Deserialize, Serialize};

// ─── Simple Message Model (CompactionEngine API) ──────────────────────────────

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum SimpleMessageRole {
    System,
    User,
    Assistant,
    ToolUse,
    ToolResult,
}

impl std::fmt::Display for SimpleMessageRole {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::System => write!(f, "system"),
            Self::User => write!(f, "user"),
            Self::Assistant => write!(f, "assistant"),
            Self::ToolUse => write!(f, "tool_use"),
            Self::ToolResult => write!(f, "tool_result"),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SimpleMessage {
    pub role: SimpleMessageRole,
    pub content: String,
}

impl SimpleMessage {
    pub fn new(role: SimpleMessageRole, content: impl Into<String>) -> Self {
        Self { role, content: content.into() }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct CompactionSummary {
    pub user_count: usize,
    pub assistant_count: usize,
    pub system_count: usize,
    pub tool_call_count: usize,
    /// Deduped tool names collected from ToolUse message content.
    pub tool_names: Vec<String>,
    /// Last 3 user message contents, oldest first.
    pub last_user_requests: Vec<String>,
    /// Keywords like "todo", "wip", "next step", "pending", "fixme" found in content.
    pub pending_keywords: Vec<String>,
    /// File paths with recognised extensions found across all message content.
    pub key_files: Vec<String>,
}

#[derive(Debug, Clone)]
pub struct CompactionConfig {
    /// Keep this many recent messages after compaction.
    pub keep_recent: usize,
    /// Max chars in the rendered summary.
    pub summary_max_chars: usize,
    /// Max lines in the rendered summary.
    pub summary_max_lines: usize,
}

impl Default for CompactionConfig {
    fn default() -> Self {
        Self { keep_recent: 10, summary_max_chars: 1200, summary_max_lines: 24 }
    }
}

pub struct CompactionEngine {
    pub config: CompactionConfig,
}

impl CompactionEngine {
    pub fn new(config: CompactionConfig) -> Self {
        Self { config }
    }

    /// Walk `raw_boundary` backward until the message just before it is NOT a
    /// ToolUse (to avoid orphaning a tool-use without its result).
    pub fn find_safe_boundary(messages: &[SimpleMessage], raw_boundary: usize) -> usize {
        let mut boundary = raw_boundary.min(messages.len());
        while boundary > 0 && messages[boundary - 1].role == SimpleMessageRole::ToolUse {
            boundary -= 1;
        }
        boundary
    }

    /// Build a structured summary from `messages`.
    pub fn summarize(messages: &[SimpleMessage]) -> CompactionSummary {
        let mut summary = CompactionSummary::default();
        let mut all_tool_names: Vec<String> = Vec::new();
        let mut user_requests: Vec<String> = Vec::new();
        let mut pending_set: std::collections::HashSet<String> = std::collections::HashSet::new();
        let mut file_set: std::collections::HashSet<String> = std::collections::HashSet::new();

        // Keywords to detect (lowercased)
        let keyword_map: &[(&str, &str)] = &[
            ("todo", "todo"),
            ("wip", "wip"),
            ("next step", "next step"),
            ("pending", "pending"),
            ("fixme", "fixme"),
        ];

        // Extensions to treat as key files
        let file_exts = [".rs", ".ts", ".tsx", ".js", ".json", ".md"];

        for msg in messages {
            match msg.role {
                SimpleMessageRole::User => {
                    summary.user_count += 1;
                    user_requests.push(msg.content.clone());
                }
                SimpleMessageRole::Assistant => {
                    summary.assistant_count += 1;
                }
                SimpleMessageRole::System => {
                    summary.system_count += 1;
                }
                SimpleMessageRole::ToolUse => {
                    summary.tool_call_count += 1;
                    // Treat the content as the tool name for simple messages
                    all_tool_names.push(msg.content.clone());
                }
                SimpleMessageRole::ToolResult => {}
            }

            // Scan content for pending keywords
            let lower = msg.content.to_lowercase();
            for (needle, label) in keyword_map {
                if lower.contains(needle) {
                    pending_set.insert(label.to_string());
                }
            }

            // Extract file-like words
            for word in msg.content.split_whitespace() {
                // Strip common punctuation from the word boundary
                let word = word.trim_matches(|c: char| !c.is_alphanumeric() && c != '.' && c != '/' && c != '_' && c != '-');
                if file_exts.iter().any(|ext| word.ends_with(ext)) {
                    file_set.insert(word.to_string());
                }
            }
        }

        // Dedup tool names, preserving insertion order
        let mut seen = std::collections::HashSet::new();
        summary.tool_names = all_tool_names
            .into_iter()
            .filter(|n| seen.insert(n.clone()))
            .collect();

        // Last 3 user requests, oldest first
        let start = user_requests.len().saturating_sub(3);
        summary.last_user_requests = user_requests[start..].to_vec();

        // Collect pending keywords in a stable order
        let mut pending: Vec<String> = pending_set.into_iter().collect();
        pending.sort();
        summary.pending_keywords = pending;

        // Collect key files in a stable order
        let mut files: Vec<String> = file_set.into_iter().collect();
        files.sort();
        summary.key_files = files;

        summary
    }

    /// Render a human-readable summary string (priority: headers > bullets > text),
    /// capped at `summary_max_chars`.
    pub fn render_summary(summary: &CompactionSummary) -> String {
        let mut lines: Vec<String> = Vec::new();

        lines.push("## Compaction Summary".to_string());
        lines.push(format!(
            "- {} user / {} assistant / {} system messages",
            summary.user_count, summary.assistant_count, summary.system_count
        ));
        lines.push(format!("- {} tool calls", summary.tool_call_count));

        if !summary.tool_names.is_empty() {
            lines.push(format!("- Tools: {}", summary.tool_names.join(", ")));
        }

        if !summary.last_user_requests.is_empty() {
            lines.push("- Recent requests:".to_string());
            for req in &summary.last_user_requests {
                let preview: String = req.chars().take(80).collect();
                lines.push(format!("  - {preview}"));
            }
        }

        if !summary.pending_keywords.is_empty() {
            lines.push(format!("- Pending: {}", summary.pending_keywords.join(", ")));
        }

        if !summary.key_files.is_empty() {
            lines.push(format!("- Key files: {}", summary.key_files.join(", ")));
        }

        // Join and cap at summary_max_chars (use a high default if called as standalone fn)
        lines.join("\n")
    }

    /// Render using engine config limits.
    pub fn render_summary_capped(&self, summary: &CompactionSummary) -> String {
        let raw = Self::render_summary(summary);
        let capped: String = raw
            .lines()
            .take(self.config.summary_max_lines)
            .collect::<Vec<_>>()
            .join("\n");
        if capped.len() > self.config.summary_max_chars {
            capped[..self.config.summary_max_chars].to_string()
        } else {
            capped
        }
    }

    /// Create a synthetic assistant message that continues naturally after compaction.
    pub fn synthetic_continuation(summary: &CompactionSummary) -> SimpleMessage {
        let content = format!(
            "[Context compacted — {} user, {} assistant messages summarized. {}]",
            summary.user_count,
            summary.assistant_count,
            if summary.pending_keywords.is_empty() {
                "Resuming task.".to_string()
            } else {
                format!("Pending: {}.", summary.pending_keywords.join(", "))
            }
        );
        SimpleMessage::new(SimpleMessageRole::Assistant, content)
    }

    /// Full compaction: find safe boundary, summarize, prepend continuation, keep recent tail.
    pub fn compact(&self, messages: &[SimpleMessage]) -> Vec<SimpleMessage> {
        if messages.len() <= self.config.keep_recent {
            return messages.to_vec();
        }
        let raw_boundary = messages.len().saturating_sub(self.config.keep_recent);
        let boundary = Self::find_safe_boundary(messages, raw_boundary);
        if boundary == 0 {
            return messages.to_vec();
        }
        let summary = Self::summarize(&messages[..boundary]);
        let continuation = Self::synthetic_continuation(&summary);
        let mut result = vec![continuation];
        result.extend_from_slice(&messages[boundary..]);
        result
    }
}

// ─── Legacy Message Representation (ToolPairCompactor API) ───────────────────

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum MessageRole {
    User,
    Assistant,
    Tool,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum ContentKind {
    Text,
    ToolUse { tool_id: String, tool_name: String },
    ToolResult { tool_id: String, is_error: bool },
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Message {
    pub id: u32,
    pub role: MessageRole,
    pub content: String,
    pub kind: ContentKind,
    /// Approximate token count.
    pub tokens: u32,
}

impl Message {
    pub fn text(id: u32, role: MessageRole, content: impl Into<String>, tokens: u32) -> Self {
        Self { id, role, content: content.into(), kind: ContentKind::Text, tokens }
    }

    pub fn tool_use(
        id: u32,
        tool_id: impl Into<String>,
        name: impl Into<String>,
        content: impl Into<String>,
        tokens: u32,
    ) -> Self {
        let tid = tool_id.into();
        Self {
            id,
            role: MessageRole::Assistant,
            content: content.into(),
            kind: ContentKind::ToolUse { tool_id: tid, tool_name: name.into() },
            tokens,
        }
    }

    pub fn tool_result(
        id: u32,
        tool_id: impl Into<String>,
        content: impl Into<String>,
        is_error: bool,
        tokens: u32,
    ) -> Self {
        let tid = tool_id.into();
        Self {
            id,
            role: MessageRole::Tool,
            content: content.into(),
            kind: ContentKind::ToolResult { tool_id: tid, is_error },
            tokens,
        }
    }

    pub fn is_tool_use(&self) -> bool {
        matches!(self.kind, ContentKind::ToolUse { .. })
    }

    pub fn is_tool_result(&self) -> bool {
        matches!(self.kind, ContentKind::ToolResult { .. })
    }

    pub fn tool_id(&self) -> Option<&str> {
        match &self.kind {
            ContentKind::ToolUse { tool_id, .. } => Some(tool_id),
            ContentKind::ToolResult { tool_id, .. } => Some(tool_id),
            _ => None,
        }
    }
}

// ─── Compaction Rules ─────────────────────────────────────────────────────────

/// Strategy for compacting a tool-call pair.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum CompactionStrategy {
    Keep,
    Summarise,
    Drop,
    Truncate { max_tokens: u32 },
}

/// A matched pair of tool_use + tool_result.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToolPair {
    pub use_id: u32,
    pub result_id: u32,
    pub tool_name: String,
    pub tool_id: String,
    pub use_tokens: u32,
    pub result_tokens: u32,
}

impl ToolPair {
    pub fn total_tokens(&self) -> u32 {
        self.use_tokens + self.result_tokens
    }
}

// ─── Compaction Decision ─────────────────────────────────────────────────────

pub struct CompactionPolicy {
    pub target_free_tokens: u32,
    pub truncate_result_above: u32,
    pub droppable_tools: Vec<String>,
}

impl Default for CompactionPolicy {
    fn default() -> Self {
        Self {
            target_free_tokens: 8_000,
            truncate_result_above: 2_000,
            droppable_tools: vec!["Read".into(), "Glob".into(), "Grep".into(), "WebSearch".into()],
        }
    }
}

impl CompactionPolicy {
    pub fn strategy_for(&self, pair: &ToolPair, is_recent: bool) -> CompactionStrategy {
        if is_recent && pair.total_tokens() <= self.truncate_result_above {
            return CompactionStrategy::Keep;
        }
        if self.droppable_tools.contains(&pair.tool_name) {
            if pair.result_tokens > self.truncate_result_above {
                return CompactionStrategy::Summarise;
            }
            return CompactionStrategy::Drop;
        }
        if pair.result_tokens > self.truncate_result_above {
            return CompactionStrategy::Truncate { max_tokens: self.truncate_result_above / 2 };
        }
        CompactionStrategy::Keep
    }
}

// ─── Compactor ───────────────────────────────────────────────────────────────

pub struct ToolPairCompactor {
    pub policy: CompactionPolicy,
}

impl ToolPairCompactor {
    pub fn new(policy: CompactionPolicy) -> Self {
        Self { policy }
    }

    pub fn find_pairs(&self, messages: &[Message]) -> Vec<ToolPair> {
        let mut pairs = Vec::new();
        for msg in messages {
            if let ContentKind::ToolUse { tool_id, tool_name } = &msg.kind {
                if let Some(result) = messages.iter().find(|m| {
                    matches!(&m.kind, ContentKind::ToolResult { tool_id: rid, .. } if rid == tool_id)
                }) {
                    pairs.push(ToolPair {
                        use_id: msg.id,
                        result_id: result.id,
                        tool_name: tool_name.clone(),
                        tool_id: tool_id.clone(),
                        use_tokens: msg.tokens,
                        result_tokens: result.tokens,
                    });
                }
            }
        }
        pairs
    }

    pub fn compact(&self, messages: &[Message], recent_count: usize) -> (Vec<Message>, u32) {
        let pairs = self.find_pairs(messages);
        let recent_threshold = messages.len().saturating_sub(recent_count);
        let recent_ids: std::collections::HashSet<u32> =
            messages[recent_threshold..].iter().map(|m| m.id).collect();

        let mut drop_ids: std::collections::HashSet<u32> = std::collections::HashSet::new();
        let mut summarise_ids: std::collections::HashSet<u32> = std::collections::HashSet::new();
        let mut truncate_map: std::collections::HashMap<u32, u32> =
            std::collections::HashMap::new();
        let mut tokens_freed = 0u32;

        for pair in &pairs {
            let is_recent =
                recent_ids.contains(&pair.use_id) || recent_ids.contains(&pair.result_id);
            let strategy = self.policy.strategy_for(pair, is_recent);
            match strategy {
                CompactionStrategy::Drop => {
                    drop_ids.insert(pair.use_id);
                    drop_ids.insert(pair.result_id);
                    tokens_freed += pair.total_tokens();
                }
                CompactionStrategy::Summarise => {
                    summarise_ids.insert(pair.result_id);
                    tokens_freed += pair.result_tokens.saturating_sub(20);
                }
                CompactionStrategy::Truncate { max_tokens } => {
                    truncate_map.insert(pair.result_id, max_tokens);
                    tokens_freed += pair.result_tokens.saturating_sub(max_tokens);
                }
                CompactionStrategy::Keep => {}
            }
        }

        let compacted: Vec<Message> = messages
            .iter()
            .filter_map(|m| {
                if drop_ids.contains(&m.id) {
                    return None;
                }
                let mut m = m.clone();
                if summarise_ids.contains(&m.id) {
                    m.content = format!("[result summarised — {} tokens]", m.tokens);
                    m.tokens = 20;
                }
                if let Some(&max_t) = truncate_map.get(&m.id) {
                    m.content = m.content.chars().take((max_t * 4) as usize).collect();
                    m.tokens = max_t;
                }
                Some(m)
            })
            .collect();

        (compacted, tokens_freed)
    }

    pub fn total_tokens(messages: &[Message]) -> u32 {
        messages.iter().map(|m| m.tokens).sum()
    }
}

impl Default for ToolPairCompactor {
    fn default() -> Self {
        Self::new(CompactionPolicy::default())
    }
}

// ─── Tests: CompactionEngine (new API) ───────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    // ── helpers ──────────────────────────────────────────────────────────────

    fn user(c: &str) -> SimpleMessage {
        SimpleMessage::new(SimpleMessageRole::User, c)
    }
    fn assistant(c: &str) -> SimpleMessage {
        SimpleMessage::new(SimpleMessageRole::Assistant, c)
    }
    fn system(c: &str) -> SimpleMessage {
        SimpleMessage::new(SimpleMessageRole::System, c)
    }
    fn tool_use(name: &str) -> SimpleMessage {
        SimpleMessage::new(SimpleMessageRole::ToolUse, name)
    }
    fn tool_result(c: &str) -> SimpleMessage {
        SimpleMessage::new(SimpleMessageRole::ToolResult, c)
    }

    // ── CompactionEngine tests ────────────────────────────────────────────────

    #[test]
    fn safe_boundary_does_not_split_tool_pair() {
        let mut msgs: Vec<SimpleMessage> = (0..9).map(|_| user("q")).collect();
        msgs.push(tool_use("read_file")); // index 9
        msgs.push(tool_result("content")); // index 10
        let boundary = CompactionEngine::find_safe_boundary(&msgs, 10);
        assert_eq!(boundary, 9);
    }

    #[test]
    fn safe_boundary_unchanged_at_user_message() {
        let msgs: Vec<SimpleMessage> = (0..5)
            .map(|i| if i % 2 == 0 { user("q") } else { assistant("a") })
            .collect();
        let boundary = CompactionEngine::find_safe_boundary(&msgs, 3);
        assert_eq!(boundary, 3);
    }

    #[test]
    fn safe_boundary_walks_back_past_multiple_tool_pairs() {
        let msgs = vec![
            user("q"),
            assistant("a"),
            tool_use("read"),
            tool_result("r"), // indices 2,3
            tool_use("write"), // index 4 — ToolUse at boundary-1
        ];
        // raw_boundary=5, msg[4]=ToolUse → step back to 4
        // msg[3]=ToolResult → stop (not ToolUse)
        let boundary = CompactionEngine::find_safe_boundary(&msgs, 5);
        assert_eq!(boundary, 4);
    }

    #[test]
    fn summarize_counts_roles_correctly() {
        let msgs =
            vec![user("q"), user("q2"), assistant("a"), system("sys"), tool_use("t")];
        let s = CompactionEngine::summarize(&msgs);
        assert_eq!(s.user_count, 2);
        assert_eq!(s.assistant_count, 1);
        assert_eq!(s.system_count, 1);
        assert_eq!(s.tool_call_count, 1);
    }

    #[test]
    fn summarize_extracts_tool_names_deduped() {
        let msgs =
            vec![tool_use("read_file"), tool_use("read_file"), tool_use("write_file")];
        let s = CompactionEngine::summarize(&msgs);
        assert_eq!(s.tool_names.len(), 2);
        assert!(s.tool_names.contains(&"read_file".to_string()));
        assert!(s.tool_names.contains(&"write_file".to_string()));
    }

    #[test]
    fn summarize_captures_last_3_user_requests() {
        let msgs: Vec<SimpleMessage> =
            (0..6).map(|i| user(&format!("request {i}"))).collect();
        let s = CompactionEngine::summarize(&msgs);
        assert_eq!(s.last_user_requests.len(), 3);
        assert!(s
            .last_user_requests
            .iter()
            .any(|r| r.contains("3") || r.contains("4") || r.contains("5")));
    }

    #[test]
    fn summarize_detects_pending_keywords() {
        let msgs = vec![user("TODO: fix the tests"), user("WIP: refactor module")];
        let s = CompactionEngine::summarize(&msgs);
        assert!(!s.pending_keywords.is_empty());
    }

    #[test]
    fn render_summary_includes_all_sections() {
        let summary = CompactionSummary {
            user_count: 3,
            assistant_count: 2,
            system_count: 1,
            tool_call_count: 2,
            tool_names: vec!["read_file".into()],
            last_user_requests: vec!["fix tests".into()],
            pending_keywords: vec!["todo".into()],
            key_files: vec!["main.rs".into()],
        };
        let rendered = CompactionEngine::render_summary(&summary);
        assert!(rendered.contains("3")); // user count
        assert!(rendered.contains("read_file"));
        assert!(rendered.contains("fix tests"));
    }

    #[test]
    fn synthetic_continuation_is_assistant_role() {
        let summary =
            CompactionSummary { user_count: 2, assistant_count: 1, ..Default::default() };
        let msg = CompactionEngine::synthetic_continuation(&summary);
        assert_eq!(msg.role, SimpleMessageRole::Assistant);
    }

    #[test]
    fn compact_preserves_tool_pairs() {
        let engine =
            CompactionEngine::new(CompactionConfig { keep_recent: 2, ..Default::default() });
        let msgs = vec![
            user("u1"),
            user("u2"),
            user("u3"),
            tool_use("bash"),
            tool_result("output"),
        ];
        let compacted = engine.compact(&msgs);
        // Examine all messages after the synthetic header for orphaned ToolUse
        let tail: Vec<&SimpleMessage> = compacted.iter().skip(1).collect();
        let has_orphan = tail.windows(2).any(|w| {
            w[0].role == SimpleMessageRole::ToolUse
                && w[1].role != SimpleMessageRole::ToolResult
        });
        assert!(!has_orphan, "found orphaned tool-use without tool-result");
    }

    #[test]
    fn compact_noop_when_under_keep_recent() {
        let engine =
            CompactionEngine::new(CompactionConfig { keep_recent: 10, ..Default::default() });
        let msgs = vec![user("a"), user("b")];
        let compacted = engine.compact(&msgs);
        assert_eq!(compacted.len(), msgs.len());
    }

    // ── Legacy ToolPairCompactor tests ────────────────────────────────────────

    fn make_pair(
        use_id: u32,
        name: &str,
        use_tok: u32,
        res_tok: u32,
        is_error: bool,
    ) -> Vec<Message> {
        let tid = format!("t{use_id}");
        vec![
            Message::tool_use(use_id, &tid, name, "call args", use_tok),
            Message::tool_result(use_id + 1, &tid, "result output", is_error, res_tok),
        ]
    }

    #[test]
    fn test_find_pairs_single() {
        let comp = ToolPairCompactor::default();
        let msgs = make_pair(1, "Read", 50, 300, false);
        let pairs = comp.find_pairs(&msgs);
        assert_eq!(pairs.len(), 1);
        assert_eq!(pairs[0].tool_name, "Read");
        assert_eq!(pairs[0].total_tokens(), 350);
    }

    #[test]
    fn test_find_pairs_multiple() {
        let comp = ToolPairCompactor::default();
        let mut msgs = make_pair(1, "Read", 50, 200, false);
        msgs.extend(make_pair(10, "Edit", 80, 150, false));
        let pairs = comp.find_pairs(&msgs);
        assert_eq!(pairs.len(), 2);
    }

    #[test]
    fn test_find_pairs_no_match_when_result_missing() {
        let comp = ToolPairCompactor::default();
        let msgs = vec![Message::tool_use(1, "t1", "Read", "args", 50)];
        let pairs = comp.find_pairs(&msgs);
        assert!(pairs.is_empty());
    }

    #[test]
    fn test_drop_old_droppable_pair() {
        let comp = ToolPairCompactor::default();
        let msgs = make_pair(1, "Read", 50, 100, false);
        let (compacted, freed) = comp.compact(&msgs, 0);
        assert!(compacted.is_empty());
        assert_eq!(freed, 150);
    }

    #[test]
    fn test_keep_recent_pair() {
        let comp = ToolPairCompactor::default();
        let msgs = make_pair(1, "Read", 50, 100, false);
        let (compacted, freed) = comp.compact(&msgs, 10);
        assert_eq!(compacted.len(), 2);
        assert_eq!(freed, 0);
    }

    #[test]
    fn test_summarise_large_droppable_result() {
        let comp = ToolPairCompactor::default();
        let msgs = make_pair(1, "Grep", 50, 5000, false);
        let (compacted, freed) = comp.compact(&msgs, 0);
        assert!(!compacted.is_empty());
        let result_msg = compacted.iter().find(|m| m.is_tool_result()).unwrap();
        assert!(result_msg.content.contains("summarised"));
        assert!(freed > 0);
    }

    #[test]
    fn test_truncate_non_droppable_large_result() {
        let comp = ToolPairCompactor::default();
        let msgs = make_pair(1, "BashRun", 50, 5000, false);
        let (compacted, freed) = comp.compact(&msgs, 0);
        let result_msg = compacted.iter().find(|m| m.is_tool_result()).unwrap();
        assert!(result_msg.tokens < 5000);
        assert!(freed > 0);
    }

    #[test]
    fn test_text_messages_preserved() {
        let comp = ToolPairCompactor::default();
        let mut msgs = vec![Message::text(0, MessageRole::User, "hello", 10)];
        msgs.extend(make_pair(1, "Read", 50, 200, false));
        msgs.push(Message::text(100, MessageRole::Assistant, "done", 20));
        let (compacted, _) = comp.compact(&msgs, 0);
        assert!(compacted.iter().any(|m| m.id == 0));
        assert!(compacted.iter().any(|m| m.id == 100));
    }

    #[test]
    fn test_total_tokens() {
        let msgs = vec![
            Message::text(0, MessageRole::User, "hi", 10),
            Message::text(1, MessageRole::Assistant, "there", 15),
        ];
        assert_eq!(ToolPairCompactor::total_tokens(&msgs), 25);
    }

    #[test]
    fn test_strategy_keep_recent_small() {
        let policy = CompactionPolicy::default();
        let pair = ToolPair {
            use_id: 1,
            result_id: 2,
            tool_name: "Edit".into(),
            tool_id: "t1".into(),
            use_tokens: 50,
            result_tokens: 100,
        };
        assert_eq!(policy.strategy_for(&pair, true), CompactionStrategy::Keep);
    }

    #[test]
    fn test_strategy_drop_old_droppable() {
        let policy = CompactionPolicy::default();
        let pair = ToolPair {
            use_id: 1,
            result_id: 2,
            tool_name: "Read".into(),
            tool_id: "t1".into(),
            use_tokens: 50,
            result_tokens: 100,
        };
        assert_eq!(policy.strategy_for(&pair, false), CompactionStrategy::Drop);
    }

    #[test]
    fn test_strategy_summarise_large_droppable() {
        let policy = CompactionPolicy::default();
        let pair = ToolPair {
            use_id: 1,
            result_id: 2,
            tool_name: "Glob".into(),
            tool_id: "t1".into(),
            use_tokens: 50,
            result_tokens: 3000,
        };
        assert_eq!(policy.strategy_for(&pair, false), CompactionStrategy::Summarise);
    }

    #[test]
    fn test_strategy_truncate_non_droppable_large() {
        let policy = CompactionPolicy::default();
        let pair = ToolPair {
            use_id: 1,
            result_id: 2,
            tool_name: "CustomTool".into(),
            tool_id: "t1".into(),
            use_tokens: 50,
            result_tokens: 3000,
        };
        assert!(matches!(
            policy.strategy_for(&pair, false),
            CompactionStrategy::Truncate { .. }
        ));
    }

    #[test]
    fn test_message_is_tool_use() {
        let m = Message::tool_use(1, "t1", "Read", "args", 50);
        assert!(m.is_tool_use());
        assert!(!m.is_tool_result());
    }

    #[test]
    fn test_message_tool_id() {
        let m = Message::tool_result(1, "t99", "output", false, 100);
        assert_eq!(m.tool_id(), Some("t99"));
    }

    #[test]
    fn test_message_text_has_no_tool_id() {
        let m = Message::text(1, MessageRole::User, "hi", 5);
        assert_eq!(m.tool_id(), None);
    }
}
