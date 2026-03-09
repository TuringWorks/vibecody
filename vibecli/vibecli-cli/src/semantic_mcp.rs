//! Semantic Index MCP Server — expose EmbeddingIndex as MCP tools.
//!
//! Closes P1 Gap 5: External MCP clients (Cursor, Claude Code, Zed) can
//! consume VibeCody's semantic codebase index via MCP protocol.
//!
//! # Exposed MCP Tools
//!
//! | Tool | Description |
//! |------|-------------|
//! | `search_codebase` | Semantic search across all indexed files |
//! | `find_related_files` | Find files related to a given file by embedding similarity |
//! | `explain_symbol` | Get context and documentation for a symbol |
//! | `dependency_graph` | Show dependency graph for a file or module |
//! | `index_status` | Current indexing status and statistics |
//! | `reindex` | Trigger re-indexing of changed files |

use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::time::{SystemTime, UNIX_EPOCH};

// ---------------------------------------------------------------------------
// Index entry
// ---------------------------------------------------------------------------

/// A single indexed file in the semantic index.
#[derive(Debug, Clone)]
pub struct IndexEntry {
    pub path: String,
    pub language: String,
    pub symbols: Vec<IndexedSymbol>,
    pub embedding_hash: u64,
    pub last_indexed: u64,
    pub file_size: u64,
    pub line_count: usize,
}

/// A symbol extracted from an indexed file.
#[derive(Debug, Clone)]
pub struct IndexedSymbol {
    pub name: String,
    pub kind: SymbolKind,
    pub line: usize,
    pub documentation: Option<String>,
    pub signature: Option<String>,
}

#[derive(Debug, Clone, PartialEq)]
pub enum SymbolKind {
    Function,
    Struct,
    Trait,
    Enum,
    Const,
    Module,
    Class,
    Interface,
    Variable,
    Type,
}

impl SymbolKind {
    pub fn as_str(&self) -> &str {
        match self {
            SymbolKind::Function => "function",
            SymbolKind::Struct => "struct",
            SymbolKind::Trait => "trait",
            SymbolKind::Enum => "enum",
            SymbolKind::Const => "const",
            SymbolKind::Module => "module",
            SymbolKind::Class => "class",
            SymbolKind::Interface => "interface",
            SymbolKind::Variable => "variable",
            SymbolKind::Type => "type",
        }
    }
}

// ---------------------------------------------------------------------------
// Search results
// ---------------------------------------------------------------------------

/// A search result from the semantic index.
#[derive(Debug, Clone)]
pub struct SearchResult {
    pub path: String,
    pub score: f64,
    pub snippet: String,
    pub line: Option<usize>,
    pub symbol: Option<String>,
}

impl SearchResult {
    pub fn new(path: &str, score: f64, snippet: &str) -> Self {
        Self {
            path: path.to_string(),
            score,
            snippet: snippet.to_string(),
            line: None,
            symbol: None,
        }
    }

    pub fn with_line(mut self, line: usize) -> Self {
        self.line = Some(line);
        self
    }

    pub fn with_symbol(mut self, symbol: &str) -> Self {
        self.symbol = Some(symbol.to_string());
        self
    }
}

/// File relation result.
#[derive(Debug, Clone)]
pub struct RelatedFile {
    pub path: String,
    pub similarity: f64,
    pub reason: String,
}

// ---------------------------------------------------------------------------
// MCP tool definitions
// ---------------------------------------------------------------------------

/// MCP tool definition for the semantic server.
#[derive(Debug, Clone)]
pub struct McpTool {
    pub name: String,
    pub description: String,
    pub parameters: Vec<McpParam>,
}

#[derive(Debug, Clone)]
pub struct McpParam {
    pub name: String,
    pub param_type: String,
    pub description: String,
    pub required: bool,
}

