#![allow(dead_code)]
//! Multi-file symbol rename — finds all references to a symbol across the
//! workspace and generates a rename diff. Matches Cursor 4.0's rename refactor.
//!
//! Pipeline:
//! 1. `ReferenceScanner::scan_file()` — regex-based symbol occurrence finder
//! 2. `RenameEngine::collect_references()` — workspace-wide scan
//! 3. `RenameEngine::generate_edits()` — produce `TextEdit` list per file
//! 4. `RenameEngine::apply_edits()` — apply and return updated file contents

use std::collections::HashMap;
use std::path::{Path, PathBuf};

// ---------------------------------------------------------------------------
// Types
// ---------------------------------------------------------------------------

/// A single occurrence of a symbol in a file.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SymbolRef {
    pub file: PathBuf,
    /// 1-based line number.
    pub line: usize,
    /// 0-based byte offset within the line.
    pub col_start: usize,
    pub col_end: usize,
    pub context_line: String,
    pub ref_kind: RefKind,
}

/// How a symbol is being referenced.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum RefKind {
    Definition,
    Call,
    TypeAnnotation,
    Import,
    DocComment,
    Unknown,
}

impl RefKind {
    fn classify(line: &str, col: usize, sym: &str) -> RefKind {
        let before = &line[..col];
        let trimmed = line.trim();

        if trimmed.starts_with("use ") || trimmed.starts_with("import ") {
            return RefKind::Import;
        }
        if trimmed.starts_with("///") || trimmed.starts_with("//!") || trimmed.starts_with("/**") {
            return RefKind::DocComment;
        }
        if before.contains("fn ") || before.contains("struct ") || before.contains("enum ") || before.contains("trait ") || before.contains("type ") {
            // Check if it's the definition name
            let after_keyword = before.rsplit_once("fn ")
                .or_else(|| before.rsplit_once("struct "))
                .or_else(|| before.rsplit_once("enum "))
                .or_else(|| before.rsplit_once("trait "))
                .or_else(|| before.rsplit_once("type "))
                .map(|(_, r)| r);
            if let Some(rest) = after_keyword {
                if rest.trim() == sym || rest.trim().is_empty() {
                    return RefKind::Definition;
                }
            }
        }
        // Type annotation: preceded by `: ` or `-> ` or `<`
        if before.trim_end().ends_with(':') || before.trim_end().ends_with("->") || before.ends_with('<') {
            return RefKind::TypeAnnotation;
        }
        // Function call: followed by `(`
        let after_sym = &line[col + sym.len()..];
        if after_sym.trim_start().starts_with('(') {
            return RefKind::Call;
        }
        RefKind::Unknown
    }
}

/// A text edit: replace `old_text` at a specific location with `new_text`.
#[derive(Debug, Clone)]
pub struct TextEdit {
    pub file: PathBuf,
    pub line: usize,
    pub col_start: usize,
    pub col_end: usize,
    pub old_text: String,
    pub new_text: String,
}

// ---------------------------------------------------------------------------
// Scanner
// ---------------------------------------------------------------------------

/// Scans a single file for occurrences of `symbol` (whole-word match).
pub struct ReferenceScanner {
    pub include_doc_comments: bool,
    pub case_sensitive: bool,
}

impl Default for ReferenceScanner {
    fn default() -> Self {
        Self {
            include_doc_comments: true,
            case_sensitive: true,
        }
    }
}

impl ReferenceScanner {
    pub fn new() -> Self {
        Self::default()
    }

    /// Scan `content` of `file_path` for occurrences of `symbol`.
    pub fn scan_content(&self, file_path: &Path, content: &str, symbol: &str) -> Vec<SymbolRef> {
        let mut refs = Vec::new();

        for (line_idx, line) in content.lines().enumerate() {
            let search_line = if self.case_sensitive {
                line.to_string()
            } else {
                line.to_lowercase()
            };
            let search_sym = if self.case_sensitive {
                symbol.to_string()
            } else {
                symbol.to_lowercase()
            };

            let mut search_start = 0;
            while let Some(pos) = search_line[search_start..].find(&search_sym) {
                let abs_pos = search_start + pos;
                let col_end = abs_pos + symbol.len();

                // Whole-word check
                let before_ok = abs_pos == 0 || !line.chars().nth(abs_pos - 1).map(|c| c.is_alphanumeric() || c == '_').unwrap_or(false);
                let after_ok = col_end >= line.len() || !line.chars().nth(col_end).map(|c| c.is_alphanumeric() || c == '_').unwrap_or(false);

                if before_ok && after_ok {
                    let ref_kind = RefKind::classify(line, abs_pos, symbol);

                    if ref_kind == RefKind::DocComment && !self.include_doc_comments {
                        search_start = abs_pos + 1;
                        continue;
                    }

                    refs.push(SymbolRef {
                        file: file_path.to_path_buf(),
                        line: line_idx + 1,
                        col_start: abs_pos,
                        col_end,
                        context_line: line.to_string(),
                        ref_kind,
                    });
                }

                search_start = abs_pos + 1;
                if search_start >= search_line.len() {
                    break;
                }
            }
        }

        refs
    }
}

