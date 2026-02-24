//! Incremental codebase indexer for symbol discovery and context-aware search.
//!
//! Walks the workspace with `walkdir`, respects common ignore patterns,
//! extracts symbols via regex-based heuristics, and caches file content
//! by modification time for fast incremental updates.

pub mod symbol;

pub use symbol::{Language, SymbolInfo, SymbolKind};

use anyhow::Result;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::time::SystemTime;
use walkdir::WalkDir;

// ── File Entry ────────────────────────────────────────────────────────────────

#[derive(Debug, Clone)]
#[allow(dead_code)]
struct FileEntry {
    modified: SystemTime,
    symbols: Vec<SymbolInfo>,
    language: Language,
    line_count: usize,
}

// ── Search Result ─────────────────────────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct IndexSearchResult {
    pub file: PathBuf,
    pub line: usize,
    pub snippet: String,
    pub score: f32,
}

// ── CodebaseIndex ─────────────────────────────────────────────────────────────

/// Incremental codebase index with symbol table and content cache.
pub struct CodebaseIndex {
    workspace_root: PathBuf,
    /// Per-file entry keyed by absolute path.
    files: HashMap<PathBuf, FileEntry>,
    /// Flattened symbol table for fast lookup.
    symbols: Vec<SymbolInfo>,
}

impl CodebaseIndex {
    pub fn new(workspace_root: PathBuf) -> Self {
        Self {
            workspace_root,
            files: HashMap::new(),
            symbols: Vec::new(),
        }
    }

    /// Build the index by walking the workspace. Skips hidden dirs and common
    /// non-source paths. Uses mtime to skip unchanged files on refresh.
    pub fn build(&mut self) -> Result<IndexStats> {
        let root = self.workspace_root.clone();
        let mut stats = IndexStats::default();

        for entry in WalkDir::new(&root)
            .follow_links(false)
            .into_iter()
            .filter_map(|e| e.ok())
        {
            let path = entry.path();

            // Skip directories
            if path.is_dir() {
                continue;
            }

            // Skip ignored paths
            if should_skip(path) {
                continue;
            }

            let ext = path.extension().and_then(|e| e.to_str()).unwrap_or("");
            let language = Language::from_extension(ext);

            if !language.is_source() {
                continue;
            }

            let modified = match std::fs::metadata(path).and_then(|m| m.modified()) {
                Ok(m) => m,
                Err(_) => continue,
            };

            // Skip if unchanged
            if let Some(existing) = self.files.get(path) {
                if existing.modified == modified {
                    stats.skipped += 1;
                    continue;
                }
            }

            let content = match std::fs::read_to_string(path) {
                Ok(c) => c,
                Err(_) => continue, // binary or unreadable
            };

            let path_buf = path.to_path_buf();
            let symbols = symbol::extract_symbols(&path_buf, &content, &language);
            let line_count = content.lines().count();

            stats.indexed += 1;
            stats.symbols_found += symbols.len();

            self.files.insert(
                path_buf,
                FileEntry { modified, symbols, language, line_count },
            );
        }

        // Rebuild flat symbol table
        self.symbols = self.files.values().flat_map(|f| f.symbols.clone()).collect();

        stats.total_files = self.files.len();
        stats.total_symbols = self.symbols.len();
        Ok(stats)
    }

    /// Refresh stale files only (call after file-change events).
    pub fn refresh(&mut self, changed: &[PathBuf]) -> Result<()> {
        for path in changed {
            if should_skip(path) {
                continue;
            }
            let ext = path.extension().and_then(|e| e.to_str()).unwrap_or("");
            let language = Language::from_extension(ext);
            if !language.is_source() {
                continue;
            }
            match std::fs::read_to_string(path) {
                Ok(content) => {
                    let modified = std::fs::metadata(path)
                        .and_then(|m| m.modified())
                        .unwrap_or(SystemTime::UNIX_EPOCH);
                    let symbols = symbol::extract_symbols(path, &content, &language);
                    let line_count = content.lines().count();
                    self.files.insert(
                        path.clone(),
                        FileEntry { modified, symbols, language, line_count },
                    );
                }
                Err(_) => {
                    // File deleted or unreadable — remove from index
                    self.files.remove(path);
                }
            }
        }
        // Rebuild symbol table
        self.symbols = self.files.values().flat_map(|f| f.symbols.clone()).collect();
        Ok(())
    }

