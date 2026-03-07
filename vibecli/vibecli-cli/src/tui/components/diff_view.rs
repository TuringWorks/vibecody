//! Enhanced diff view component for the VibeCLI TUI.
//!
//! Supports unified and side-by-side view modes, syntax-colored diff lines,
//! line-number gutters, and scroll navigation.
#![allow(dead_code)]

use ratatui::{
    style::{Color, Modifier, Style},
    text::{Line, Span},
};
use vibe_core::diff::{DiffEngine, DiffHunk};

// ── View mode ────────────────────────────────────────────────────────────────

/// Controls how the diff is displayed.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DiffViewMode {
    /// Standard unified diff (single column, +/- prefixes).
    Unified,
    /// Old and new content rendered side-by-side.
    SideBySide,
}

impl DiffViewMode {
    /// Return the other mode (toggle helper).
    pub fn toggled(self) -> Self {
        match self {
            DiffViewMode::Unified    => DiffViewMode::SideBySide,
            DiffViewMode::SideBySide => DiffViewMode::Unified,
        }
    }

    /// Human-readable label for the status bar.
    pub fn label(self) -> &'static str {
        match self {
            DiffViewMode::Unified    => "Unified",
            DiffViewMode::SideBySide => "Side-by-Side",
        }
    }
}

// ── Parsed diff line ─────────────────────────────────────────────────────────

/// Classification of a single line inside a diff.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DiffLineKind {
    Added,
    Removed,
    Context,
    HunkHeader,
    FileHeader,
}

/// A diff line with its classification and optional line numbers.
#[derive(Debug, Clone)]
pub struct DiffLine {
    pub kind: DiffLineKind,
    pub content: String,
    /// Line number in the old file (None for added lines / headers).
    pub old_lineno: Option<usize>,
    /// Line number in the new file (None for removed lines / headers).
    pub new_lineno: Option<usize>,
}

// ── Component ────────────────────────────────────────────────────────────────

pub struct DiffViewComponent {
    pub hunks: Vec<DiffHunk>,
    pub raw_lines: Vec<String>,
    pub scroll: u16,
    pub view_mode: DiffViewMode,
    /// Parsed representation of the current diff (rebuilt on set_*).
    parsed_lines: Vec<DiffLine>,
}

impl DiffViewComponent {
    pub fn new() -> Self {
        Self {
            hunks: Vec::new(),
            raw_lines: Vec::new(),
            scroll: 0,
            view_mode: DiffViewMode::Unified,
            parsed_lines: Vec::new(),
        }
    }

    pub fn set_diff(&mut self, original: &str, modified: &str) {
        self.hunks = DiffEngine::generate_diff(original, modified);
        self.raw_lines.clear();
        self.scroll = 0;
        self.rebuild_parsed_from_hunks();
    }

    pub fn set_raw_diff(&mut self, diff: &str) {
        self.raw_lines = diff.lines().map(|s| s.to_string()).collect();
        self.hunks.clear();
        self.scroll = 0;
        self.rebuild_parsed_from_raw();
    }

    #[allow(dead_code)]
    pub fn clear(&mut self) {
        self.hunks.clear();
        self.raw_lines.clear();
        self.parsed_lines.clear();
        self.scroll = 0;
    }

    pub fn scroll_up(&mut self) {
        self.scroll = self.scroll.saturating_sub(1);
    }

    pub fn scroll_down(&mut self) {
        let max = (self.parsed_lines.len() as u16).saturating_sub(1);
        self.scroll = self.scroll.saturating_add(1).min(max);
    }

    /// Switch between Unified and SideBySide.
    pub fn toggle_view_mode(&mut self) {
        self.view_mode = self.view_mode.toggled();
    }

    /// Return the current view mode.
    pub fn view_mode(&self) -> DiffViewMode {
        self.view_mode
    }

    /// Read-only access to parsed lines.
    pub fn parsed_lines(&self) -> &[DiffLine] {
        &self.parsed_lines
    }

    // ── Rendering ────────────────────────────────────────────────────────────