// ---------------------------------------------------------------------------
// Engine
// ---------------------------------------------------------------------------

/// Collects references and generates rename edits across an in-memory file set.
pub struct RenameEngine {
    scanner: ReferenceScanner,
    /// file_path → file_content
    pub files: HashMap<PathBuf, String>,
}

impl RenameEngine {
    pub fn new() -> Self {
        Self {
            scanner: ReferenceScanner::new(),
            files: HashMap::new(),
        }
    }

    pub fn with_files(files: HashMap<PathBuf, String>) -> Self {
        Self {
            scanner: ReferenceScanner::new(),
            files,
        }
    }

    pub fn add_file(&mut self, path: impl Into<PathBuf>, content: impl Into<String>) {
        self.files.insert(path.into(), content.into());
    }

    /// Collect all references to `symbol` across the workspace.
    pub fn collect_references(&self, symbol: &str) -> Vec<SymbolRef> {
        let mut all_refs: Vec<SymbolRef> = Vec::new();
        for (path, content) in &self.files {
            let file_refs = self.scanner.scan_content(path, content, symbol);
            all_refs.extend(file_refs);
        }
        // Sort for determinism: by file path, then line, then col
        all_refs.sort_by(|a, b| {
            a.file.cmp(&b.file)
                .then(a.line.cmp(&b.line))
                .then(a.col_start.cmp(&b.col_start))
        });
        all_refs
    }

    /// Generate `TextEdit`s to rename `old_symbol` → `new_symbol`.
    pub fn generate_edits(&self, old_symbol: &str, new_symbol: &str) -> Vec<TextEdit> {
        let refs = self.collect_references(old_symbol);
        refs.into_iter()
            .map(|r| TextEdit {
                file: r.file,
                line: r.line,
                col_start: r.col_start,
                col_end: r.col_end,
                old_text: old_symbol.to_string(),
                new_text: new_symbol.to_string(),
            })
            .collect()
    }

    /// Apply edits and return updated file contents keyed by path.
    pub fn apply_edits(&self, edits: &[TextEdit]) -> HashMap<PathBuf, String> {
        // Group edits by file
        let mut by_file: HashMap<&PathBuf, Vec<&TextEdit>> = HashMap::new();
        for edit in edits {
            by_file.entry(&edit.file).or_default().push(edit);
        }

        let mut results: HashMap<PathBuf, String> = HashMap::new();

        for (file, file_edits) in &by_file {
            let content = match self.files.get(*file) {
                Some(c) => c.clone(),
                None => continue,
            };

            let mut lines: Vec<String> = content.lines().map(|l| l.to_string()).collect();

            // Sort edits: descending col so multi-edit on same line works right-to-left
            let mut sorted = file_edits.to_vec();
            sorted.sort_by(|a, b| b.line.cmp(&a.line).then(b.col_start.cmp(&a.col_start)));

            for edit in sorted {
                let line_idx = edit.line - 1;
                if line_idx >= lines.len() {
                    continue;
                }
                let line = &lines[line_idx];
                if edit.col_end <= line.len() {
                    let mut new_line = String::new();
                    new_line.push_str(&line[..edit.col_start]);
                    new_line.push_str(&edit.new_text);
                    new_line.push_str(&line[edit.col_end..]);
                    lines[line_idx] = new_line;
                }
            }

            results.insert((*file).clone(), lines.join("\n") + "\n");
        }

        results
    }

    /// High-level: rename `old` → `new` across all files.
    pub fn rename(&self, old_symbol: &str, new_symbol: &str) -> RenameResult {
        let edits = self.generate_edits(old_symbol, new_symbol);
        let files_affected = {
            let mut set: std::collections::HashSet<PathBuf> = std::collections::HashSet::new();
            for e in &edits {
                set.insert(e.file.clone());
            }
            set.len()
        };
        let total_edits = edits.len();
        let updated = self.apply_edits(&edits);
        RenameResult {
            old_symbol: old_symbol.to_string(),
            new_symbol: new_symbol.to_string(),
            files_affected,
            total_edits,
            updated_files: updated,
        }
    }
}

/// Result of a rename operation.
#[derive(Debug)]
pub struct RenameResult {
    pub old_symbol: String,
    pub new_symbol: String,
    pub files_affected: usize,
    pub total_edits: usize,
    pub updated_files: HashMap<PathBuf, String>,
}

impl RenameResult {
    pub fn summary(&self) -> String {
        format!(
            "Renamed `{}` → `{}`: {} edits across {} file(s)",
            self.old_symbol, self.new_symbol, self.total_edits, self.files_affected
        )
    }
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;