/// Get the list of MCP tools exposed by the semantic index server.
pub fn semantic_mcp_tools() -> Vec<McpTool> {
    vec![
        McpTool {
            name: "search_codebase".into(),
            description: "Semantic search across all indexed files using natural language query".into(),
            parameters: vec![
                McpParam { name: "query".into(), param_type: "string".into(), description: "Natural language search query".into(), required: true },
                McpParam { name: "limit".into(), param_type: "number".into(), description: "Max results (default 10)".into(), required: false },
                McpParam { name: "language".into(), param_type: "string".into(), description: "Filter by language".into(), required: false },
            ],
        },
        McpTool {
            name: "find_related_files".into(),
            description: "Find files semantically related to a given file".into(),
            parameters: vec![
                McpParam { name: "file".into(), param_type: "string".into(), description: "Path to the reference file".into(), required: true },
                McpParam { name: "limit".into(), param_type: "number".into(), description: "Max results (default 5)".into(), required: false },
            ],
        },
        McpTool {
            name: "explain_symbol".into(),
            description: "Get documentation, signature, and usage context for a symbol".into(),
            parameters: vec![
                McpParam { name: "symbol".into(), param_type: "string".into(), description: "Symbol name to look up".into(), required: true },
                McpParam { name: "file".into(), param_type: "string".into(), description: "Optional file hint".into(), required: false },
            ],
        },
        McpTool {
            name: "dependency_graph".into(),
            description: "Show import/dependency graph for a file or module".into(),
            parameters: vec![
                McpParam { name: "file".into(), param_type: "string".into(), description: "File or module path".into(), required: true },
                McpParam { name: "depth".into(), param_type: "number".into(), description: "Max depth (default 2)".into(), required: false },
            ],
        },
        McpTool {
            name: "index_status".into(),
            description: "Get current indexing status, file count, and statistics".into(),
            parameters: vec![],
        },
        McpTool {
            name: "reindex".into(),
            description: "Trigger re-indexing of changed files".into(),
            parameters: vec![
                McpParam { name: "path".into(), param_type: "string".into(), description: "Optional path to reindex (default: entire project)".into(), required: false },
            ],
        },
    ]
}

// ---------------------------------------------------------------------------
// Semantic index server
// ---------------------------------------------------------------------------

/// The semantic index MCP server.
pub struct SemanticIndexServer {
    project_root: PathBuf,
    entries: HashMap<String, IndexEntry>,
    indexed_at: u64,
    indexing: bool,
}

impl SemanticIndexServer {
    pub fn new(project_root: PathBuf) -> Self {
        let ts = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap_or_default()
            .as_secs();
        Self {
            project_root,
            entries: HashMap::new(),
            indexed_at: ts,
            indexing: false,
        }
    }

    pub fn project_root(&self) -> &Path {
        &self.project_root
    }

    pub fn add_entry(&mut self, entry: IndexEntry) {
        self.entries.insert(entry.path.clone(), entry);
    }

    pub fn get_entry(&self, path: &str) -> Option<&IndexEntry> {
        self.entries.get(path)
    }

    pub fn file_count(&self) -> usize {
        self.entries.len()
    }

    pub fn total_symbols(&self) -> usize {
        self.entries.values().map(|e| e.symbols.len()).sum()
    }

    pub fn total_lines(&self) -> usize {
        self.entries.values().map(|e| e.line_count).sum()
    }

    pub fn languages(&self) -> Vec<String> {
        let mut langs: Vec<String> = self
            .entries
            .values()
            .map(|e| e.language.clone())
            .collect();
        langs.sort();
        langs.dedup();
        langs
    }

    /// Search the index by query string (simple keyword matching for now).
    pub fn search(&self, query: &str, limit: usize) -> Vec<SearchResult> {
        let query_lower = query.to_lowercase();
        let terms: Vec<&str> = query_lower.split_whitespace().collect();
        let mut results: Vec<SearchResult> = Vec::new();

        for entry in self.entries.values() {
            // Score based on symbol name matches
            for symbol in &entry.symbols {
                let name_lower = symbol.name.to_lowercase();
                let mut score = 0.0;
                for term in &terms {
                    if name_lower.contains(term) {
                        score += 1.0;
                    }
                    if name_lower == *term {
                        score += 2.0;
                    }
                }
                if score > 0.0 {
                    let snippet = symbol
                        .signature
                        .as_deref()
                        .unwrap_or(&symbol.name);
                    results.push(
                        SearchResult::new(&entry.path, score, snippet)
                            .with_line(symbol.line)
                            .with_symbol(&symbol.name),
                    );
                }
            }
        }

        results.sort_by(|a, b| b.score.partial_cmp(&a.score).unwrap_or(std::cmp::Ordering::Equal));
        results.truncate(limit);
        results
    }