    /// Produce styled Ratatui `Line`s for the current diff state and view mode.
    pub fn render_lines(&self) -> Vec<Line<'_>> {
        match self.view_mode {
            DiffViewMode::Unified    => self.render_unified(),
            DiffViewMode::SideBySide => self.render_side_by_side(),
        }
    }

    fn render_unified(&self) -> Vec<Line<'_>> {
        self.parsed_lines
            .iter()
            .map(|dl| {
                let gutter = Self::format_gutter(dl.old_lineno, dl.new_lineno);
                let gutter_style = Style::default().fg(Color::DarkGray);
                let content_style = Self::style_for_kind(dl.kind);

                Line::from(vec![
                    Span::styled(gutter, gutter_style),
                    Span::styled(" ", gutter_style),
                    Span::styled(dl.content.as_str(), content_style),
                ])
            })
            .collect()
    }

    fn render_side_by_side(&self) -> Vec<Line<'_>> {
        // Pair up removed/added lines; context lines appear on both sides.
        let mut out: Vec<Line<'_>> = Vec::new();
        let mut i = 0;
        let lines = &self.parsed_lines;

        while i < lines.len() {
            match lines[i].kind {
                DiffLineKind::HunkHeader | DiffLineKind::FileHeader => {
                    let style = Self::style_for_kind(lines[i].kind);
                    out.push(Line::from(Span::styled(lines[i].content.as_str(), style)));
                    i += 1;
                }
                DiffLineKind::Context => {
                    let gutter_l = Self::format_lineno(lines[i].old_lineno);
                    let gutter_r = Self::format_lineno(lines[i].new_lineno);
                    let gs = Style::default().fg(Color::DarkGray);
                    let cs = Self::style_for_kind(DiffLineKind::Context);
                    out.push(Line::from(vec![
                        Span::styled(gutter_l, gs),
                        Span::styled(" ", gs),
                        Span::styled(lines[i].content.as_str(), cs),
                        Span::styled("  |  ", gs),
                        Span::styled(gutter_r, gs),
                        Span::styled(" ", gs),
                        Span::styled(lines[i].content.as_str(), cs),
                    ]));
                    i += 1;
                }
                DiffLineKind::Removed => {
                    // Collect consecutive removed lines, then pair with subsequent added lines.
                    let start = i;
                    while i < lines.len() && lines[i].kind == DiffLineKind::Removed {
                        i += 1;
                    }
                    let removed = &lines[start..i];

                    let add_start = i;
                    while i < lines.len() && lines[i].kind == DiffLineKind::Added {
                        i += 1;
                    }
                    let added = &lines[add_start..i];

                    let pairs = removed.len().max(added.len());
                    let gs = Style::default().fg(Color::DarkGray);
                    for p in 0..pairs {
                        let left = removed.get(p);
                        let right = added.get(p);
                        let mut spans: Vec<Span<'_>> = Vec::new();

                        // Left (old) side
                        if let Some(dl) = left {
                            spans.push(Span::styled(Self::format_lineno(dl.old_lineno), gs));
                            spans.push(Span::styled(" ", gs));
                            spans.push(Span::styled(dl.content.as_str(), Self::style_for_kind(DiffLineKind::Removed)));
                        } else {
                            spans.push(Span::styled("     ", gs));
                        }

                        spans.push(Span::styled("  |  ", gs));

                        // Right (new) side
                        if let Some(dl) = right {
                            spans.push(Span::styled(Self::format_lineno(dl.new_lineno), gs));
                            spans.push(Span::styled(" ", gs));
                            spans.push(Span::styled(dl.content.as_str(), Self::style_for_kind(DiffLineKind::Added)));
                        } else {
                            spans.push(Span::styled("     ", gs));
                        }

                        out.push(Line::from(spans));
                    }
                }
                DiffLineKind::Added => {
                    // Added lines without a preceding removed block — show on the right only.
                    let gs = Style::default().fg(Color::DarkGray);
                    out.push(Line::from(vec![
                        Span::styled("     ", gs),
                        Span::styled("  |  ", gs),
                        Span::styled(Self::format_lineno(lines[i].new_lineno), gs),
                        Span::styled(" ", gs),
                        Span::styled(lines[i].content.as_str(), Self::style_for_kind(DiffLineKind::Added)),
                    ]));
                    i += 1;
                }
            }
        }

        out
    }

    // ── Style helpers ────────────────────────────────────────────────────────

    /// Return the `Style` for a given diff line kind.
    pub fn style_for_kind(kind: DiffLineKind) -> Style {
        match kind {
            DiffLineKind::Added      => Style::default().fg(Color::Green).bg(Color::Rgb(0, 40, 0)),
            DiffLineKind::Removed    => Style::default().fg(Color::Red).bg(Color::Rgb(40, 0, 0)),
            DiffLineKind::HunkHeader => Style::default().fg(Color::Cyan).add_modifier(Modifier::BOLD),
            DiffLineKind::FileHeader => Style::default().fg(Color::Yellow).add_modifier(Modifier::BOLD),
            DiffLineKind::Context    => Style::default().fg(Color::DarkGray),
        }
    }

    /// Classify a raw diff text line.
    pub fn classify_line(line: &str) -> DiffLineKind {
        if line.starts_with("@@") {
            DiffLineKind::HunkHeader
        } else if line.starts_with("diff ") || line.starts_with("index ")
            || line.starts_with("--- ") || line.starts_with("+++ ")
        {
            DiffLineKind::FileHeader
        } else if line.starts_with('+') {
            DiffLineKind::Added
        } else if line.starts_with('-') {
            DiffLineKind::Removed
        } else {
            DiffLineKind::Context
        }
    }

    // ── Gutter formatting ────────────────────────────────────────────────────

    fn format_gutter(old: Option<usize>, new: Option<usize>) -> String {
        let l = old.map_or("    ".to_string(), |n| format!("{:>4}", n));
        let r = new.map_or("    ".to_string(), |n| format!("{:>4}", n));
        format!("{} {}", l, r)
    }

    fn format_lineno(n: Option<usize>) -> String {
        n.map_or("    ".to_string(), |v| format!("{:>4}", v))
    }

    // ── Parsing helpers ──────────────────────────────────────────────────────

    fn rebuild_parsed_from_raw(&mut self) {
        self.parsed_lines.clear();
        let mut old_lineno: usize = 0;
        let mut new_lineno: usize = 0;

        for line in &self.raw_lines {
            let kind = Self::classify_line(line);

            // Try to extract starting line numbers from hunk headers.
            if kind == DiffLineKind::HunkHeader {
                if let Some((o, n)) = Self::parse_hunk_header(line) {
                    old_lineno = o;
                    new_lineno = n;
                }
                self.parsed_lines.push(DiffLine {
                    kind,
                    content: line.clone(),
                    old_lineno: None,
                    new_lineno: None,
                });
                continue;
            }

            if kind == DiffLineKind::FileHeader {
                self.parsed_lines.push(DiffLine {
                    kind,
                    content: line.clone(),
                    old_lineno: None,
                    new_lineno: None,
                });
                continue;
            }

            match kind {
                DiffLineKind::Added => {
                    new_lineno += 1;
                    self.parsed_lines.push(DiffLine {
                        kind,
                        content: line.clone(),
                        old_lineno: None,
                        new_lineno: Some(new_lineno),
                    });
                }
                DiffLineKind::Removed => {
                    old_lineno += 1;
                    self.parsed_lines.push(DiffLine {
                        kind,
                        content: line.clone(),
                        old_lineno: Some(old_lineno),
                        new_lineno: None,
                    });
                }
                _ => {
                    // Context
                    old_lineno += 1;
                    new_lineno += 1;
                    self.parsed_lines.push(DiffLine {
                        kind,
                        content: line.clone(),
                        old_lineno: Some(old_lineno),
                        new_lineno: Some(new_lineno),
                    });
                }
            }
        }
    }

    fn rebuild_parsed_from_hunks(&mut self) {
        self.parsed_lines.clear();

        for hunk in &self.hunks {
            self.parsed_lines.push(DiffLine {
                kind: DiffLineKind::HunkHeader,
                content: format!(
                    "@@ -{},{} +{},{} @@",
                    hunk.old_start, hunk.old_count, hunk.new_start, hunk.new_count
                ),
                old_lineno: None,
                new_lineno: None,
            });

            let mut old_lineno = hunk.old_start;
            let mut new_lineno = hunk.new_start;

            for line in &hunk.lines {
                match line.tag {
                    vibe_core::diff::DiffTag::Equal => {
                        self.parsed_lines.push(DiffLine {
                            kind: DiffLineKind::Context,
                            content: format!(" {}", line.content),
                            old_lineno: Some(old_lineno),
                            new_lineno: Some(new_lineno),
                        });
                        old_lineno += 1;
                        new_lineno += 1;
                    }
                    vibe_core::diff::DiffTag::Insert => {
                        self.parsed_lines.push(DiffLine {
                            kind: DiffLineKind::Added,
                            content: format!("+{}", line.content),
                            old_lineno: None,
                            new_lineno: Some(new_lineno),
                        });
                        new_lineno += 1;
                    }
                    vibe_core::diff::DiffTag::Delete => {
                        self.parsed_lines.push(DiffLine {
                            kind: DiffLineKind::Removed,
                            content: format!("-{}", line.content),
                            old_lineno: Some(old_lineno),
                            new_lineno: None,
                        });
                        old_lineno += 1;
                    }
                }
            }
        }
    }

    /// Parse `@@ -old_start,old_count +new_start,new_count @@` into (old_start, new_start).
    fn parse_hunk_header(line: &str) -> Option<(usize, usize)> {
        // Minimal parser: find the numbers after - and +.
        let after_at = line.strip_prefix("@@ ")?;
        let parts: Vec<&str> = after_at.splitn(3, ' ').collect();
        if parts.len() < 2 {
            return None;
        }
        let old_part = parts[0].strip_prefix('-')?;
        let new_part = parts[1].strip_prefix('+')?;
        let old_start = old_part.split(',').next()?.parse::<usize>().ok()?;
        let new_start = new_part.split(',').next()?.parse::<usize>().ok()?;
        // Return start minus 1 because line numbers are incremented before use.
        Some((old_start.saturating_sub(1), new_start.saturating_sub(1)))
    }
}

