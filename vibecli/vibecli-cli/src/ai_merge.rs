#![allow(dead_code)]
//! Semantic merge resolver — AI-assisted three-way merge conflict resolution.
//! Matches GitHub Copilot Workspace v2's semantic merge feature.
//!
//! Strategy:
//! 1. Parse conflict markers (`<<<<<<<`, `=======`, `>>>>>>>`) into `ConflictHunk`s
//! 2. Classify each conflict by type (whitespace, rename, logic, structural)
//! 3. Auto-resolve trivial conflicts; return `Resolution::NeedsReview` for logic conflicts
//! 4. Produce a clean merged file or annotated diff

use std::collections::HashMap;
use std::fmt;

// ---------------------------------------------------------------------------
// Types
// ---------------------------------------------------------------------------

/// Classification of a merge conflict's likely cause.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ConflictKind {
    /// Only whitespace / indentation differs.
    Whitespace,
    /// An identifier was renamed on one side.
    Rename,
    /// Import / use statement ordering differs.
    ImportOrder,
    /// Pure addition on one side, no overlap on other.
    NonOverlapping,
    /// General logic divergence — requires human/AI review.
    Logic,
    /// Structural change (function signature, type shape).
    Structural,
}

impl fmt::Display for ConflictKind {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            ConflictKind::Whitespace => write!(f, "whitespace"),
            ConflictKind::Rename => write!(f, "rename"),
            ConflictKind::ImportOrder => write!(f, "import-order"),
            ConflictKind::NonOverlapping => write!(f, "non-overlapping"),
            ConflictKind::Logic => write!(f, "logic"),
            ConflictKind::Structural => write!(f, "structural"),
        }
    }
}

/// A single `<<<<<<<` … `=======` … `>>>>>>>` conflict block.
#[derive(Debug, Clone)]
pub struct ConflictHunk {
    pub index: usize,
    pub ours_label: String,
    pub theirs_label: String,
    pub ours: Vec<String>,
    pub theirs: Vec<String>,
    pub base: Option<Vec<String>>, // diff3 middle section
    pub kind: ConflictKind,
    pub start_line: usize,
}

/// Outcome of attempting to resolve a conflict.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Resolution {
    /// Conflict was resolved automatically; contains the resolved lines.
    AutoResolved(Vec<String>),
    /// Conflict requires human or AI review.
    NeedsReview,
    /// Prefer our side.
    TakeOurs,
    /// Prefer their side.
    TakeTheirs,
    /// Both sides appended (non-overlapping additions).
    TakeBoth,
}

/// Summary of a merge operation.
#[derive(Debug, Default)]
pub struct MergeSummary {
    pub total_conflicts: usize,
    pub auto_resolved: usize,
    pub needs_review: usize,
    pub resolutions: HashMap<usize, Resolution>,
}

impl MergeSummary {
    pub fn auto_resolve_rate(&self) -> f64 {
        if self.total_conflicts == 0 {
            return 1.0;
        }
        self.auto_resolved as f64 / self.total_conflicts as f64
    }
}

// ---------------------------------------------------------------------------
// Parser
// ---------------------------------------------------------------------------

/// Parses a file with Git conflict markers into hunks + clean lines.
pub struct ConflictParser {
    pub prefer_longer: bool,
}

impl Default for ConflictParser {
    fn default() -> Self {
        Self { prefer_longer: false }
    }
}

impl ConflictParser {
    pub fn new() -> Self {
        Self::default()
    }

