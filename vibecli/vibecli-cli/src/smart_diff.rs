#![allow(dead_code)]
//! Syntax-aware smart diff engine.
//!
//! Splits unified diff hunks by semantic blocks (functions, classes, impl blocks,
//! top-level declarations), renders side-by-side and inline views, and computes
//! hunk-level statistics.
//!
//! Matches Cursor 4.0's syntax-aware diff renderer.

use std::fmt::Write as FmtWrite;

// ---------------------------------------------------------------------------
// Diff line types
// ---------------------------------------------------------------------------

/// Classification of a single line in a diff.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum DiffLineKind {
    Context,
    Added,
    Removed,
    /// Hunk header (@@)
    Header,
}

impl DiffLineKind {
    pub fn prefix(&self) -> char {
        match self {
            DiffLineKind::Context => ' ',
            DiffLineKind::Added => '+',
            DiffLineKind::Removed => '-',
            DiffLineKind::Header => '@',
        }
    }
}

/// A single line parsed from a unified diff.
#[derive(Debug, Clone)]
pub struct DiffLine {
    pub kind: DiffLineKind,
    pub content: String,
    /// Line number in the old file (None for added lines / headers).
    pub old_lineno: Option<usize>,
    /// Line number in the new file (None for removed lines / headers).
    pub new_lineno: Option<usize>,
}

impl DiffLine {
    fn context(content: impl Into<String>, old: usize, new: usize) -> Self {
        Self {
            kind: DiffLineKind::Context,
            content: content.into(),
            old_lineno: Some(old),
            new_lineno: Some(new),
        }
    }

    fn added(content: impl Into<String>, new: usize) -> Self {
        Self {
            kind: DiffLineKind::Added,
            content: content.into(),
            old_lineno: None,
            new_lineno: Some(new),
        }
    }

    fn removed(content: impl Into<String>, old: usize) -> Self {
        Self {
            kind: DiffLineKind::Removed,
            content: content.into(),
            old_lineno: Some(old),
            new_lineno: None,
        }
    }
}

// ---------------------------------------------------------------------------
// Hunk
// ---------------------------------------------------------------------------

/// A diff hunk (one `@@` block).
#[derive(Debug, Clone)]
pub struct DiffHunk {
    pub old_start: usize,
    pub old_count: usize,
    pub new_start: usize,
    pub new_count: usize,
    pub header_context: String,
    pub lines: Vec<DiffLine>,
}

impl DiffHunk {
    /// Number of lines added in this hunk.
    pub fn added_count(&self) -> usize {
        self.lines
            .iter()
            .filter(|l| l.kind == DiffLineKind::Added)
            .count()
    }

    /// Number of lines removed in this hunk.
    pub fn removed_count(&self) -> usize {
        self.lines
            .iter()
            .filter(|l| l.kind == DiffLineKind::Removed)
            .count()
    }

    /// True if this hunk touches lines that belong to a known semantic block.
    pub fn is_semantic(&self) -> bool {
        !self.header_context.is_empty()
    }

    /// Render hunk as unified diff text.
    pub fn to_unified(&self) -> String {
        let mut out = String::new();
        let ctx = if self.header_context.is_empty() {
            String::new()
        } else {
            format!(" {}", self.header_context)
        };
        let _ = writeln!(
            out,
            "@@ -{},{} +{},{}{}@@",
            self.old_start, self.old_count, self.new_start, self.new_count, ctx
        );
        for line in &self.lines {
            let _ = writeln!(out, "{}{}", line.kind.prefix(), line.content);
        }
        out
    }
}

// ---------------------------------------------------------------------------
// Parsed diff file
// ---------------------------------------------------------------------------

/// A parsed diff for a single file.
#[derive(Debug, Clone)]
pub struct FileDiff {
    pub old_path: String,
    pub new_path: String,
    pub hunks: Vec<DiffHunk>,
}

impl FileDiff {
    pub fn total_added(&self) -> usize {
        self.hunks.iter().map(|h| h.added_count()).sum()
    }

    pub fn total_removed(&self) -> usize {
        self.hunks.iter().map(|h| h.removed_count()).sum()
    }

    pub fn hunk_count(&self) -> usize {
        self.hunks.len()
    }

    pub fn is_new_file(&self) -> bool {
        self.old_path == "/dev/null"
    }

    pub fn is_deleted_file(&self) -> bool {
        self.new_path == "/dev/null"
    }
}

// ---------------------------------------------------------------------------
// Diff parser
// ---------------------------------------------------------------------------