    /// Search symbols by name (case-insensitive substring match), scored by relevance.
    pub fn search_symbols(&self, query: &str) -> Vec<SymbolInfo> {
        let q = query.to_lowercase();
        let mut scored: Vec<(f32, &SymbolInfo)> = self
            .symbols
            .iter()
            .filter_map(|s| {
                let name_lower = s.name.to_lowercase();
                let score = score_symbol(&name_lower, &q);
                if score > 0.0 { Some((score, s)) } else { None }
            })
            .collect();
        scored.sort_by(|a, b| b.0.partial_cmp(&a.0).unwrap());
        scored.into_iter().map(|(_, s)| s.clone()).collect()
    }

    /// Return all symbols in a specific file.
    pub fn symbols_in_file(&self, path: &Path) -> Vec<SymbolInfo> {
        self.files
            .get(path)
            .map(|f| f.symbols.clone())
            .unwrap_or_default()
    }

    /// Return all indexed symbols.
    pub fn all_symbols(&self) -> &[SymbolInfo] {
        &self.symbols
    }

    /// Return all indexed file paths.
    pub fn indexed_files(&self) -> impl Iterator<Item = &PathBuf> {
        self.files.keys()
    }

    pub fn file_count(&self) -> usize {
        self.files.len()
    }

    pub fn symbol_count(&self) -> usize {
        self.symbols.len()
    }

    /// Return symbols most relevant to `task` text (scored by term overlap).
    pub fn relevant_symbols(&self, task: &str, limit: usize) -> Vec<SymbolInfo> {
        let task_terms: Vec<String> = tokenize(task);
        if task_terms.is_empty() {
            return self.symbols.iter().take(limit).cloned().collect();
        }
        let mut scored: Vec<(f32, &SymbolInfo)> = self
            .symbols
            .iter()
            .map(|s| {
                let score = relevance_score(s, &task_terms);
                (score, s)
            })
            .collect();
        scored.sort_by(|a, b| b.0.partial_cmp(&a.0).unwrap());
        scored
            .into_iter()
            .take(limit)
            .filter(|(s, _)| *s > 0.0)
            .map(|(_, s)| s.clone())
            .collect()
    }
}

// ── Scoring ───────────────────────────────────────────────────────────────────

fn score_symbol(name: &str, query: &str) -> f32 {
    if name == query {
        return 1.0;
    }
    if name.starts_with(query) {
        return 0.9;
    }
    if name.contains(query) {
        return 0.7;
    }
    0.0
}

fn relevance_score(symbol: &SymbolInfo, task_terms: &[String]) -> f32 {
    let name_lower = symbol.name.to_lowercase();
    let sig_lower = symbol.signature.to_lowercase();
    let file_name = symbol
        .file
        .file_name()
        .and_then(|n| n.to_str())
        .unwrap_or("")
        .to_lowercase();

    let mut score = 0.0f32;
    for term in task_terms {
        if name_lower.contains(term.as_str()) {
            score += 2.0;
        } else if sig_lower.contains(term.as_str()) {
            score += 1.0;
        } else if file_name.contains(term.as_str()) {
            score += 0.5;
        }
    }
    score
}

fn tokenize(text: &str) -> Vec<String> {
    text.split(|c: char| !c.is_alphanumeric() && c != '_')
        .filter(|t| t.len() >= 3)
        .map(|t| t.to_lowercase())
        .collect()
}