    /// Parse `content` into a list of hunks and the surrounding clean lines.
    /// Returns `(clean_sections, hunks)` where clean_sections[i] precedes hunks[i].
    pub fn parse(&self, content: &str) -> (Vec<Vec<String>>, Vec<ConflictHunk>) {
        let lines: Vec<&str> = content.lines().collect();
        let mut clean_sections: Vec<Vec<String>> = Vec::new();
        let mut hunks: Vec<ConflictHunk> = Vec::new();

        let mut current_clean: Vec<String> = Vec::new();
        let mut i = 0;
        let mut hunk_index = 0;

        while i < lines.len() {
            let line = lines[i];

            if line.starts_with("<<<<<<<") {
                // Start of conflict
                let ours_label = line.trim_start_matches('<').trim().to_string();
                let start_line = i + 1;
                i += 1;

                let mut ours: Vec<String> = Vec::new();
                let mut base: Option<Vec<String>> = None;
                let mut theirs: Vec<String> = Vec::new();
                let mut theirs_label = String::new();

                // Collect ours
                while i < lines.len() && !lines[i].starts_with("=======") && !lines[i].starts_with("|||||||") {
                    ours.push(lines[i].to_string());
                    i += 1;
                }

                // Optional diff3 base section
                if i < lines.len() && lines[i].starts_with("|||||||") {
                    i += 1;
                    let mut base_lines: Vec<String> = Vec::new();
                    while i < lines.len() && !lines[i].starts_with("=======") {
                        base_lines.push(lines[i].to_string());
                        i += 1;
                    }
                    base = Some(base_lines);
                }

                // Skip =======
                if i < lines.len() && lines[i].starts_with("=======") {
                    i += 1;
                }

                // Collect theirs
                while i < lines.len() && !lines[i].starts_with(">>>>>>>") {
                    theirs.push(lines[i].to_string());
                    i += 1;
                }

                if i < lines.len() && lines[i].starts_with(">>>>>>>") {
                    theirs_label = lines[i].trim_start_matches('>').trim().to_string();
                    i += 1;
                }

                let kind = classify_conflict(&ours, &theirs, base.as_deref());

                clean_sections.push(current_clean.clone());
                current_clean = Vec::new();

                hunks.push(ConflictHunk {
                    index: hunk_index,
                    ours_label,
                    theirs_label,
                    ours,
                    theirs,
                    base,
                    kind,
                    start_line,
                });
                hunk_index += 1;
            } else {
                current_clean.push(line.to_string());
                i += 1;
            }
        }

        // Trailing clean section
        clean_sections.push(current_clean);

        (clean_sections, hunks)
    }
}

// ---------------------------------------------------------------------------
// Classifier
// ---------------------------------------------------------------------------

fn normalize_whitespace(line: &str) -> String {
    line.split_whitespace().collect::<Vec<_>>().join(" ")
}

fn classify_conflict(ours: &[String], theirs: &[String], _base: Option<&[String]>) -> ConflictKind {
    // Whitespace-only difference
    let ours_norm: Vec<String> = ours.iter().map(|l| normalize_whitespace(l)).collect();
    let theirs_norm: Vec<String> = theirs.iter().map(|l| normalize_whitespace(l)).collect();
    if ours_norm == theirs_norm {
        return ConflictKind::Whitespace;
    }

    // Import ordering (both sides are all import/use lines)
    let ours_imports = ours.iter().all(|l| l.trim().starts_with("use ") || l.trim().starts_with("import "));
    let theirs_imports = theirs.iter().all(|l| l.trim().starts_with("use ") || l.trim().starts_with("import "));
    if ours_imports && theirs_imports {
        return ConflictKind::ImportOrder;
    }

    // One side empty → non-overlapping addition
    if ours.iter().all(|l| l.trim().is_empty()) || theirs.iter().all(|l| l.trim().is_empty()) {
        return ConflictKind::NonOverlapping;
    }

    // Rename detection: same structure, only identifiers differ
    if ours.len() == theirs.len() {
        let token_diffs: usize = ours.iter().zip(theirs.iter())
            .map(|(a, b)| {
                let a_tokens: Vec<&str> = a.split_whitespace().collect();
                let b_tokens: Vec<&str> = b.split_whitespace().collect();
                if a_tokens.len() != b_tokens.len() { return 10usize; }
                a_tokens.iter().zip(b_tokens.iter()).filter(|(x, y)| x != y).count()
            })
            .sum();
        let total_tokens: usize = ours.iter().map(|l| l.split_whitespace().count()).sum();
        if total_tokens > 0 && (token_diffs as f64) / (total_tokens as f64) < 0.3 {
            return ConflictKind::Rename;
        }
    }

    // Structural: function signature or type definition changed
    let structural_keywords = ["fn ", "struct ", "enum ", "trait ", "impl ", "type ", "pub fn ", "async fn "];
    let ours_structural = ours.iter().any(|l| structural_keywords.iter().any(|k| l.contains(k)));
    let theirs_structural = theirs.iter().any(|l| structural_keywords.iter().any(|k| l.contains(k)));
    if ours_structural || theirs_structural {
        return ConflictKind::Structural;
    }

    ConflictKind::Logic
}