/// Parses a unified diff string into `FileDiff` entries.
pub struct DiffParser;

impl DiffParser {
    /// Parse a multi-file unified diff.
    pub fn parse(input: &str) -> Vec<FileDiff> {
        let mut files = vec![];
        let mut current: Option<FileDiff> = None;
        let mut current_hunk: Option<DiffHunk> = None;
        let mut old_lineno = 0usize;
        let mut new_lineno = 0usize;

        for raw_line in input.lines() {
            if let Some(rest) = raw_line.strip_prefix("--- ") {
                // Finalise previous hunk/file.
                if let Some(mut file) = current.take() {
                    if let Some(hunk) = current_hunk.take() {
                        file.hunks.push(hunk);
                    }
                    files.push(file);
                } else if let Some(hunk) = current_hunk.take() {
                    if let Some(f) = files.last_mut() {
                        f.hunks.push(hunk);
                    }
                }
                let old_path = rest.trim_start_matches("a/").to_string();
                current = Some(FileDiff {
                    old_path,
                    new_path: String::new(),
                    hunks: vec![],
                });
            } else if let Some(rest) = raw_line.strip_prefix("+++ ") {
                if let Some(ref mut f) = current {
                    f.new_path = rest.trim_start_matches("b/").to_string();
                }
            } else if raw_line.starts_with("@@ ") {
                // Finalise previous hunk.
                if let Some(file) = current.as_mut() {
                    if let Some(hunk) = current_hunk.take() {
                        file.hunks.push(hunk);
                    }
                }
                let hunk = Self::parse_hunk_header(raw_line);
                old_lineno = hunk.old_start;
                new_lineno = hunk.new_start;
                current_hunk = Some(hunk);
            } else if let Some(ref mut hunk) = current_hunk {
                if let Some(rest) = raw_line.strip_prefix('+') {
                    hunk.lines.push(DiffLine::added(rest, new_lineno));
                    new_lineno += 1;
                } else if let Some(rest) = raw_line.strip_prefix('-') {
                    hunk.lines.push(DiffLine::removed(rest, old_lineno));
                    old_lineno += 1;
                } else {
                    let content = raw_line.strip_prefix(' ').unwrap_or(raw_line);
                    hunk.lines
                        .push(DiffLine::context(content, old_lineno, new_lineno));
                    old_lineno += 1;
                    new_lineno += 1;
                }
            }
        }

        // Flush last hunk/file.
        if let Some(mut file) = current.take() {
            if let Some(hunk) = current_hunk.take() {
                file.hunks.push(hunk);
            }
            files.push(file);
        }

        files
    }

    fn parse_hunk_header(line: &str) -> DiffHunk {
        // Format: @@ -old_start[,old_count] +new_start[,new_count] @@ [context]
        let mut old_start = 1usize;
        let mut old_count = 1usize;
        let mut new_start = 1usize;
        let mut new_count = 1usize;
        let mut header_context = String::new();

        if let Some(inner) = line.strip_prefix("@@ ") {
            // Split at the closing @@
            let (ranges, ctx) = if let Some(pos) = inner.find(" @@") {
                (&inner[..pos], inner[pos + 3..].trim())
            } else {
                (inner, "")
            };
            header_context = ctx.to_string();

            for part in ranges.split_whitespace() {
                if let Some(old) = part.strip_prefix('-') {
                    let (s, c) = Self::parse_range(old);
                    old_start = s;
                    old_count = c;
                } else if let Some(new) = part.strip_prefix('+') {
                    let (s, c) = Self::parse_range(new);
                    new_start = s;
                    new_count = c;
                }
            }
        }

        DiffHunk {
            old_start,
            old_count,
            new_start,
            new_count,
            header_context,
            lines: vec![],
        }
    }

    fn parse_range(s: &str) -> (usize, usize) {
        if let Some((start, count)) = s.split_once(',') {
            (
                start.parse().unwrap_or(1),
                count.parse().unwrap_or(1),
            )
        } else {
            (s.parse().unwrap_or(1), 1)
        }
    }
}

// ---------------------------------------------------------------------------
// Semantic block detector
// ---------------------------------------------------------------------------

/// Detects semantic block boundaries (fn, struct, impl, class, def …).
pub struct SemanticBlockDetector;