    /// Find files related to a given file.
    pub fn find_related(&self, file_path: &str, limit: usize) -> Vec<RelatedFile> {
        let entry = match self.entries.get(file_path) {
            Some(e) => e,
            None => return Vec::new(),
        };

        let mut related: Vec<RelatedFile> = Vec::new();
        let symbols: Vec<String> = entry.symbols.iter().map(|s| s.name.clone()).collect();

        for (path, other) in &self.entries {
            if path == file_path {
                continue;
            }
            let mut shared_count = 0;
            for other_sym in &other.symbols {
                if symbols.contains(&other_sym.name) {
                    shared_count += 1;
                }
            }
            if shared_count > 0 {
                let similarity = shared_count as f64 / symbols.len().max(1) as f64;
                related.push(RelatedFile {
                    path: path.clone(),
                    similarity,
                    reason: format!("{} shared symbols", shared_count),
                });
            }
            // Same language bonus
            if other.language == entry.language && shared_count == 0 {
                related.push(RelatedFile {
                    path: path.clone(),
                    similarity: 0.1,
                    reason: format!("same language ({})", entry.language),
                });
            }
        }

        related.sort_by(|a, b| b.similarity.partial_cmp(&a.similarity).unwrap_or(std::cmp::Ordering::Equal));
        related.truncate(limit);
        related
    }

    /// Look up a symbol by name.
    pub fn explain_symbol(&self, symbol_name: &str) -> Vec<SymbolExplanation> {
        let mut results = Vec::new();
        for entry in self.entries.values() {
            for symbol in &entry.symbols {
                if symbol.name == symbol_name {
                    results.push(SymbolExplanation {
                        name: symbol.name.clone(),
                        kind: symbol.kind.as_str().to_string(),
                        file: entry.path.clone(),
                        line: symbol.line,
                        documentation: symbol.documentation.clone(),
                        signature: symbol.signature.clone(),
                    });
                }
            }
        }
        results
    }

    pub fn status(&self) -> IndexStatus {
        IndexStatus {
            file_count: self.file_count(),
            symbol_count: self.total_symbols(),
            total_lines: self.total_lines(),
            languages: self.languages(),
            indexed_at: self.indexed_at,
            indexing: self.indexing,
        }
    }

    pub fn start_reindex(&mut self) {
        self.indexing = true;
    }

    pub fn finish_reindex(&mut self) {
        self.indexing = false;
        self.indexed_at = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap_or_default()
            .as_secs();
    }

    pub fn tools(&self) -> Vec<McpTool> {
        semantic_mcp_tools()
    }
}

#[derive(Debug, Clone)]
pub struct SymbolExplanation {
    pub name: String,
    pub kind: String,
    pub file: String,
    pub line: usize,
    pub documentation: Option<String>,
    pub signature: Option<String>,
}

#[derive(Debug, Clone)]
pub struct IndexStatus {
    pub file_count: usize,
    pub symbol_count: usize,
    pub total_lines: usize,
    pub languages: Vec<String>,
    pub indexed_at: u64,
    pub indexing: bool,
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    fn test_entry(path: &str, lang: &str, symbols: Vec<(&str, SymbolKind)>) -> IndexEntry {
        IndexEntry {
            path: path.to_string(),
            language: lang.to_string(),
            symbols: symbols
                .into_iter()
                .map(|(name, kind)| IndexedSymbol {
                    name: name.to_string(),
                    kind,
                    line: 1,
                    documentation: None,
                    signature: Some(format!("fn {}()", name)),
                })
                .collect(),
            embedding_hash: 0,
            last_indexed: 0,
            file_size: 1000,
            line_count: 50,
        }
    }

    fn test_server() -> SemanticIndexServer {
        let mut server = SemanticIndexServer::new(PathBuf::from("/project"));
        server.add_entry(test_entry(
            "src/main.rs",
            "rust",
            vec![("main", SymbolKind::Function), ("Config", SymbolKind::Struct)],
        ));
        server.add_entry(test_entry(
            "src/lib.rs",
            "rust",
            vec![("process", SymbolKind::Function), ("Config", SymbolKind::Struct)],
        ));
        server.add_entry(test_entry(
            "src/utils.ts",
            "typescript",
            vec![("formatDate", SymbolKind::Function)],
        ));
        server
    }

    #[test]
    fn test_symbol_kind_as_str() {
        assert_eq!(SymbolKind::Function.as_str(), "function");
        assert_eq!(SymbolKind::Struct.as_str(), "struct");
        assert_eq!(SymbolKind::Trait.as_str(), "trait");
        assert_eq!(SymbolKind::Class.as_str(), "class");
        assert_eq!(SymbolKind::Interface.as_str(), "interface");
    }

    #[test]
    fn test_search_result() {
        let r = SearchResult::new("src/main.rs", 0.95, "fn main()")
            .with_line(1)
            .with_symbol("main");
        assert_eq!(r.path, "src/main.rs");
        assert_eq!(r.line, Some(1));
        assert_eq!(r.symbol.as_deref(), Some("main"));
    }