    fn p(s: &str) -> PathBuf { PathBuf::from(s) }

    #[test]
    fn test_scan_single_occurrence() {
        let scanner = ReferenceScanner::new();
        let refs = scanner.scan_content(&p("main.rs"), "fn foo() {}", "foo");
        assert_eq!(refs.len(), 1);
        assert_eq!(refs[0].line, 1);
        assert_eq!(refs[0].col_start, 3);
    }

    #[test]
    fn test_whole_word_match() {
        let scanner = ReferenceScanner::new();
        let refs = scanner.scan_content(&p("a.rs"), "foobar + foo + xfoo", "foo");
        assert_eq!(refs.len(), 1);
        assert_eq!(refs[0].col_start, 9);
    }

    #[test]
    fn test_classify_call() {
        let kind = RefKind::classify("    foo(args);", 4, "foo");
        assert_eq!(kind, RefKind::Call);
    }

    #[test]
    fn test_classify_import() {
        let kind = RefKind::classify("use crate::foo;", 11, "foo");
        assert_eq!(kind, RefKind::Import);
    }

    #[test]
    fn test_classify_type_annotation() {
        let kind = RefKind::classify("let x: MyType = ...", 7, "MyType");
        assert_eq!(kind, RefKind::TypeAnnotation);
    }

    #[test]
    fn test_rename_single_file() {
        let mut engine = RenameEngine::new();
        engine.add_file("src/lib.rs", "fn foo() {}\nfoo();\n");
        let result = engine.rename("foo", "bar");
        assert_eq!(result.total_edits, 2);
        let content = result.updated_files.get(&p("src/lib.rs")).unwrap();
        assert!(content.contains("fn bar()"));
        assert!(content.contains("bar();"));
        assert!(!content.contains("foo"));
    }

    #[test]
    fn test_rename_multi_file() {
        let mut engine = RenameEngine::new();
        engine.add_file("src/a.rs", "pub fn process() {}\n");
        engine.add_file("src/b.rs", "use crate::process;\nprocess();\n");
        let result = engine.rename("process", "handle");
        assert_eq!(result.files_affected, 2);
        assert_eq!(result.total_edits, 3);
    }

    #[test]
    fn test_rename_summary() {
        let mut engine = RenameEngine::new();
        engine.add_file("main.rs", "fn my_fn() {}\n");
        let result = engine.rename("my_fn", "new_fn");
        let summary = result.summary();
        assert!(summary.contains("my_fn"));
        assert!(summary.contains("new_fn"));
    }

    #[test]
    fn test_no_partial_match() {
        let mut engine = RenameEngine::new();
        engine.add_file("a.rs", "let foobar = 1;\nfoo();\n");
        let result = engine.rename("foo", "bar");
        assert_eq!(result.total_edits, 1); // only `foo()` line
    }

    #[test]
    fn test_collect_references_sorted() {
        let mut engine = RenameEngine::new();
        engine.add_file("z.rs", "fn sym() {}\n");
        engine.add_file("a.rs", "sym();\nsym();\n");
        let refs = engine.collect_references("sym");
        assert!(refs[0].file <= refs[1].file);
    }

    #[test]
    fn test_generate_edits_count() {
        let mut engine = RenameEngine::new();
        engine.add_file("x.rs", "foo\nfoo\nfoo\n");
        let edits = engine.generate_edits("foo", "bar");
        assert_eq!(edits.len(), 3);
    }

    #[test]
    fn test_case_insensitive_scan() {
        let mut scanner = ReferenceScanner::new();
        scanner.case_sensitive = false;
        let refs = scanner.scan_content(&p("x.rs"), "FOO + foo + Foo", "foo");
        assert_eq!(refs.len(), 3);
    }

    #[test]
    fn test_no_doc_comments_option() {
        let mut scanner = ReferenceScanner::new();
        scanner.include_doc_comments = false;
        let refs = scanner.scan_content(&p("x.rs"), "/// foo is great\nfoo();\n", "foo");
        assert_eq!(refs.len(), 1);
        assert_eq!(refs[0].line, 2);
    }

    #[test]
    fn test_empty_workspace() {
        let engine = RenameEngine::new();
        let result = engine.rename("nonexistent", "new");
        assert_eq!(result.total_edits, 0);
        assert_eq!(result.files_affected, 0);
    }

    #[test]
    fn test_multi_occurrence_same_line() {
        let mut engine = RenameEngine::new();
        engine.add_file("x.rs", "let foo = foo();\n");
        let result = engine.rename("foo", "bar");
        assert_eq!(result.total_edits, 2);
        let content = result.updated_files.get(&p("x.rs")).unwrap();
        assert!(!content.contains("foo"));
    }
}