// ---------------------------------------------------------------------------
// Resolver
// ---------------------------------------------------------------------------

/// Attempts to automatically resolve merge conflicts based on their kind.
pub struct SemanticMergeResolver {
    /// If true, prefer `ours` for Rename conflicts.
    pub prefer_ours_on_rename: bool,
    /// If true, sort import lines alphabetically for ImportOrder conflicts.
    pub sort_imports: bool,
}

impl Default for SemanticMergeResolver {
    fn default() -> Self {
        Self {
            prefer_ours_on_rename: false,
            sort_imports: true,
        }
    }
}

impl SemanticMergeResolver {
    pub fn new() -> Self {
        Self::default()
    }

    /// Resolve a single conflict hunk.
    pub fn resolve_hunk(&self, hunk: &ConflictHunk) -> Resolution {
        match hunk.kind {
            ConflictKind::Whitespace => {
                // Take ours (whitespace is cosmetic)
                Resolution::AutoResolved(hunk.ours.clone())
            }
            ConflictKind::ImportOrder => {
                // Merge and sort both sides
                let mut merged: Vec<String> = hunk.ours.iter()
                    .chain(hunk.theirs.iter())
                    .filter(|l| !l.trim().is_empty())
                    .cloned()
                    .collect::<std::collections::HashSet<_>>()
                    .into_iter()
                    .collect();
                if self.sort_imports {
                    merged.sort();
                }
                Resolution::AutoResolved(merged)
            }
            ConflictKind::NonOverlapping => {
                // Append both; non-empty side first
                let ours_empty = hunk.ours.iter().all(|l| l.trim().is_empty());
                if ours_empty {
                    Resolution::AutoResolved(hunk.theirs.clone())
                } else {
                    let mut combined = hunk.ours.clone();
                    combined.extend(hunk.theirs.iter().filter(|l| !l.trim().is_empty()).cloned());
                    Resolution::AutoResolved(combined)
                }
            }
            ConflictKind::Rename => {
                if self.prefer_ours_on_rename {
                    Resolution::TakeOurs
                } else {
                    Resolution::NeedsReview
                }
            }
            ConflictKind::Logic | ConflictKind::Structural => Resolution::NeedsReview,
        }
    }

    /// Resolve all hunks and produce a `MergeSummary`.
    pub fn resolve_all(&self, hunks: &[ConflictHunk]) -> MergeSummary {
        let mut summary = MergeSummary {
            total_conflicts: hunks.len(),
            ..Default::default()
        };

        for hunk in hunks {
            let resolution = self.resolve_hunk(hunk);
            if matches!(resolution, Resolution::AutoResolved(_) | Resolution::TakeOurs | Resolution::TakeTheirs | Resolution::TakeBoth) {
                summary.auto_resolved += 1;
            } else {
                summary.needs_review += 1;
            }
            summary.resolutions.insert(hunk.index, resolution);
        }

        summary
    }