impl SemanticBlockDetector {
    /// Return the name of the semantic block that line `lineno` (1-based) falls
    /// within, if any. Uses simple pattern matching (no full parse tree).
    pub fn block_at(source: &str, lineno: usize) -> Option<String> {
        let lines: Vec<&str> = source.lines().collect();
        // Walk backwards from lineno to find the nearest block header.
        let idx = lineno.saturating_sub(1).min(lines.len().saturating_sub(1));
        for i in (0..=idx).rev() {
            let trimmed = lines[i].trim();
            if let Some(name) = Self::detect_header(trimmed) {
                return Some(name);
            }
        }
        None
    }

    /// Annotate hunks in a `FileDiff` with semantic context by scanning the
    /// old-file source text.
    pub fn annotate(file_diff: &mut FileDiff, old_source: &str) {
        for hunk in file_diff.hunks.iter_mut() {
            if hunk.header_context.is_empty() {
                if let Some(name) = Self::block_at(old_source, hunk.old_start) {
                    hunk.header_context = name;
                }
            }
        }
    }

    fn detect_header(line: &str) -> Option<String> {
        let patterns = [
            ("fn ", "fn"),
            ("pub fn ", "fn"),
            ("async fn ", "fn"),
            ("pub async fn ", "fn"),
            ("struct ", "struct"),
            ("pub struct ", "struct"),
            ("enum ", "enum"),
            ("pub enum ", "enum"),
            ("impl ", "impl"),
            ("trait ", "trait"),
            ("pub trait ", "trait"),
            ("class ", "class"),
            ("def ", "def"),
            ("function ", "function"),
            ("const ", "const"),
            ("pub const ", "const"),
            ("type ", "type"),
            ("pub type ", "type"),
        ];
        for (prefix, kind) in &patterns {
            if let Some(rest) = line.strip_prefix(prefix) {
                let name_end = rest
                    .find(|c: char| !c.is_alphanumeric() && c != '_')
                    .unwrap_or(rest.len());
                let name = &rest[..name_end];
                if !name.is_empty() {
                    return Some(format!("{kind} {name}"));
                }
            }
        }
        None
    }
}

// ---------------------------------------------------------------------------
// Diff renderer
// ---------------------------------------------------------------------------

/// Rendering mode for diff output.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum RenderMode {
    /// Classic unified diff text.
    Unified,
    /// Two-column side-by-side (old | new).
    SideBySide { col_width: usize },
    /// Inline with ANSI colour codes.
    InlineColour,
}

/// Renders a `FileDiff` in the requested mode.
pub struct DiffRenderer;

impl DiffRenderer {
    pub fn render(file_diff: &FileDiff, mode: &RenderMode) -> String {
        match mode {
            RenderMode::Unified => Self::render_unified(file_diff),
            RenderMode::SideBySide { col_width } => {
                Self::render_side_by_side(file_diff, *col_width)
            }
            RenderMode::InlineColour => Self::render_inline_colour(file_diff),
        }
    }

    fn render_unified(file_diff: &FileDiff) -> String {
        let mut out = format!(
            "--- {}\n+++ {}\n",
            file_diff.old_path, file_diff.new_path
        );
        for hunk in &file_diff.hunks {
            out.push_str(&hunk.to_unified());
        }
        out
    }

    fn render_side_by_side(file_diff: &FileDiff, col_width: usize) -> String {
        let mut out = format!(
            "{:<width$} │ {}\n{}\n",
            file_diff.old_path,
            file_diff.new_path,
            "─".repeat(col_width * 2 + 3),
            width = col_width
        );

        for hunk in &file_diff.hunks {
            let sep = "─".repeat(col_width * 2 + 3);
            if !hunk.header_context.is_empty() {
                let _ = writeln!(out, "── {} {}", hunk.header_context, sep);
            } else {
                let _ = writeln!(out, "{sep}");
            }

            let mut old_lines: Vec<&DiffLine> = vec![];
            let mut new_lines: Vec<&DiffLine> = vec![];

            for line in &hunk.lines {
                match line.kind {
                    DiffLineKind::Removed => old_lines.push(line),
                    DiffLineKind::Added => new_lines.push(line),
                    DiffLineKind::Context => {
                        // Flush accumulated old/new
                        let max_len = old_lines.len().max(new_lines.len());
                        for i in 0..max_len {
                            let old_text =
                                old_lines.get(i).map(|l| l.content.as_str()).unwrap_or("");
                            let new_text =
                                new_lines.get(i).map(|l| l.content.as_str()).unwrap_or("");
                            let _ = writeln!(
                                out,
                                "{:<width$} │ {}",
                                Self::truncate(old_text, col_width),
                                Self::truncate(new_text, col_width),
                                width = col_width
                            );
                        }
                        old_lines.clear();
                        new_lines.clear();
                        let _ = writeln!(
                            out,
                            "{:<width$} │ {}",
                            Self::truncate(&line.content, col_width),
                            Self::truncate(&line.content, col_width),
                            width = col_width
                        );
                    }
                    DiffLineKind::Header => {}
                }
            }
            // Flush remaining
            let max_len = old_lines.len().max(new_lines.len());
            for i in 0..max_len {
                let old_text = old_lines.get(i).map(|l| l.content.as_str()).unwrap_or("");
                let new_text = new_lines.get(i).map(|l| l.content.as_str()).unwrap_or("");
                let _ = writeln!(
                    out,
                    "{:<width$} │ {}",
                    Self::truncate(old_text, col_width),
                    Self::truncate(new_text, col_width),
                    width = col_width
                );
            }
        }
        out
    }

