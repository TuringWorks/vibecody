#![allow(dead_code)]
//! Streaming patch applicator — applies unified diffs as a stream of hunks
//! with per-hunk rollback support.
//!
//! Matches Claude Code 1.x and Devin 2.0's streaming patch application.

use std::collections::VecDeque;

// ---------------------------------------------------------------------------
// Patch types (mirror smart_diff structures, kept self-contained)
// ---------------------------------------------------------------------------

/// A single line in a patch hunk.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum PatchLine {
    Context(String),
    Added(String),
    Removed(String),
}

impl PatchLine {
    pub fn content(&self) -> &str {
        match self {
            PatchLine::Context(s) | PatchLine::Added(s) | PatchLine::Removed(s) => s,
        }
    }
}

/// A single patch hunk with its header offset information.
#[derive(Debug, Clone)]
pub struct PatchHunk {
    pub old_start: usize,
    pub new_start: usize,
    pub lines: Vec<PatchLine>,
    pub header_context: String,
}

impl PatchHunk {
    pub fn added_count(&self) -> usize {
        self.lines
            .iter()
            .filter(|l| matches!(l, PatchLine::Added(_)))
            .count()
    }

    pub fn removed_count(&self) -> usize {
        self.lines
            .iter()
            .filter(|l| matches!(l, PatchLine::Removed(_)))
            .count()
    }
}

// ---------------------------------------------------------------------------
// Patch result
// ---------------------------------------------------------------------------

/// Result of applying a single hunk.
#[derive(Debug, Clone, PartialEq)]
pub enum HunkResult {
    Applied,
    Skipped { reason: String },
    Conflict { expected: String, got: String },
}

/// Result of a streaming patch session.
#[derive(Debug, Clone)]
pub struct PatchSummary {
    pub hunks_applied: usize,
    pub hunks_skipped: usize,
    pub hunks_conflicted: usize,
    pub lines_added: usize,
    pub lines_removed: usize,
}

impl PatchSummary {
    pub fn success(&self) -> bool {
        self.hunks_conflicted == 0
    }
}

// ---------------------------------------------------------------------------
// Snapshot for rollback
// ---------------------------------------------------------------------------

#[derive(Debug, Clone)]
struct FileSnapshot {
    content: Vec<String>,
}

// ---------------------------------------------------------------------------
// Streaming patch applicator
// ---------------------------------------------------------------------------

/// Applies patch hunks to an in-memory file representation, one hunk at a time.
/// Supports rollback to the last snapshot.
pub struct StreamPatcher {
    /// Current file content as lines.
    lines: Vec<String>,
    /// Snapshot taken before applying any hunk (for rollback).
    snapshots: VecDeque<FileSnapshot>,
    /// Max snapshots to retain.
    snapshot_cap: usize,
    /// Hunk results recorded during this session.
    results: Vec<HunkResult>,
    /// Running stats.
    lines_added: usize,
    lines_removed: usize,
}

impl StreamPatcher {
    /// Create a patcher for the given file content.
    pub fn new(content: &str) -> Self {
        Self {
            lines: content.lines().map(String::from).collect(),
            snapshots: VecDeque::new(),
            snapshot_cap: 20,
            results: vec![],
            lines_added: 0,
            lines_removed: 0,
        }
    }