    /// Apply resolutions to produce the final merged content.
    pub fn apply(
        &self,
        clean_sections: &[Vec<String>],
        hunks: &[ConflictHunk],
        summary: &MergeSummary,
    ) -> String {
        let mut output = String::new();

        for (i, hunk) in hunks.iter().enumerate() {
            // Clean section before this hunk
            if let Some(clean) = clean_sections.get(i) {
                for line in clean {
                    output.push_str(line);
                    output.push('\n');
                }
            }

            match summary.resolutions.get(&hunk.index) {
                Some(Resolution::AutoResolved(lines)) => {
                    for line in lines {
                        output.push_str(line);
                        output.push('\n');
                    }
                }
                Some(Resolution::TakeOurs) => {
                    for line in &hunk.ours {
                        output.push_str(line);
                        output.push('\n');
                    }
                }
                Some(Resolution::TakeTheirs) => {
                    for line in &hunk.theirs {
                        output.push_str(line);
                        output.push('\n');
                    }
                }
                Some(Resolution::TakeBoth) => {
                    for line in &hunk.ours {
                        output.push_str(line);
                        output.push('\n');
                    }
                    for line in &hunk.theirs {
                        output.push_str(line);
                        output.push('\n');
                    }
                }
                _ => {
                    // NeedsReview: re-emit conflict markers
                    output.push_str(&format!("<<<<<<< {}\n", hunk.ours_label));
                    for line in &hunk.ours {
                        output.push_str(line);
                        output.push('\n');
                    }
                    output.push_str("=======\n");
                    for line in &hunk.theirs {
                        output.push_str(line);
                        output.push('\n');
                    }
                    output.push_str(&format!(">>>>>>> {}\n", hunk.theirs_label));
                }
            }
        }

        // Final trailing clean section
        if let Some(clean) = clean_sections.get(hunks.len()) {
            for line in clean {
                output.push_str(line);
                output.push('\n');
            }
        }

        output
    }
}

// ---------------------------------------------------------------------------
// Public API
// ---------------------------------------------------------------------------