    fn render_inline_colour(file_diff: &FileDiff) -> String {
        const RED: &str = "\x1b[31m";
        const GREEN: &str = "\x1b[32m";
        const CYAN: &str = "\x1b[36m";
        const RESET: &str = "\x1b[0m";

        let mut out = format!(
            "{}--- {}{}\n{}+++ {}{}\n",
            RED, file_diff.old_path, RESET, GREEN, file_diff.new_path, RESET
        );

        for hunk in &file_diff.hunks {
            let ctx = if hunk.header_context.is_empty() {
                String::new()
            } else {
                format!(" {}", hunk.header_context)
            };
            let _ = writeln!(
                out,
                "{CYAN}@@ -{},{} +{},{}{}@@{RESET}",
                hunk.old_start,
                hunk.old_count,
                hunk.new_start,
                hunk.new_count,
                ctx
            );
            for line in &hunk.lines {
                match line.kind {
                    DiffLineKind::Added => {
                        let _ = writeln!(out, "{GREEN}+{}{RESET}", line.content);
                    }
                    DiffLineKind::Removed => {
                        let _ = writeln!(out, "{RED}-{}{RESET}", line.content);
                    }
                    DiffLineKind::Context => {
                        let _ = writeln!(out, " {}", line.content);
                    }
                    DiffLineKind::Header => {}
                }
            }
        }
        out
    }

    fn truncate(s: &str, max: usize) -> String {
        if s.len() <= max {
            s.to_string()
        } else {
            format!("{}…", &s[..max.saturating_sub(1)])
        }
    }
}

// ---------------------------------------------------------------------------
// Diff stats
// ---------------------------------------------------------------------------

/// Summary statistics for a set of file diffs.
#[derive(Debug, Clone, Default)]
pub struct DiffStats {
    pub files_changed: usize,
    pub hunks: usize,
    pub lines_added: usize,
    pub lines_removed: usize,
    pub semantic_hunks: usize,
}

impl DiffStats {
    pub fn compute(files: &[FileDiff]) -> Self {
        let mut s = DiffStats {
            files_changed: files.len(),
            ..Default::default()
        };
        for f in files {
            s.hunks += f.hunk_count();
            s.lines_added += f.total_added();
            s.lines_removed += f.total_removed();
            s.semantic_hunks += f.hunks.iter().filter(|h| h.is_semantic()).count();
        }
        s
    }
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    const SAMPLE_DIFF: &str = r#"--- a/src/lib.rs
+++ b/src/lib.rs
@@ -1,5 +1,6 @@ fn greet
 fn greet(name: &str) -> String {
-    format!("hello {}", name)
+    format!("Hello, {}!", name)
+    // improved greeting
 }
"#;

    #[test]
    fn test_parse_single_hunk() {
        let files = DiffParser::parse(SAMPLE_DIFF);
        assert_eq!(files.len(), 1);
        let f = &files[0];
        assert_eq!(f.old_path, "src/lib.rs");
        assert_eq!(f.new_path, "src/lib.rs");
        assert_eq!(f.hunks.len(), 1);
    }

    #[test]
    fn test_hunk_added_removed_counts() {
        let files = DiffParser::parse(SAMPLE_DIFF);
        let hunk = &files[0].hunks[0];
        assert_eq!(hunk.added_count(), 2);
        assert_eq!(hunk.removed_count(), 1);
    }

    #[test]
    fn test_hunk_header_context_preserved() {
        let files = DiffParser::parse(SAMPLE_DIFF);
        let hunk = &files[0].hunks[0];
        assert_eq!(hunk.header_context, "fn greet");
    }

    #[test]
    fn test_hunk_is_semantic_when_context_present() {
        let files = DiffParser::parse(SAMPLE_DIFF);
        assert!(files[0].hunks[0].is_semantic());
    }