    /// Apply a single hunk. Returns the `HunkResult`.
    pub fn apply_hunk(&mut self, hunk: &PatchHunk) -> HunkResult {
        // Take a snapshot before each hunk.
        self.push_snapshot();

        // Validate context lines at `old_start`.
        if let Some(conflict) = self.validate_context(hunk) {
            let result = HunkResult::Conflict {
                expected: conflict.0,
                got: conflict.1,
            };
            self.results.push(result.clone());
            return result;
        }

        // Apply the hunk by rebuilding lines around old_start.
        let insert_idx = hunk.old_start.saturating_sub(1).min(self.lines.len());
        let mut new_lines: Vec<String> = Vec::with_capacity(self.lines.len() + 16);
        let mut source_idx = 0;

        // Copy lines before the hunk.
        for _ in 0..insert_idx {
            if source_idx < self.lines.len() {
                new_lines.push(self.lines[source_idx].clone());
                source_idx += 1;
            }
        }

        // Apply the hunk lines.
        for patch_line in &hunk.lines {
            match patch_line {
                PatchLine::Context(s) => {
                    new_lines.push(s.clone());
                    source_idx += 1;
                }
                PatchLine::Added(s) => {
                    new_lines.push(s.clone());
                    self.lines_added += 1;
                }
                PatchLine::Removed(_) => {
                    // Skip — consume from source.
                    source_idx += 1;
                    self.lines_removed += 1;
                }
            }
        }

        // Append remaining lines.
        while source_idx < self.lines.len() {
            new_lines.push(self.lines[source_idx].clone());
            source_idx += 1;
        }

        self.lines = new_lines;
        let result = HunkResult::Applied;
        self.results.push(result.clone());
        result
    }

    /// Roll back the last applied hunk.
    pub fn rollback_last(&mut self) -> bool {
        if let Some(snap) = self.snapshots.pop_back() {
            self.lines = snap.content;
            if let Some(last) = self.results.last() {
                if *last == HunkResult::Applied {
                    self.results.pop();
                }
            }
            true
        } else {
            false
        }
    }

    /// Roll back all applied hunks to the original content.
    pub fn rollback_all(&mut self) -> bool {
        if let Some(first) = self.snapshots.pop_front() {
            self.lines = first.content;
            self.snapshots.clear();
            self.results.clear();
            self.lines_added = 0;
            self.lines_removed = 0;
            true
        } else {
            false
        }
    }

    /// Return current file content as a string.
    pub fn content(&self) -> String {
        self.lines.join("\n")
    }

    /// Return current content as lines.
    pub fn lines(&self) -> &[String] {
        &self.lines
    }

    /// Summary of applied hunks.
    pub fn summary(&self) -> PatchSummary {
        let mut applied = 0;
        let mut skipped = 0;
        let mut conflicted = 0;
        for r in &self.results {
            match r {
                HunkResult::Applied => applied += 1,
                HunkResult::Skipped { .. } => skipped += 1,
                HunkResult::Conflict { .. } => conflicted += 1,
            }
        }
        PatchSummary {
            hunks_applied: applied,
            hunks_skipped: skipped,
            hunks_conflicted: conflicted,
            lines_added: self.lines_added,
            lines_removed: self.lines_removed,
        }
    }

    /// Preview: compute what the file would look like after applying the hunk,
    /// without modifying state.
    pub fn preview_hunk(&self, hunk: &PatchHunk) -> Vec<String> {
        let preview = self.lines.clone();
        let insert_idx = hunk.old_start.saturating_sub(1).min(preview.len());
        let mut new_lines = Vec::with_capacity(preview.len() + 16);
        let mut source_idx = 0;
        for _ in 0..insert_idx {
            if source_idx < preview.len() {
                new_lines.push(preview[source_idx].clone());
                source_idx += 1;
            }
        }
        for patch_line in &hunk.lines {
            match patch_line {
                PatchLine::Context(s) | PatchLine::Added(s) => {
                    new_lines.push(s.clone());
                    if matches!(patch_line, PatchLine::Context(_)) {
                        source_idx += 1;
                    }
                }
                PatchLine::Removed(_) => {
                    source_idx += 1;
                }
            }
        }
        while source_idx < preview.len() {
            new_lines.push(preview[source_idx].clone());
            source_idx += 1;
        }
        new_lines
    }

    // Snapshot helpers.
    fn push_snapshot(&mut self) {
        if self.snapshots.len() >= self.snapshot_cap {
            self.snapshots.pop_front();
        }
        self.snapshots.push_back(FileSnapshot {
            content: self.lines.clone(),
        });
    }