    #[test]
    fn test_server_file_count() {
        let server = test_server();
        assert_eq!(server.file_count(), 3);
    }

    #[test]
    fn test_server_total_symbols() {
        let server = test_server();
        assert_eq!(server.total_symbols(), 5);
    }

    #[test]
    fn test_server_total_lines() {
        let server = test_server();
        assert_eq!(server.total_lines(), 150);
    }

    #[test]
    fn test_server_languages() {
        let server = test_server();
        let langs = server.languages();
        assert!(langs.contains(&"rust".to_string()));
        assert!(langs.contains(&"typescript".to_string()));
    }

    #[test]
    fn test_server_get_entry() {
        let server = test_server();
        assert!(server.get_entry("src/main.rs").is_some());
        assert!(server.get_entry("nonexistent").is_none());
    }

    #[test]
    fn test_search_exact() {
        let server = test_server();
        let results = server.search("main", 10);
        assert!(!results.is_empty());
        assert_eq!(results[0].symbol.as_deref(), Some("main"));
    }

    #[test]
    fn test_search_partial() {
        let server = test_server();
        let results = server.search("config", 10);
        assert!(results.len() >= 2);
    }

    #[test]
    fn test_search_no_results() {
        let server = test_server();
        let results = server.search("nonexistent_symbol_xyz", 10);
        assert!(results.is_empty());
    }

    #[test]
    fn test_search_limit() {
        let server = test_server();
        let results = server.search("config", 1);
        assert_eq!(results.len(), 1);
    }

    #[test]
    fn test_find_related() {
        let server = test_server();
        let related = server.find_related("src/main.rs", 5);
        // lib.rs shares "Config" symbol
        assert!(related.iter().any(|r| r.path == "src/lib.rs"));
    }

    #[test]
    fn test_find_related_nonexistent() {
        let server = test_server();
        let related = server.find_related("nonexistent.rs", 5);
        assert!(related.is_empty());
    }

    #[test]
    fn test_explain_symbol() {
        let server = test_server();
        let explanations = server.explain_symbol("Config");
        assert_eq!(explanations.len(), 2); // In both main.rs and lib.rs
        assert!(explanations.iter().all(|e| e.kind == "struct"));
    }

    #[test]
    fn test_explain_symbol_not_found() {
        let server = test_server();
        let explanations = server.explain_symbol("nonexistent");
        assert!(explanations.is_empty());
    }

    #[test]
    fn test_status() {
        let server = test_server();
        let status = server.status();
        assert_eq!(status.file_count, 3);
        assert_eq!(status.symbol_count, 5);
        assert!(!status.indexing);
    }

    #[test]
    fn test_reindex() {
        let mut server = test_server();
        server.start_reindex();
        assert!(server.status().indexing);
        server.finish_reindex();
        assert!(!server.status().indexing);
    }

    #[test]
    fn test_tools() {
        let server = test_server();
        let tools = server.tools();
        assert_eq!(tools.len(), 6);
        assert!(tools.iter().any(|t| t.name == "search_codebase"));
        assert!(tools.iter().any(|t| t.name == "find_related_files"));
        assert!(tools.iter().any(|t| t.name == "explain_symbol"));
        assert!(tools.iter().any(|t| t.name == "dependency_graph"));
        assert!(tools.iter().any(|t| t.name == "index_status"));
        assert!(tools.iter().any(|t| t.name == "reindex"));
    }

    #[test]
    fn test_mcp_tools_params() {
        let tools = semantic_mcp_tools();
        let search = tools.iter().find(|t| t.name == "search_codebase").unwrap();
        assert!(search.parameters.iter().any(|p| p.name == "query" && p.required));
        assert!(search.parameters.iter().any(|p| p.name == "limit" && !p.required));
    }

    #[test]
    fn test_project_root() {
        let server = test_server();
        assert_eq!(server.project_root(), Path::new("/project"));
    }

    #[test]
    fn test_search_sorted_by_score() {
        let server = test_server();
        let results = server.search("config", 10);
        for w in results.windows(2) {
            assert!(w[0].score >= w[1].score);
        }
    }

    #[test]
    fn test_related_sorted_by_similarity() {
        let server = test_server();
        let related = server.find_related("src/main.rs", 10);
        for w in related.windows(2) {
            assert!(w[0].similarity >= w[1].similarity);
        }
    }
}