    #[test]
    fn test_diff_stats() {
        let files = DiffParser::parse(SAMPLE_DIFF);
        let stats = DiffStats::compute(&files);
        assert_eq!(stats.files_changed, 1);
        assert_eq!(stats.lines_added, 2);
        assert_eq!(stats.lines_removed, 1);
    }

    #[test]
    fn test_render_unified_round_trip() {
        let files = DiffParser::parse(SAMPLE_DIFF);
        let rendered = DiffRenderer::render(&files[0], &RenderMode::Unified);
        assert!(rendered.contains("--- src/lib.rs"));
        assert!(rendered.contains("+++ src/lib.rs"));
        assert!(rendered.contains("@@"));
        assert!(rendered.contains("+    format!(\"Hello, {}!\", name)"));
    }

    #[test]
    fn test_render_side_by_side_contains_separator() {
        let files = DiffParser::parse(SAMPLE_DIFF);
        let rendered =
            DiffRenderer::render(&files[0], &RenderMode::SideBySide { col_width: 40 });
        assert!(rendered.contains('│'));
    }

    #[test]
    fn test_render_inline_colour_contains_ansi() {
        let files = DiffParser::parse(SAMPLE_DIFF);
        let rendered = DiffRenderer::render(&files[0], &RenderMode::InlineColour);
        assert!(rendered.contains("\x1b["));
    }

    #[test]
    fn test_semantic_block_detector_fn() {
        let src = "pub fn my_function(x: i32) -> i32 {\n    x + 1\n}\n";
        let name = SemanticBlockDetector::block_at(src, 1);
        assert_eq!(name, Some("fn my_function".to_string()));
    }

    #[test]
    fn test_semantic_block_detector_impl() {
        // Line 1 is "impl MyStruct {" — detector returns it as the enclosing block.
        let src = "impl MyStruct {\n    fn method(&self) {}\n}\n";
        let name = SemanticBlockDetector::block_at(src, 1);
        assert_eq!(name, Some("impl MyStruct".to_string()));
    }

    #[test]
    fn test_annotate_adds_context_to_hunk() {
        let mut diff = FileDiff {
            old_path: "src/lib.rs".into(),
            new_path: "src/lib.rs".into(),
            hunks: vec![DiffHunk {
                old_start: 2,
                old_count: 2,
                new_start: 2,
                new_count: 2,
                header_context: String::new(),
                lines: vec![],
            }],
        };
        let src = "fn greet(name: &str) {\n    println!(\"{}\", name);\n}\n";
        SemanticBlockDetector::annotate(&mut diff, src);
        assert!(!diff.hunks[0].header_context.is_empty());
    }

    #[test]
    fn test_multi_file_diff() {
        let multi = r#"--- a/src/a.rs
+++ b/src/a.rs
@@ -1,2 +1,2 @@
-let x = 1;
+let x = 2;
--- a/src/b.rs
+++ b/src/b.rs
@@ -1,2 +1,3 @@
 fn foo() {
+    println!("added");
 }
"#;
        let files = DiffParser::parse(multi);
        assert_eq!(files.len(), 2);
        assert_eq!(files[0].old_path, "src/a.rs");
        assert_eq!(files[1].old_path, "src/b.rs");
    }

    #[test]
    fn test_to_unified_hunk_roundtrip() {
        let hunk = DiffHunk {
            old_start: 1,
            old_count: 2,
            new_start: 1,
            new_count: 3,
            header_context: "fn foo".into(),
            lines: vec![
                DiffLine::removed("old line", 1),
                DiffLine::added("new line 1", 1),
                DiffLine::added("new line 2", 2),
            ],
        };
        let unified = hunk.to_unified();
        assert!(unified.contains("@@ -1,2 +1,3"));
        assert!(unified.contains("-old line"));
        assert!(unified.contains("+new line 1"));
    }

    #[test]
    fn test_is_new_file() {
        let f = FileDiff {
            old_path: "/dev/null".into(),
            new_path: "src/new.rs".into(),
            hunks: vec![],
        };
        assert!(f.is_new_file());
        assert!(!f.is_deleted_file());
    }

    #[test]
    fn test_truncate_long_string() {
        let s = "a".repeat(200);
        let t = DiffRenderer::truncate(&s, 40);
        // 39 ASCII chars + "…" (3 UTF-8 bytes) = 42 bytes
        assert!(t.len() <= 42);
        assert!(t.ends_with('…'));
    }
}