    // Validate context lines; returns Some((expected, got)) on conflict.
    fn validate_context(&self, hunk: &PatchHunk) -> Option<(String, String)> {
        let start = hunk.old_start.saturating_sub(1);
        let mut file_idx = start;
        for patch_line in &hunk.lines {
            match patch_line {
                PatchLine::Context(expected) | PatchLine::Removed(expected) => {
                    match self.lines.get(file_idx) {
                        Some(actual) if actual == expected => {
                            file_idx += 1;
                        }
                        Some(actual) => {
                            return Some((expected.clone(), actual.clone()));
                        }
                        None => {
                            return Some((
                                expected.clone(),
                                "<end of file>".to_string(),
                            ));
                        }
                    }
                }
                PatchLine::Added(_) => {}
            }
        }
        None
    }
}

// ---------------------------------------------------------------------------
// Simple diff-to-hunk parser (for test fixtures)
// ---------------------------------------------------------------------------

/// Parse a minimal unified diff string into `PatchHunk` list.
pub fn parse_patch(input: &str) -> Vec<PatchHunk> {
    let mut hunks = vec![];
    let mut current: Option<PatchHunk> = None;

    for line in input.lines() {
        if line.starts_with("@@ ") {
            if let Some(h) = current.take() {
                hunks.push(h);
            }
            let (old_start, new_start, ctx) = parse_hunk_header(line);
            current = Some(PatchHunk {
                old_start,
                new_start,
                lines: vec![],
                header_context: ctx,
            });
        } else if let Some(ref mut h) = current {
            if let Some(rest) = line.strip_prefix('+') {
                h.lines.push(PatchLine::Added(rest.to_string()));
            } else if let Some(rest) = line.strip_prefix('-') {
                h.lines.push(PatchLine::Removed(rest.to_string()));
            } else if let Some(rest) = line.strip_prefix(' ') {
                h.lines.push(PatchLine::Context(rest.to_string()));
            }
        }
    }
    if let Some(h) = current.take() {
        hunks.push(h);
    }
    hunks
}