// ── Ignore Logic ──────────────────────────────────────────────────────────────

fn should_skip(path: &Path) -> bool {
    const SKIP_DIRS: &[&str] = &[
        ".git", ".svn", "node_modules", "target", "dist", "build",
        "__pycache__", ".venv", "venv", ".tox", ".mypy_cache",
        ".pytest_cache", "vendor", ".cargo",
    ];
    const SKIP_PATTERNS: &[&str] = &[
        ".min.js", ".min.css", ".bundle.js", "package-lock.json",
        "yarn.lock", "Cargo.lock", ".d.ts",
    ];

    let path_str = path.to_string_lossy();

    for skip_dir in SKIP_DIRS {
        if path_str.contains(&format!("/{}/", skip_dir))
            || path_str.contains(&format!("\\{}\\", skip_dir))
            || path_str.ends_with(&format!("/{}", skip_dir))
        {
            return true;
        }
    }

    // Skip hidden files/dirs
    for component in path.components() {
        let s = component.as_os_str().to_string_lossy();
        if s.starts_with('.') && s.len() > 1 {
            return true;
        }
    }

    let file_name = path.file_name().and_then(|n| n.to_str()).unwrap_or("");
    for pattern in SKIP_PATTERNS {
        if file_name.ends_with(pattern) {
            return true;
        }
    }

    false
}

// ── Stats ─────────────────────────────────────────────────────────────────────

#[derive(Debug, Default)]
pub struct IndexStats {
    pub indexed: usize,
    pub skipped: usize,
    pub total_files: usize,
    pub total_symbols: usize,
    pub symbols_found: usize,
}

// ── Tests ─────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;
    use super::symbol::{extract_symbols, Language};

    #[test]
    fn test_rust_symbol_extraction() {
        let content = "pub fn main() {}\npub struct Foo;\npub enum Bar { A, B }\n";
        let path = PathBuf::from("test.rs");
        let symbols = extract_symbols(&path, content, &Language::Rust);
        let names: Vec<_> = symbols.iter().map(|s| s.name.as_str()).collect();
        assert!(names.contains(&"main"), "expected main fn");
        assert!(names.contains(&"Foo"), "expected Foo struct");
        assert!(names.contains(&"Bar"), "expected Bar enum");
    }

    #[test]
    fn test_python_symbol_extraction() {
        let content = "def hello():\n    pass\nclass MyClass:\n    def method(self):\n        pass\n";
        let path = PathBuf::from("test.py");
        let symbols = extract_symbols(&path, content, &Language::Python);
        let names: Vec<_> = symbols.iter().map(|s| s.name.as_str()).collect();
        assert!(names.contains(&"hello"));
        assert!(names.contains(&"MyClass"));
    }

    #[test]
    fn test_relevance_scoring() {
        let symbols = vec![
            SymbolInfo {
                name: "authenticate_user".to_string(),
                kind: SymbolKind::Function,
                file: PathBuf::from("auth.rs"),
                line: 1,
                signature: "pub fn authenticate_user(token: &str)".to_string(),
                language: Language::Rust,
            },
            SymbolInfo {
                name: "get_config".to_string(),
                kind: SymbolKind::Function,
                file: PathBuf::from("config.rs"),
                line: 1,
                signature: "pub fn get_config() -> Config".to_string(),
                language: Language::Rust,
            },
        ];
        let terms = vec!["auth".to_string(), "user".to_string()];
        let score_auth = relevance_score(&symbols[0], &terms);
        let score_config = relevance_score(&symbols[1], &terms);
        assert!(score_auth > score_config, "auth symbol should score higher");
    }

    #[test]
    fn test_skip_logic() {
        assert!(should_skip(Path::new("/proj/node_modules/foo.js")));
        assert!(should_skip(Path::new("/proj/target/debug/lib.rs")));
        assert!(!should_skip(Path::new("/proj/src/main.rs")));
    }
}