// ── Tests ────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    // ── DiffViewMode ─────────────────────────────────────────────────────────

    #[test]
    fn view_mode_toggle() {
        assert_eq!(DiffViewMode::Unified.toggled(), DiffViewMode::SideBySide);
        assert_eq!(DiffViewMode::SideBySide.toggled(), DiffViewMode::Unified);
    }

    #[test]
    fn view_mode_label() {
        assert_eq!(DiffViewMode::Unified.label(), "Unified");
        assert_eq!(DiffViewMode::SideBySide.label(), "Side-by-Side");
    }

    // ── DiffViewComponent::new ───────────────────────────────────────────────

    #[test]
    fn new_initial_state_empty_hunks() {
        let comp = DiffViewComponent::new();
        assert!(comp.hunks.is_empty());
    }

    #[test]
    fn new_initial_state_empty_raw_lines() {
        let comp = DiffViewComponent::new();
        assert!(comp.raw_lines.is_empty());
    }

    #[test]
    fn new_initial_state_zero_scroll() {
        let comp = DiffViewComponent::new();
        assert_eq!(comp.scroll, 0);
    }

    #[test]
    fn new_initial_state_unified_mode() {
        let comp = DiffViewComponent::new();
        assert_eq!(comp.view_mode, DiffViewMode::Unified);
    }

    #[test]
    fn new_initial_state_empty_parsed() {
        let comp = DiffViewComponent::new();
        assert!(comp.parsed_lines().is_empty());
    }

    // ── set_diff ─────────────────────────────────────────────────────────────

    #[test]
    fn set_diff_stores_hunks() {
        let mut comp = DiffViewComponent::new();
        comp.set_diff("hello\n", "hello\nworld\n");
        assert!(!comp.hunks.is_empty(), "hunks should be populated after set_diff");
        let has_insert = comp.hunks.iter().any(|h| {
            h.lines.iter().any(|l| l.tag == vibe_core::diff::DiffTag::Insert)
        });
        assert!(has_insert, "diff should contain at least one inserted line");
    }

    #[test]
    fn set_diff_clears_raw_lines_and_resets_scroll() {
        let mut comp = DiffViewComponent::new();
        comp.raw_lines = vec!["old".to_string()];
        comp.scroll = 5;
        comp.set_diff("a\n", "b\n");
        assert!(comp.raw_lines.is_empty(), "raw_lines should be cleared by set_diff");
        assert_eq!(comp.scroll, 0, "scroll should be reset by set_diff");
    }

    #[test]
    fn set_diff_populates_parsed_lines() {
        let mut comp = DiffViewComponent::new();
        comp.set_diff("hello\n", "hello\nworld\n");
        assert!(!comp.parsed_lines().is_empty(), "parsed_lines should be populated");
        // Should have at least one hunk header + content lines.
        assert!(comp.parsed_lines().iter().any(|l| l.kind == DiffLineKind::HunkHeader));
    }

    // ── set_raw_diff ─────────────────────────────────────────────────────────

    #[test]
    fn set_raw_diff_stores_text() {
        let mut comp = DiffViewComponent::new();
        comp.set_raw_diff("--- a/file.txt\n+++ b/file.txt\n-old\n+new\n");
        assert_eq!(comp.raw_lines.len(), 4);
        assert_eq!(comp.raw_lines[0], "--- a/file.txt");
        assert_eq!(comp.raw_lines[3], "+new");
    }

    #[test]
    fn set_raw_diff_clears_hunks_and_resets_scroll() {
        let mut comp = DiffViewComponent::new();
        comp.set_diff("a\n", "b\n");
        comp.scroll = 10;
        comp.set_raw_diff("some diff text");
        assert!(comp.hunks.is_empty(), "hunks should be cleared by set_raw_diff");
        assert_eq!(comp.scroll, 0, "scroll should be reset by set_raw_diff");
    }

    #[test]
    fn set_raw_diff_populates_parsed_lines() {
        let mut comp = DiffViewComponent::new();
        comp.set_raw_diff("--- a/f.txt\n+++ b/f.txt\n@@ -1,2 +1,2 @@\n-old\n+new\n ctx\n");
        assert_eq!(comp.parsed_lines().len(), 6);
    }

    // ── classify_line ────────────────────────────────────────────────────────

    #[test]
    fn classify_added() {
        assert_eq!(DiffViewComponent::classify_line("+new line"), DiffLineKind::Added);
    }

    #[test]
    fn classify_removed() {
        assert_eq!(DiffViewComponent::classify_line("-old line"), DiffLineKind::Removed);
    }

    #[test]
    fn classify_hunk_header() {
        assert_eq!(
            DiffViewComponent::classify_line("@@ -1,3 +1,4 @@"),
            DiffLineKind::HunkHeader,
        );
    }

    #[test]
    fn classify_file_header_diff() {
        assert_eq!(
            DiffViewComponent::classify_line("diff --git a/foo b/foo"),
            DiffLineKind::FileHeader,
        );
    }

    #[test]
    fn classify_file_header_index() {
        assert_eq!(
            DiffViewComponent::classify_line("index abc123..def456 100644"),
            DiffLineKind::FileHeader,
        );
    }

    #[test]
    fn classify_file_header_minus_file() {
        assert_eq!(
            DiffViewComponent::classify_line("--- a/foo.rs"),
            DiffLineKind::FileHeader,
        );
    }

    #[test]
    fn classify_file_header_plus_file() {
        assert_eq!(
            DiffViewComponent::classify_line("+++ b/foo.rs"),
            DiffLineKind::FileHeader,
        );
    }

    #[test]
    fn classify_context() {
        assert_eq!(DiffViewComponent::classify_line(" context line"), DiffLineKind::Context);
        assert_eq!(DiffViewComponent::classify_line("plain text"), DiffLineKind::Context);
    }

    // ── style_for_kind ───────────────────────────────────────────────────────

    #[test]
    fn style_added_uses_green_fg() {
        let s = DiffViewComponent::style_for_kind(DiffLineKind::Added);
        assert_eq!(s.fg, Some(Color::Green));
    }

    #[test]
    fn style_removed_uses_red_fg() {
        let s = DiffViewComponent::style_for_kind(DiffLineKind::Removed);
        assert_eq!(s.fg, Some(Color::Red));
    }

    #[test]
    fn style_hunk_header_uses_cyan() {
        let s = DiffViewComponent::style_for_kind(DiffLineKind::HunkHeader);
        assert_eq!(s.fg, Some(Color::Cyan));
    }

    #[test]
    fn style_context_uses_dark_gray() {
        let s = DiffViewComponent::style_for_kind(DiffLineKind::Context);
        assert_eq!(s.fg, Some(Color::DarkGray));
    }

    #[test]
    fn style_file_header_uses_yellow() {
        let s = DiffViewComponent::style_for_kind(DiffLineKind::FileHeader);
        assert_eq!(s.fg, Some(Color::Yellow));
    }

    #[test]
    fn style_added_has_green_bg() {
        let s = DiffViewComponent::style_for_kind(DiffLineKind::Added);
        assert_eq!(s.bg, Some(Color::Rgb(0, 40, 0)));
    }

    #[test]
    fn style_removed_has_red_bg() {
        let s = DiffViewComponent::style_for_kind(DiffLineKind::Removed);
        assert_eq!(s.bg, Some(Color::Rgb(40, 0, 0)));
    }

    // ── Scroll ───────────────────────────────────────────────────────────────

    #[test]
    fn scroll_up_saturates_at_zero() {
        let mut comp = DiffViewComponent::new();
        assert_eq!(comp.scroll, 0);
        comp.scroll_up();
        assert_eq!(comp.scroll, 0);
        comp.scroll_up();
        assert_eq!(comp.scroll, 0);
    }

    #[test]
    fn scroll_down_increments() {
        let mut comp = DiffViewComponent::new();
        comp.set_raw_diff("@@ -1,3 +1,3 @@\n-a\n+b\n c\n d\n e\n");
        comp.scroll_down();
        assert_eq!(comp.scroll, 1);
        comp.scroll_down();
        assert_eq!(comp.scroll, 2);
    }

    #[test]
    fn scroll_down_clamps_to_max() {
        let mut comp = DiffViewComponent::new();
        comp.set_raw_diff("-a\n+b\n");
        // 2 parsed lines => max scroll = 1
        comp.scroll_down();
        comp.scroll_down();
        comp.scroll_down();
        assert_eq!(comp.scroll, 1);
    }

    #[test]
    fn scroll_up_after_down() {
        let mut comp = DiffViewComponent::new();
        comp.set_raw_diff("-a\n+b\n c\n d\n");
        comp.scroll_down();
        comp.scroll_down();
        comp.scroll_down();
        assert_eq!(comp.scroll, 3);
        comp.scroll_up();
        assert_eq!(comp.scroll, 2);
        comp.scroll_up();
        comp.scroll_up();
        assert_eq!(comp.scroll, 0);
        comp.scroll_up();
        assert_eq!(comp.scroll, 0);
    }

    // ── toggle_view_mode ─────────────────────────────────────────────────────

    #[test]
    fn toggle_view_mode_switches() {
        let mut comp = DiffViewComponent::new();
        assert_eq!(comp.view_mode(), DiffViewMode::Unified);
        comp.toggle_view_mode();
        assert_eq!(comp.view_mode(), DiffViewMode::SideBySide);
        comp.toggle_view_mode();
        assert_eq!(comp.view_mode(), DiffViewMode::Unified);
    }

    // ── clear ────────────────────────────────────────────────────────────────

    #[test]
    fn clear_resets_everything() {
        let mut comp = DiffViewComponent::new();
        comp.set_diff("a\n", "b\n");
        comp.scroll = 7;
        comp.clear();
        assert!(comp.hunks.is_empty());
        assert!(comp.raw_lines.is_empty());
        assert!(comp.parsed_lines().is_empty());
        assert_eq!(comp.scroll, 0);
    }

    // ── parse_hunk_header ────────────────────────────────────────────────────

    #[test]
    fn parse_hunk_header_basic() {
        let result = DiffViewComponent::parse_hunk_header("@@ -10,5 +20,7 @@");
        assert_eq!(result, Some((9, 19)));
    }

    #[test]
    fn parse_hunk_header_single_line() {
        let result = DiffViewComponent::parse_hunk_header("@@ -1 +1 @@");
        assert_eq!(result, Some((0, 0)));
    }

    #[test]
    fn parse_hunk_header_invalid() {
        assert!(DiffViewComponent::parse_hunk_header("not a header").is_none());
    }

    // ── Parsed line numbers ──────────────────────────────────────────────────

    #[test]
    fn raw_diff_line_numbers_added() {
        let mut comp = DiffViewComponent::new();
        comp.set_raw_diff("@@ -1,2 +1,3 @@\n ctx\n+added\n ctx2\n");
        // Lines: header, ctx(old=1,new=1), +added(new=2), ctx2(old=2,new=3)
        let added = comp.parsed_lines().iter().find(|l| l.kind == DiffLineKind::Added).unwrap();
        assert!(added.old_lineno.is_none());
        assert!(added.new_lineno.is_some());
    }

    #[test]
    fn raw_diff_line_numbers_removed() {
        let mut comp = DiffViewComponent::new();
        comp.set_raw_diff("@@ -1,2 +1,1 @@\n-removed\n ctx\n");
        let removed = comp.parsed_lines().iter().find(|l| l.kind == DiffLineKind::Removed).unwrap();
        assert!(removed.old_lineno.is_some());
        assert!(removed.new_lineno.is_none());
    }

    #[test]
    fn raw_diff_context_has_both_line_numbers() {
        let mut comp = DiffViewComponent::new();
        comp.set_raw_diff("@@ -5,2 +10,2 @@\n ctx1\n ctx2\n");
        let ctx_lines: Vec<_> = comp.parsed_lines().iter()
            .filter(|l| l.kind == DiffLineKind::Context)
            .collect();
        assert_eq!(ctx_lines.len(), 2);
        assert_eq!(ctx_lines[0].old_lineno, Some(5));
        assert_eq!(ctx_lines[0].new_lineno, Some(10));
        assert_eq!(ctx_lines[1].old_lineno, Some(6));
        assert_eq!(ctx_lines[1].new_lineno, Some(11));
    }

    // ── render_lines ─────────────────────────────────────────────────────────

    #[test]
    fn render_lines_unified_produces_output() {
        let mut comp = DiffViewComponent::new();
        comp.set_raw_diff("@@ -1,2 +1,2 @@\n-old\n+new\n ctx\n");
        let lines = comp.render_lines();
        assert_eq!(lines.len(), 4);
    }

    #[test]
    fn render_lines_side_by_side_produces_output() {
        let mut comp = DiffViewComponent::new();
        comp.set_raw_diff("@@ -1,2 +1,2 @@\n-old\n+new\n ctx\n");
        comp.toggle_view_mode();
        let lines = comp.render_lines();
        // Header + paired removed/added + context = 3 lines
        assert!(!lines.is_empty());
    }

    // ── gutter formatting ────────────────────────────────────────────────────

    #[test]
    fn format_gutter_both_present() {
        let g = DiffViewComponent::format_gutter(Some(1), Some(2));
        assert!(g.contains('1'));
        assert!(g.contains('2'));
    }

    #[test]
    fn format_gutter_none_old() {
        let g = DiffViewComponent::format_gutter(None, Some(5));
        assert!(g.contains('5'));
        // Old side should be blank spaces.
        assert!(g.starts_with("    "));
    }

    #[test]
    fn format_gutter_none_new() {
        let g = DiffViewComponent::format_gutter(Some(3), None);
        assert!(g.contains('3'));
        assert!(g.ends_with("    "));
    }

    // ── sequential operations ────────────────────────────────────────────────

    #[test]
    fn sequential_operations() {
        let mut comp = DiffViewComponent::new();
        comp.set_diff("hello\n", "world\n");
        assert!(!comp.hunks.is_empty());
        assert!(!comp.parsed_lines().is_empty());
        comp.scroll_down();
        comp.scroll_down();
        assert_eq!(comp.scroll, 2);

        comp.set_raw_diff("line1\nline2\nline3");
        assert!(comp.hunks.is_empty());
        assert_eq!(comp.raw_lines.len(), 3);
        assert_eq!(comp.scroll, 0);
        assert_eq!(comp.parsed_lines().len(), 3);

        comp.scroll_down();
        assert_eq!(comp.scroll, 1);

        comp.toggle_view_mode();
        assert_eq!(comp.view_mode(), DiffViewMode::SideBySide);

        comp.clear();
        assert!(comp.raw_lines.is_empty());
        assert!(comp.parsed_lines().is_empty());
        assert_eq!(comp.scroll, 0);
    }
}