/// High-level entry point: parse, resolve, and apply.
pub fn semantic_merge(content: &str) -> (String, MergeSummary) {
    let parser = ConflictParser::new();
    let resolver = SemanticMergeResolver::new();

    let (clean, hunks) = parser.parse(content);
    let summary = resolver.resolve_all(&hunks);
    let merged = resolver.apply(&clean, &hunks, &summary);

    (merged, summary)
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    fn conflict(ours: &str, theirs: &str) -> String {
        format!("<<<<<<< HEAD\n{}\n=======\n{}\n>>>>>>> feature\n", ours, theirs)
    }

    #[test]
    fn test_parse_single_conflict() {
        let input = conflict("let x = 1;", "let x = 2;");
        let parser = ConflictParser::new();
        let (clean, hunks) = parser.parse(&input);
        assert_eq!(hunks.len(), 1);
        assert_eq!(clean.len(), 2); // before + after
        assert_eq!(hunks[0].ours, vec!["let x = 1;"]);
        assert_eq!(hunks[0].theirs, vec!["let x = 2;"]);
    }

    #[test]
    fn test_classify_whitespace() {
        let kind = classify_conflict(
            &["  let x = 1;".to_string()],
            &["let x = 1;".to_string()],
            None,
        );
        assert_eq!(kind, ConflictKind::Whitespace);
    }

    #[test]
    fn test_classify_import_order() {
        let kind = classify_conflict(
            &["use std::collections::HashMap;".to_string(), "use std::fmt;".to_string()],
            &["use std::fmt;".to_string(), "use std::collections::HashMap;".to_string()],
            None,
        );
        assert_eq!(kind, ConflictKind::ImportOrder);
    }

    #[test]
    fn test_classify_non_overlapping() {
        let kind = classify_conflict(
            &["".to_string()],
            &["fn new_function() {}".to_string()],
            None,
        );
        assert_eq!(kind, ConflictKind::NonOverlapping);
    }

    #[test]
    fn test_classify_logic() {
        let kind = classify_conflict(
            &["if x > 0 { do_a(); } else { do_b(); }".to_string()],
            &["if x >= 0 { do_c(); }".to_string()],
            None,
        );
        assert_eq!(kind, ConflictKind::Logic);
    }

    #[test]
    fn test_classify_structural() {
        let kind = classify_conflict(
            &["fn foo(x: i32) -> i32 {".to_string()],
            &["fn foo(x: i32, y: i32) -> i32 {".to_string()],
            None,
        );
        assert_eq!(kind, ConflictKind::Structural);
    }

    #[test]
    fn test_resolve_whitespace() {
        let hunk = ConflictHunk {
            index: 0,
            ours_label: "HEAD".into(),
            theirs_label: "feature".into(),
            ours: vec!["  let x = 1;".to_string()],
            theirs: vec!["let x = 1;".to_string()],
            base: None,
            kind: ConflictKind::Whitespace,
            start_line: 2,
        };
        let resolver = SemanticMergeResolver::new();
        let res = resolver.resolve_hunk(&hunk);
        assert!(matches!(res, Resolution::AutoResolved(_)));
    }

    #[test]
    fn test_resolve_import_order_sorted() {
        let hunk = ConflictHunk {
            index: 0,
            ours_label: "HEAD".into(),
            theirs_label: "feature".into(),
            ours: vec!["use std::fmt;".to_string()],
            theirs: vec!["use std::collections::HashMap;".to_string()],
            base: None,
            kind: ConflictKind::ImportOrder,
            start_line: 1,
        };
        let resolver = SemanticMergeResolver::new();
        if let Resolution::AutoResolved(lines) = resolver.resolve_hunk(&hunk) {
            assert_eq!(lines.len(), 2);
            // Should be sorted alphabetically
            assert!(lines[0] < lines[1]);
        } else {
            panic!("expected AutoResolved");
        }
    }

    #[test]
    fn test_resolve_logic_needs_review() {
        let hunk = ConflictHunk {
            index: 0,
            ours_label: "HEAD".into(),
            theirs_label: "feature".into(),
            ours: vec!["do_a();".to_string()],
            theirs: vec!["do_b();".to_string()],
            base: None,
            kind: ConflictKind::Logic,
            start_line: 5,
        };
        let resolver = SemanticMergeResolver::new();
        assert_eq!(resolver.resolve_hunk(&hunk), Resolution::NeedsReview);
    }

    #[test]
    fn test_full_merge_whitespace_only() {
        let input = "fn main() {\n<<<<<<< HEAD\n    let x = 1;\n=======\n  let x = 1;\n>>>>>>> feature\n}\n";
        let (merged, summary) = semantic_merge(input);
        assert_eq!(summary.total_conflicts, 1);
        assert_eq!(summary.auto_resolved, 1);
        assert_eq!(summary.needs_review, 0);
        assert!(merged.contains("let x = 1;"));
        assert!(!merged.contains("<<<<<<<"));
    }

    #[test]
    fn test_full_merge_logic_conflict_preserved() {
        let input = conflict("do_a();", "do_b();");
        let (merged, summary) = semantic_merge(&input);
        assert_eq!(summary.needs_review, 1);
        assert!(merged.contains("<<<<<<<"));
    }

    #[test]
    fn test_auto_resolve_rate() {
        let mut summary = MergeSummary::default();
        summary.total_conflicts = 4;
        summary.auto_resolved = 3;
        assert!((summary.auto_resolve_rate() - 0.75).abs() < 1e-9);
    }

    #[test]
    fn test_resolve_non_overlapping() {
        let hunk = ConflictHunk {
            index: 0,
            ours_label: "HEAD".into(),
            theirs_label: "feature".into(),
            ours: vec!["".to_string()],
            theirs: vec!["fn new_fn() {}".to_string()],
            base: None,
            kind: ConflictKind::NonOverlapping,
            start_line: 1,
        };
        let resolver = SemanticMergeResolver::new();
        let res = resolver.resolve_hunk(&hunk);
        assert!(matches!(res, Resolution::AutoResolved(_)));
        if let Resolution::AutoResolved(lines) = res {
            assert!(lines.contains(&"fn new_fn() {}".to_string()));
        }
    }

    #[test]
    fn test_multiple_conflicts_summary() {
        let input = format!(
            "{}\nsome code\n{}",
            conflict("use std::fmt;", "use std::collections;"),
            conflict("complex_logic_a()", "complex_logic_b()")
        );
        let (_, summary) = semantic_merge(&input);
        assert_eq!(summary.total_conflicts, 2);
        assert_eq!(summary.auto_resolved, 1); // import order
        assert_eq!(summary.needs_review, 1); // logic
    }

    #[test]
    fn test_empty_input() {
        let (merged, summary) = semantic_merge("");
        assert_eq!(summary.total_conflicts, 0);
        assert_eq!(summary.auto_resolved, 0);
        assert!(merged.trim().is_empty());
    }
}