fn parse_hunk_header(line: &str) -> (usize, usize, String) {
    let mut old_start = 1usize;
    let mut new_start = 1usize;
    let mut ctx = String::new();

    if let Some(inner) = line.strip_prefix("@@ ") {
        let (ranges, context) = if let Some(pos) = inner.find(" @@") {
            (&inner[..pos], inner[pos + 3..].trim())
        } else {
            (inner, "")
        };
        ctx = context.to_string();
        for part in ranges.split_whitespace() {
            if let Some(old) = part.strip_prefix('-') {
                old_start = old.split(',').next().and_then(|s| s.parse().ok()).unwrap_or(1);
            } else if let Some(new) = part.strip_prefix('+') {
                new_start = new.split(',').next().and_then(|s| s.parse().ok()).unwrap_or(1);
            }
        }
    }
    (old_start, new_start, ctx)
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    const ORIGINAL: &str = "fn greet(name: &str) -> String {\n    format!(\"hello {}\", name)\n}";

    const PATCH: &str = r#"@@ -1,3 +1,3 @@
 fn greet(name: &str) -> String {
-    format!("hello {}", name)
+    format!("Hello, {}!", name)
 }"#;

    #[test]
    fn test_parse_patch_single_hunk() {
        let hunks = parse_patch(PATCH);
        assert_eq!(hunks.len(), 1);
        assert_eq!(hunks[0].added_count(), 1);
        assert_eq!(hunks[0].removed_count(), 1);
    }

    #[test]
    fn test_apply_hunk_changes_content() {
        let mut patcher = StreamPatcher::new(ORIGINAL);
        let hunks = parse_patch(PATCH);
        let result = patcher.apply_hunk(&hunks[0]);
        assert_eq!(result, HunkResult::Applied);
        let content = patcher.content();
        assert!(content.contains("Hello,"));
        assert!(!content.contains("\"hello"));
    }

    #[test]
    fn test_rollback_restores_original() {
        let mut patcher = StreamPatcher::new(ORIGINAL);
        let hunks = parse_patch(PATCH);
        patcher.apply_hunk(&hunks[0]);
        patcher.rollback_last();
        assert_eq!(patcher.content(), ORIGINAL);
    }

    #[test]
    fn test_rollback_all() {
        let mut patcher = StreamPatcher::new(ORIGINAL);
        let hunks = parse_patch(PATCH);
        patcher.apply_hunk(&hunks[0]);
        patcher.rollback_all();
        assert_eq!(patcher.content(), ORIGINAL);
    }

    #[test]
    fn test_summary_after_apply() {
        let mut patcher = StreamPatcher::new(ORIGINAL);
        let hunks = parse_patch(PATCH);
        patcher.apply_hunk(&hunks[0]);
        let s = patcher.summary();
        assert_eq!(s.hunks_applied, 1);
        assert_eq!(s.hunks_conflicted, 0);
        assert_eq!(s.lines_added, 1);
        assert_eq!(s.lines_removed, 1);
        assert!(s.success());
    }

    #[test]
    fn test_conflict_on_wrong_context() {
        let wrong_original = "fn greet(name: &str) {\n    println!(\"hey\");\n}";
        let mut patcher = StreamPatcher::new(wrong_original);
        let hunks = parse_patch(PATCH);
        let result = patcher.apply_hunk(&hunks[0]);
        // Context lines don't match — conflict.
        assert!(matches!(result, HunkResult::Conflict { .. }));
    }

    #[test]
    fn test_preview_does_not_modify_state() {
        let patcher = StreamPatcher::new(ORIGINAL);
        let hunks = parse_patch(PATCH);
        let preview = patcher.preview_hunk(&hunks[0]);
        // Original should be unchanged
        assert_eq!(patcher.content(), ORIGINAL);
        // Preview should have the changed line
        assert!(preview.iter().any(|l| l.contains("Hello,")));
    }

    #[test]
    fn test_multiple_hunks_applied_sequentially() {
        let src = "line1\nline2\nline3\nline4\nline5\n";
        let patch_str = "@@ -1,2 +1,2 @@\n-line1\n+LINE1\n line2\n@@ -4,2 +4,2 @@\n line4\n-line5\n+LINE5";
        let mut patcher = StreamPatcher::new(src.trim_end_matches('\n'));
        let hunks = parse_patch(patch_str);
        assert_eq!(hunks.len(), 2);
        for h in &hunks {
            patcher.apply_hunk(h);
        }
        let s = patcher.summary();
        assert_eq!(s.hunks_applied, 2);
    }

    #[test]
    fn test_add_only_hunk() {
        let src = "fn foo() {}";
        let patch_str = "@@ -1,1 +1,2 @@\n fn foo() {}\n+// new line";
        let mut patcher = StreamPatcher::new(src);
        let hunks = parse_patch(patch_str);
        patcher.apply_hunk(&hunks[0]);
        assert!(patcher.content().contains("// new line"));
    }

    #[test]
    fn test_rollback_when_nothing_to_rollback() {
        let mut patcher = StreamPatcher::new(ORIGINAL);
        assert!(!patcher.rollback_last());
    }

    #[test]
    fn test_hunk_added_removed_counts() {
        let h = PatchHunk {
            old_start: 1,
            new_start: 1,
            header_context: String::new(),
            lines: vec![
                PatchLine::Removed("old".into()),
                PatchLine::Added("new1".into()),
                PatchLine::Added("new2".into()),
                PatchLine::Context("ctx".into()),
            ],
        };
        assert_eq!(h.removed_count(), 1);
        assert_eq!(h.added_count(), 2);
    }
}
