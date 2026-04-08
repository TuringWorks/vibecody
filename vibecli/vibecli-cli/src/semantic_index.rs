//! Deep semantic codebase index — Gap 5 from FIT-GAP v7.
//!
//! Provides a rich structural index of codebases: symbols, call graphs,
//! type hierarchies, import graphs, and API contracts. Supports incremental
//! file indexing/removal, fuzzy symbol search, and regex-based parsing
//! heuristics for Rust, TypeScript, and Python.

use serde::{Deserialize, Serialize};
use std::collections::HashMap;

// ── Enums ───────────────────────────────────────────────────────────────────

/// Kind of source-code symbol.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum SymbolKind {
    Function,
    Method,
    Class,
    Struct,
    Enum,
    Interface,
    Trait,
    Module,
    Constant,
    Variable,
    TypeAlias,
    Macro,
}

impl SymbolKind {
    fn as_str(&self) -> &'static str {
        match self {
            Self::Function => "Function",
            Self::Method => "Method",
            Self::Class => "Class",
            Self::Struct => "Struct",
            Self::Enum => "Enum",
            Self::Interface => "Interface",
            Self::Trait => "Trait",
            Self::Module => "Module",
            Self::Constant => "Constant",
            Self::Variable => "Variable",
            Self::TypeAlias => "TypeAlias",
            Self::Macro => "Macro",
        }
    }
}

/// Visibility of a symbol.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum Visibility {
    Public,
    Private,
    Protected,
    Internal,
    Crate,
}

/// Source language.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum Language {
    Rust,
    TypeScript,
    Python,
    Go,
    Java,
    CSharp,
    Cpp,
    C,
    Ruby,
    Php,
    Swift,
    Kotlin,
    Scala,
    Haskell,
    Elixir,
    Dart,
    Zig,
    Lua,
    Bash,
    Unknown,
}

impl Language {
    fn as_str(&self) -> &'static str {
        match self {
            Self::Rust => "Rust",
            Self::TypeScript => "TypeScript",
            Self::Python => "Python",
            Self::Go => "Go",
            Self::Java => "Java",
            Self::CSharp => "CSharp",
            Self::Cpp => "Cpp",
            Self::C => "C",
            Self::Ruby => "Ruby",
            Self::Php => "PHP",
            Self::Swift => "Swift",
            Self::Kotlin => "Kotlin",
            Self::Scala => "Scala",
            Self::Haskell => "Haskell",
            Self::Elixir => "Elixir",
            Self::Dart => "Dart",
            Self::Zig => "Zig",
            Self::Lua => "Lua",
            Self::Bash => "Bash",
            Self::Unknown => "Unknown",
        }
    }
}

/// Type of function/method call.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum CallType {
    Direct,
    Method,
    Constructor,
    Callback,
    Async,
    Dynamic,
}

/// Relationship between two types.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum TypeRelationType {
    Inherits,
    Implements,
    TraitImpl,
    Extends,
    Mixin,
}

/// Import type.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum ImportType {
    Named,
    Wildcard,
    Default,
    Reexport,
}

// ── Core structs ────────────────────────────────────────────────────────────

/// A source-code symbol with full metadata.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Symbol {
    pub name: String,
    pub kind: SymbolKind,
    pub qualified_name: String,
    pub file_path: String,
    pub line_start: usize,
    pub line_end: usize,
    pub signature: Option<String>,
    pub doc_comment: Option<String>,
    pub visibility: Visibility,
    pub language: Language,
}

/// A directed edge in the call graph.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct CallEdge {
    pub caller: String,
    pub callee: String,
    pub file_path: String,
    pub line: usize,
    pub call_type: CallType,
}

/// A relationship between two types (inheritance / implementation).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct TypeRelation {
    pub parent: String,
    pub child: String,
    pub relation_type: TypeRelationType,
}

/// A file-to-file import edge.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ImportEdge {
    pub source_file: String,
    pub target_file: String,
    pub imported_symbols: Vec<String>,
    pub import_type: ImportType,
}

/// Describes the contract of an API function.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ApiContract {
    pub symbol_name: String,
    pub parameters: Vec<ParamInfo>,
    pub return_type: Option<String>,
    pub error_types: Vec<String>,
    pub is_async: bool,
}

/// A single parameter description.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ParamInfo {
    pub name: String,
    pub param_type: String,
    pub optional: bool,
    pub default_value: Option<String>,
}

/// A recursive tree view of the type hierarchy.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct TypeHierarchyTree {
    pub root: String,
    pub children: Vec<TypeHierarchyTree>,
    pub relation: TypeRelationType,
}

/// Aggregate metrics about the index.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct IndexMetrics {
    pub total_symbols: usize,
    pub total_call_edges: usize,
    pub total_type_relations: usize,
    pub total_import_edges: usize,
    pub total_files: usize,
    pub symbols_by_kind: HashMap<String, usize>,
    pub symbols_by_language: HashMap<String, usize>,
}

impl IndexMetrics {
    fn new() -> Self {
        Self {
            total_symbols: 0,
            total_call_edges: 0,
            total_type_relations: 0,
            total_import_edges: 0,
            total_files: 0,
            symbols_by_kind: HashMap::new(),
            symbols_by_language: HashMap::new(),
        }
    }
}

// ── SemanticIndex ───────────────────────────────────────────────────────────

/// The central semantic index holding symbols, graphs, and contracts.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct SemanticIndex {
    pub symbols: HashMap<String, Symbol>,
    pub call_graph: Vec<CallEdge>,
    pub type_hierarchy_relations: Vec<TypeRelation>,
    pub import_graph: Vec<ImportEdge>,
    pub api_contracts: HashMap<String, ApiContract>,
    pub file_index: HashMap<String, Vec<String>>,
    pub metrics: IndexMetrics,
}

impl Default for SemanticIndex {
    fn default() -> Self {
        Self::new()
    }
}

impl SemanticIndex {
    /// Create an empty index.
    pub fn new() -> Self {
        Self {
            symbols: HashMap::new(),
            call_graph: Vec::new(),
            type_hierarchy_relations: Vec::new(),
            import_graph: Vec::new(),
            api_contracts: HashMap::new(),
            file_index: HashMap::new(),
            metrics: IndexMetrics::new(),
        }
    }

    /// Add a symbol to the index.
    pub fn add_symbol(&mut self, symbol: Symbol) {
        let qn = symbol.qualified_name.clone();
        let kind_key = symbol.kind.as_str().to_string();
        let lang_key = symbol.language.as_str().to_string();
        let file = symbol.file_path.clone();

        self.symbols.insert(qn.clone(), symbol);

        // Update file_index.
        self.file_index.entry(file).or_default().push(qn);

        // Update metrics.
        *self.metrics.symbols_by_kind.entry(kind_key).or_insert(0) += 1;
        *self.metrics.symbols_by_language.entry(lang_key).or_insert(0) += 1;
        self.metrics.total_symbols = self.symbols.len();
        self.metrics.total_files = self.file_index.len();
    }

    /// Add a call edge.
    pub fn add_call_edge(&mut self, edge: CallEdge) {
        self.call_graph.push(edge);
        self.metrics.total_call_edges = self.call_graph.len();
    }

    /// Add a type relation.
    pub fn add_type_relation(&mut self, relation: TypeRelation) {
        self.type_hierarchy_relations.push(relation);
        self.metrics.total_type_relations = self.type_hierarchy_relations.len();
    }

    /// Add an import edge.
    pub fn add_import_edge(&mut self, edge: ImportEdge) {
        self.import_graph.push(edge);
        self.metrics.total_import_edges = self.import_graph.len();
    }

    /// Add an API contract.
    pub fn add_api_contract(&mut self, contract: ApiContract) {
        self.api_contracts
            .insert(contract.symbol_name.clone(), contract);
    }

    /// Index all symbols from a file using the simple parser heuristics.
    pub fn index_file(&mut self, path: &str, content: &str, language: Language) {
        let symbols = match language {
            Language::Rust => SimpleParser::parse_rust(content, path),
            Language::TypeScript => SimpleParser::parse_typescript(content, path),
            Language::Python => SimpleParser::parse_python(content, path),
            _ => Vec::new(),
        };

        for sym in symbols {
            self.add_symbol(sym);
        }
    }

    /// Remove all symbols and edges associated with a file (incremental update).
    pub fn remove_file(&mut self, path: &str) {
        // Remove symbols and update kind/language metrics.
        if let Some(qns) = self.file_index.remove(path) {
            for qn in &qns {
                if let Some(sym) = self.symbols.remove(qn) {
                    let kind_key = sym.kind.as_str().to_string();
                    let lang_key = sym.language.as_str().to_string();
                    if let Some(c) = self.metrics.symbols_by_kind.get_mut(&kind_key) {
                        *c = c.saturating_sub(1);
                    }
                    if let Some(c) = self.metrics.symbols_by_language.get_mut(&lang_key) {
                        *c = c.saturating_sub(1);
                    }
                }
            }
        }

        // Remove call edges referencing this file.
        self.call_graph.retain(|e| e.file_path != path);

        // Remove import edges referencing this file.
        self.import_graph
            .retain(|e| e.source_file != path && e.target_file != path);

        // Refresh aggregate counts.
        self.metrics.total_symbols = self.symbols.len();
        self.metrics.total_call_edges = self.call_graph.len();
        self.metrics.total_import_edges = self.import_graph.len();
        self.metrics.total_files = self.file_index.len();
    }

    /// Return all call edges where `qualified_name` is the callee.
    pub fn callers(&self, qualified_name: &str) -> Vec<&CallEdge> {
        self.call_graph
            .iter()
            .filter(|e| e.callee == qualified_name)
            .collect()
    }

    /// Return all call edges where `qualified_name` is the caller.
    pub fn callees(&self, qualified_name: &str) -> Vec<&CallEdge> {
        self.call_graph
            .iter()
            .filter(|e| e.caller == qualified_name)
            .collect()
    }

    /// Return symbols that implement a given trait / interface.
    pub fn implementations(&self, trait_name: &str) -> Vec<&Symbol> {
        let implementors: Vec<&str> = self
            .type_hierarchy_relations
            .iter()
            .filter(|r| {
                r.parent == trait_name
                    && matches!(
                        r.relation_type,
                        TypeRelationType::Implements | TypeRelationType::TraitImpl
                    )
            })
            .map(|r| r.child.as_str())
            .collect();

        implementors
            .iter()
            .filter_map(|name| self.symbols.get(*name))
            .collect()
    }

    /// Return file paths that import from the given module path.
    pub fn dependents(&self, module_path: &str) -> Vec<String> {
        self.import_graph
            .iter()
            .filter(|e| e.target_file == module_path)
            .map(|e| e.source_file.clone())
            .collect()
    }

    /// Build a type-hierarchy tree rooted at the given type name.
    pub fn type_hierarchy(&self, type_name: &str) -> TypeHierarchyTree {
        self.build_hierarchy_node(type_name, &mut Vec::new())
    }

    fn build_hierarchy_node(
        &self,
        type_name: &str,
        visited: &mut Vec<String>,
    ) -> TypeHierarchyTree {
        // Prevent infinite recursion on circular hierarchies.
        if visited.contains(&type_name.to_string()) {
            return TypeHierarchyTree {
                root: type_name.to_string(),
                children: Vec::new(),
                relation: TypeRelationType::Inherits,
            };
        }
        visited.push(type_name.to_string());

        let children: Vec<TypeHierarchyTree> = self
            .type_hierarchy_relations
            .iter()
            .filter(|r| r.parent == type_name)
            .map(|r| {
                let mut child_node = self.build_hierarchy_node(&r.child, visited);
                child_node.relation = r.relation_type.clone();
                child_node
            })
            .collect();

        // Determine the relation for the root node itself (pick first parent relation, or default).
        let own_relation = self
            .type_hierarchy_relations
            .iter()
            .find(|r| r.child == type_name)
            .map(|r| r.relation_type.clone())
            .unwrap_or(TypeRelationType::Inherits);

        TypeHierarchyTree {
            root: type_name.to_string(),
            children,
            relation: own_relation,
        }
    }

    /// Fuzzy symbol search by name substring (case-insensitive).
    pub fn search_symbols(&self, query: &str) -> Vec<&Symbol> {
        let q = query.to_lowercase();
        self.symbols
            .values()
            .filter(|s| s.name.to_lowercase().contains(&q) || s.qualified_name.to_lowercase().contains(&q))
            .collect()
    }

    /// Return all symbols defined in a file.
    pub fn symbols_in_file(&self, path: &str) -> Vec<&Symbol> {
        self.file_index
            .get(path)
            .map(|qns| qns.iter().filter_map(|qn| self.symbols.get(qn)).collect())
            .unwrap_or_default()
    }

    /// Look up an API contract by symbol name.
    pub fn get_api_contract(&self, name: &str) -> Option<&ApiContract> {
        self.api_contracts.get(name)
    }
}

// ── SimpleParser ────────────────────────────────────────────────────────────

/// Regex-free heuristic parser for basic symbol extraction.
pub struct SimpleParser;

impl SimpleParser {
    /// Detect language from file extension.
    pub fn detect_language(file_path: &str) -> Language {
        let ext = file_path.rsplit('.').next().unwrap_or("");
        match ext {
            "rs" => Language::Rust,
            "ts" | "tsx" => Language::TypeScript,
            "js" | "jsx" => Language::TypeScript, // treat JS as TS for simplicity
            "py" => Language::Python,
            "go" => Language::Go,
            "java" => Language::Java,
            "cs" => Language::CSharp,
            "cpp" | "cc" | "cxx" => Language::Cpp,
            "c" | "h" => Language::C,
            "rb" => Language::Ruby,
            "php" => Language::Php,
            "swift" => Language::Swift,
            "kt" | "kts" => Language::Kotlin,
            "scala" | "sc" => Language::Scala,
            "hs" => Language::Haskell,
            "ex" | "exs" => Language::Elixir,
            "dart" => Language::Dart,
            "zig" => Language::Zig,
            "lua" => Language::Lua,
            "sh" | "bash" | "zsh" => Language::Bash,
            _ => Language::Unknown,
        }
    }

    /// Parse Rust source for symbols (fn, struct, enum, trait, mod, const, type, macro).
    pub fn parse_rust(content: &str, file_path: &str) -> Vec<Symbol> {
        let mut symbols = Vec::new();
        let lines: Vec<&str> = content.lines().collect();
        let mut current_doc: Vec<String> = Vec::new();

        for (i, line) in lines.iter().enumerate() {
            let trimmed = line.trim();

            // Collect doc comments.
            if trimmed.starts_with("///") || trimmed.starts_with("//!") {
                current_doc.push(trimmed.to_string());
                continue;
            }

            let (kind, name, vis, sig) = if let Some(rest) = Self::strip_rust_item(trimmed, "fn ") {
                let name = Self::extract_ident(rest);
                let vis = Self::rust_visibility(trimmed);
                let sig = Self::extract_until(trimmed, '{').or_else(|| Some(trimmed.to_string()));
                (SymbolKind::Function, name, vis, sig)
            } else if let Some(rest) = Self::strip_rust_item(trimmed, "struct ") {
                (SymbolKind::Struct, Self::extract_ident(rest), Self::rust_visibility(trimmed), None)
            } else if let Some(rest) = Self::strip_rust_item(trimmed, "enum ") {
                (SymbolKind::Enum, Self::extract_ident(rest), Self::rust_visibility(trimmed), None)
            } else if let Some(rest) = Self::strip_rust_item(trimmed, "trait ") {
                (SymbolKind::Trait, Self::extract_ident(rest), Self::rust_visibility(trimmed), None)
            } else if let Some(rest) = Self::strip_rust_item(trimmed, "mod ") {
                let name = Self::extract_ident(rest);
                (SymbolKind::Module, name, Self::rust_visibility(trimmed), None)
            } else if let Some(rest) = Self::strip_rust_item(trimmed, "const ") {
                (SymbolKind::Constant, Self::extract_ident(rest), Self::rust_visibility(trimmed), None)
            } else if let Some(rest) = Self::strip_rust_item(trimmed, "type ") {
                (SymbolKind::TypeAlias, Self::extract_ident(rest), Self::rust_visibility(trimmed), None)
            } else if let Some(rest) = Self::strip_rust_item(trimmed, "macro_rules! ") {
                (SymbolKind::Macro, Self::extract_ident(rest), Visibility::Public, None)
            } else {
                current_doc.clear();
                continue;
            };

            if !name.is_empty() {
                let doc = if current_doc.is_empty() {
                    None
                } else {
                    Some(current_doc.join("\n"))
                };
                let qn = format!("{}::{}", file_path, name);
                symbols.push(Symbol {
                    name: name.to_string(),
                    kind,
                    qualified_name: qn,
                    file_path: file_path.to_string(),
                    line_start: i + 1,
                    line_end: i + 1,
                    signature: sig,
                    doc_comment: doc,
                    visibility: vis,
                    language: Language::Rust,
                });
            }
            current_doc.clear();
        }

        symbols
    }

    /// Parse TypeScript source for symbols (function, class, interface, const, type, enum).
    pub fn parse_typescript(content: &str, file_path: &str) -> Vec<Symbol> {
        let mut symbols = Vec::new();
        let lines: Vec<&str> = content.lines().collect();
        let mut current_doc: Vec<String> = Vec::new();

        for (i, line) in lines.iter().enumerate() {
            let trimmed = line.trim();

            // JSDoc lines.
            if trimmed.starts_with("/**") || trimmed.starts_with("*") || trimmed.starts_with("*/") {
                current_doc.push(trimmed.to_string());
                continue;
            }

            let (kind, name, vis, sig) = if trimmed.contains("function ") {
                let rest = Self::after_keyword(trimmed, "function ");
                let name = Self::extract_ident(rest);
                let vis = Self::ts_visibility(trimmed);
                let sig = Self::extract_until(trimmed, '{').or_else(|| Some(trimmed.to_string()));
                (SymbolKind::Function, name, vis, sig)
            } else if trimmed.contains("class ") {
                let rest = Self::after_keyword(trimmed, "class ");
                (SymbolKind::Class, Self::extract_ident(rest), Self::ts_visibility(trimmed), None)
            } else if trimmed.contains("interface ") {
                let rest = Self::after_keyword(trimmed, "interface ");
                (SymbolKind::Interface, Self::extract_ident(rest), Self::ts_visibility(trimmed), None)
            } else if trimmed.contains("enum ") && !trimmed.starts_with("//") {
                let rest = Self::after_keyword(trimmed, "enum ");
                (SymbolKind::Enum, Self::extract_ident(rest), Self::ts_visibility(trimmed), None)
            } else if (trimmed.starts_with("const ") || trimmed.starts_with("export const ")) && !trimmed.contains("=> {") {
                let rest = Self::after_keyword(trimmed, "const ");
                (SymbolKind::Constant, Self::extract_ident(rest), Self::ts_visibility(trimmed), None)
            } else if trimmed.contains("type ") && trimmed.contains('=') {
                let rest = Self::after_keyword(trimmed, "type ");
                (SymbolKind::TypeAlias, Self::extract_ident(rest), Self::ts_visibility(trimmed), None)
            } else {
                current_doc.clear();
                continue;
            };

            if !name.is_empty() {
                let doc = if current_doc.is_empty() {
                    None
                } else {
                    Some(current_doc.join("\n"))
                };
                let qn = format!("{}::{}", file_path, name);
                symbols.push(Symbol {
                    name: name.to_string(),
                    kind,
                    qualified_name: qn,
                    file_path: file_path.to_string(),
                    line_start: i + 1,
                    line_end: i + 1,
                    signature: sig,
                    doc_comment: doc,
                    visibility: vis,
                    language: Language::TypeScript,
                });
            }
            current_doc.clear();
        }

        symbols
    }

    /// Parse Python source for symbols (def, class, import).
    pub fn parse_python(content: &str, file_path: &str) -> Vec<Symbol> {
        let mut symbols = Vec::new();
        let lines: Vec<&str> = content.lines().collect();
        let mut current_doc: Vec<String> = Vec::new();

        for (i, line) in lines.iter().enumerate() {
            let trimmed = line.trim();

            // Docstring / comment accumulation.
            if trimmed.starts_with('#') {
                current_doc.push(trimmed.to_string());
                continue;
            }

            let (kind, name, vis, sig) = if trimmed.starts_with("def ") || trimmed.starts_with("async def ") {
                let rest = if let Some(r) = trimmed.strip_prefix("async def ") {
                    r
                } else if let Some(r) = trimmed.strip_prefix("def ") {
                    r
                } else {
                    unreachable!()
                };
                let name = Self::extract_ident(rest);
                let vis = if name.starts_with('_') {
                    Visibility::Private
                } else {
                    Visibility::Public
                };
                let sig = Self::extract_until(trimmed, ':').or_else(|| Some(trimmed.to_string()));
                (SymbolKind::Function, name, vis, sig)
            } else if let Some(rest) = trimmed.strip_prefix("class ") {
                let name = Self::extract_ident(rest);
                (SymbolKind::Class, name, Visibility::Public, None)
            } else {
                current_doc.clear();
                continue;
            };

            if !name.is_empty() {
                let doc = if current_doc.is_empty() {
                    None
                } else {
                    Some(current_doc.join("\n"))
                };
                let qn = format!("{}::{}", file_path, name);
                symbols.push(Symbol {
                    name: name.to_string(),
                    kind,
                    qualified_name: qn,
                    file_path: file_path.to_string(),
                    line_start: i + 1,
                    line_end: i + 1,
                    signature: sig,
                    doc_comment: doc,
                    visibility: vis,
                    language: Language::Python,
                });
            }
            current_doc.clear();
        }

        symbols
    }

    // ── helpers ─────────────────────────────────────────────────────────────

    /// Strip optional `pub`/`pub(crate)` prefix and the keyword, returning the rest.
    fn strip_rust_item<'a>(line: &'a str, keyword: &str) -> Option<&'a str> {
        // e.g. "pub fn foo()" → "foo()"
        let candidate = if let Some(r) = line.strip_prefix("pub(crate) ") {
            r
        } else if let Some(r) = line.strip_prefix("pub ") {
            r
        } else {
            line
        };
        candidate.strip_prefix(keyword)
    }

    fn rust_visibility(line: &str) -> Visibility {
        if line.starts_with("pub(crate)") {
            Visibility::Crate
        } else if line.starts_with("pub ") || line.starts_with("pub(") {
            Visibility::Public
        } else {
            Visibility::Private
        }
    }

    fn ts_visibility(line: &str) -> Visibility {
        if line.contains("export ") {
            Visibility::Public
        } else {
            Visibility::Private
        }
    }

    /// Extract the first identifier (alphanumeric + '_') from `s`.
    fn extract_ident(s: &str) -> String {
        s.chars()
            .take_while(|c| c.is_alphanumeric() || *c == '_')
            .collect()
    }

    /// Return everything up to (but not including) `ch`.
    fn extract_until(s: &str, ch: char) -> Option<String> {
        s.find(ch).map(|idx| s[..idx].trim().to_string())
    }

    /// Return the substring after the first occurrence of `kw`.
    fn after_keyword<'a>(s: &'a str, kw: &str) -> &'a str {
        match s.find(kw) {
            Some(idx) => &s[idx + kw.len()..],
            None => "",
        }
    }
}

// ── Tests ───────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    // ── helpers ─────────────────────────────────────────────────────────

    fn make_symbol(name: &str, kind: SymbolKind, file: &str) -> Symbol {
        Symbol {
            name: name.to_string(),
            kind,
            qualified_name: format!("{}::{}", file, name),
            file_path: file.to_string(),
            line_start: 1,
            line_end: 1,
            signature: None,
            doc_comment: None,
            visibility: Visibility::Public,
            language: Language::Rust,
        }
    }

    fn make_call_edge(caller: &str, callee: &str, file: &str, line: usize) -> CallEdge {
        CallEdge {
            caller: caller.to_string(),
            callee: callee.to_string(),
            file_path: file.to_string(),
            line,
            call_type: CallType::Direct,
        }
    }

    // ── symbol addition & retrieval ────────────────────────────────────

    #[test]
    fn test_add_symbol() {
        let mut idx = SemanticIndex::new();
        let sym = make_symbol("foo", SymbolKind::Function, "src/lib.rs");
        idx.add_symbol(sym.clone());
        assert_eq!(idx.symbols.len(), 1);
        assert_eq!(idx.symbols.get("src/lib.rs::foo"), Some(&sym));
    }

    #[test]
    fn test_add_multiple_symbols() {
        let mut idx = SemanticIndex::new();
        idx.add_symbol(make_symbol("foo", SymbolKind::Function, "a.rs"));
        idx.add_symbol(make_symbol("Bar", SymbolKind::Struct, "a.rs"));
        idx.add_symbol(make_symbol("Baz", SymbolKind::Enum, "b.rs"));
        assert_eq!(idx.symbols.len(), 3);
        assert_eq!(idx.file_index.len(), 2);
    }

    #[test]
    fn test_duplicate_symbol_overwrites() {
        let mut idx = SemanticIndex::new();
        let s1 = make_symbol("foo", SymbolKind::Function, "a.rs");
        let mut s2 = s1.clone();
        s2.line_start = 42;
        idx.add_symbol(s1);
        idx.add_symbol(s2.clone());
        assert_eq!(idx.symbols.get("a.rs::foo").unwrap().line_start, 42);
    }

    #[test]
    fn test_symbols_in_file() {
        let mut idx = SemanticIndex::new();
        idx.add_symbol(make_symbol("a", SymbolKind::Function, "x.rs"));
        idx.add_symbol(make_symbol("b", SymbolKind::Struct, "x.rs"));
        idx.add_symbol(make_symbol("c", SymbolKind::Enum, "y.rs"));
        assert_eq!(idx.symbols_in_file("x.rs").len(), 2);
        assert_eq!(idx.symbols_in_file("y.rs").len(), 1);
        assert_eq!(idx.symbols_in_file("z.rs").len(), 0);
    }

    // ── call graph ─────────────────────────────────────────────────────

    #[test]
    fn test_add_call_edge() {
        let mut idx = SemanticIndex::new();
        idx.add_call_edge(make_call_edge("a::main", "a::foo", "a.rs", 10));
        assert_eq!(idx.call_graph.len(), 1);
        assert_eq!(idx.metrics.total_call_edges, 1);
    }

    #[test]
    fn test_callers() {
        let mut idx = SemanticIndex::new();
        idx.add_call_edge(make_call_edge("main", "foo", "a.rs", 10));
        idx.add_call_edge(make_call_edge("bar", "foo", "b.rs", 20));
        idx.add_call_edge(make_call_edge("foo", "baz", "a.rs", 15));
        let callers = idx.callers("foo");
        assert_eq!(callers.len(), 2);
    }

    #[test]
    fn test_callees() {
        let mut idx = SemanticIndex::new();
        idx.add_call_edge(make_call_edge("main", "foo", "a.rs", 10));
        idx.add_call_edge(make_call_edge("main", "bar", "a.rs", 11));
        idx.add_call_edge(make_call_edge("foo", "baz", "a.rs", 15));
        let callees = idx.callees("main");
        assert_eq!(callees.len(), 2);
    }

    #[test]
    fn test_no_callers() {
        let idx = SemanticIndex::new();
        assert!(idx.callers("nonexistent").is_empty());
    }

    #[test]
    fn test_no_callees() {
        let idx = SemanticIndex::new();
        assert!(idx.callees("nonexistent").is_empty());
    }

    // ── type hierarchy ─────────────────────────────────────────────────

    #[test]
    fn test_add_type_relation() {
        let mut idx = SemanticIndex::new();
        idx.add_type_relation(TypeRelation {
            parent: "Animal".into(),
            child: "Dog".into(),
            relation_type: TypeRelationType::Inherits,
        });
        assert_eq!(idx.type_hierarchy_relations.len(), 1);
        assert_eq!(idx.metrics.total_type_relations, 1);
    }

    #[test]
    fn test_implementations() {
        let mut idx = SemanticIndex::new();
        idx.add_symbol(make_symbol("MyStruct", SymbolKind::Struct, "a.rs"));
        idx.add_type_relation(TypeRelation {
            parent: "Display".into(),
            child: "a.rs::MyStruct".into(),
            relation_type: TypeRelationType::TraitImpl,
        });
        let impls = idx.implementations("Display");
        assert_eq!(impls.len(), 1);
        assert_eq!(impls[0].name, "MyStruct");
    }

    #[test]
    fn test_implementations_filters_inherits() {
        let mut idx = SemanticIndex::new();
        idx.add_symbol(make_symbol("Derived", SymbolKind::Class, "a.ts"));
        idx.add_type_relation(TypeRelation {
            parent: "Base".into(),
            child: "a.ts::Derived".into(),
            relation_type: TypeRelationType::Inherits,
        });
        // Inherits does not count as implementation.
        assert!(idx.implementations("Base").is_empty());
    }

    #[test]
    fn test_type_hierarchy_tree() {
        let mut idx = SemanticIndex::new();
        idx.add_type_relation(TypeRelation {
            parent: "Animal".into(),
            child: "Dog".into(),
            relation_type: TypeRelationType::Inherits,
        });
        idx.add_type_relation(TypeRelation {
            parent: "Animal".into(),
            child: "Cat".into(),
            relation_type: TypeRelationType::Inherits,
        });
        idx.add_type_relation(TypeRelation {
            parent: "Dog".into(),
            child: "Poodle".into(),
            relation_type: TypeRelationType::Inherits,
        });
        let tree = idx.type_hierarchy("Animal");
        assert_eq!(tree.root, "Animal");
        assert_eq!(tree.children.len(), 2);
        let dog = tree.children.iter().find(|c| c.root == "Dog").unwrap();
        assert_eq!(dog.children.len(), 1);
        assert_eq!(dog.children[0].root, "Poodle");
    }

    #[test]
    fn test_type_hierarchy_circular() {
        let mut idx = SemanticIndex::new();
        idx.add_type_relation(TypeRelation {
            parent: "A".into(),
            child: "B".into(),
            relation_type: TypeRelationType::Extends,
        });
        idx.add_type_relation(TypeRelation {
            parent: "B".into(),
            child: "A".into(),
            relation_type: TypeRelationType::Extends,
        });
        let tree = idx.type_hierarchy("A");
        // Should terminate without stack overflow.
        assert_eq!(tree.root, "A");
    }

    #[test]
    fn test_type_hierarchy_no_children() {
        let idx = SemanticIndex::new();
        let tree = idx.type_hierarchy("Leaf");
        assert_eq!(tree.root, "Leaf");
        assert!(tree.children.is_empty());
    }

    // ── import graph ───────────────────────────────────────────────────

    #[test]
    fn test_add_import_edge() {
        let mut idx = SemanticIndex::new();
        idx.add_import_edge(ImportEdge {
            source_file: "a.rs".into(),
            target_file: "b.rs".into(),
            imported_symbols: vec!["Foo".into()],
            import_type: ImportType::Named,
        });
        assert_eq!(idx.import_graph.len(), 1);
        assert_eq!(idx.metrics.total_import_edges, 1);
    }

    #[test]
    fn test_dependents() {
        let mut idx = SemanticIndex::new();
        idx.add_import_edge(ImportEdge {
            source_file: "a.rs".into(),
            target_file: "lib.rs".into(),
            imported_symbols: vec!["Foo".into()],
            import_type: ImportType::Named,
        });
        idx.add_import_edge(ImportEdge {
            source_file: "b.rs".into(),
            target_file: "lib.rs".into(),
            imported_symbols: vec!["Bar".into()],
            import_type: ImportType::Wildcard,
        });
        let deps = idx.dependents("lib.rs");
        assert_eq!(deps.len(), 2);
        assert!(deps.contains(&"a.rs".to_string()));
        assert!(deps.contains(&"b.rs".to_string()));
    }

    #[test]
    fn test_dependents_empty() {
        let idx = SemanticIndex::new();
        assert!(idx.dependents("nonexistent.rs").is_empty());
    }

    // ── file indexing & removal ────────────────────────────────────────

    #[test]
    fn test_index_file_rust() {
        let mut idx = SemanticIndex::new();
        let content = "pub fn greet() {\n}\n\nstruct Config {\n}\n";
        idx.index_file("lib.rs", content, Language::Rust);
        assert_eq!(idx.symbols.len(), 2);
        assert!(idx.symbols.contains_key("lib.rs::greet"));
        assert!(idx.symbols.contains_key("lib.rs::Config"));
    }

    #[test]
    fn test_remove_file() {
        let mut idx = SemanticIndex::new();
        idx.add_symbol(make_symbol("foo", SymbolKind::Function, "a.rs"));
        idx.add_symbol(make_symbol("bar", SymbolKind::Function, "b.rs"));
        idx.add_call_edge(make_call_edge("a.rs::foo", "b.rs::bar", "a.rs", 5));
        idx.add_import_edge(ImportEdge {
            source_file: "a.rs".into(),
            target_file: "b.rs".into(),
            imported_symbols: vec!["bar".into()],
            import_type: ImportType::Named,
        });

        idx.remove_file("a.rs");
        assert_eq!(idx.symbols.len(), 1);
        assert!(!idx.symbols.contains_key("a.rs::foo"));
        assert!(idx.symbols.contains_key("b.rs::bar"));
        assert!(idx.call_graph.is_empty());
        assert!(idx.import_graph.is_empty());
        assert_eq!(idx.metrics.total_files, 1);
    }

    #[test]
    fn test_remove_nonexistent_file() {
        let mut idx = SemanticIndex::new();
        idx.add_symbol(make_symbol("foo", SymbolKind::Function, "a.rs"));
        idx.remove_file("nonexistent.rs");
        assert_eq!(idx.symbols.len(), 1);
    }

    #[test]
    fn test_file_reindex() {
        let mut idx = SemanticIndex::new();
        let content_v1 = "pub fn old_fn() {}\n";
        idx.index_file("lib.rs", content_v1, Language::Rust);
        assert!(idx.symbols.contains_key("lib.rs::old_fn"));

        idx.remove_file("lib.rs");
        let content_v2 = "pub fn new_fn() {}\n";
        idx.index_file("lib.rs", content_v2, Language::Rust);
        assert!(!idx.symbols.contains_key("lib.rs::old_fn"));
        assert!(idx.symbols.contains_key("lib.rs::new_fn"));
    }

    // ── symbol search ──────────────────────────────────────────────────

    #[test]
    fn test_search_exact() {
        let mut idx = SemanticIndex::new();
        idx.add_symbol(make_symbol("process_data", SymbolKind::Function, "a.rs"));
        idx.add_symbol(make_symbol("load_config", SymbolKind::Function, "b.rs"));
        let results = idx.search_symbols("process_data");
        assert_eq!(results.len(), 1);
        assert_eq!(results[0].name, "process_data");
    }

    #[test]
    fn test_search_fuzzy_substring() {
        let mut idx = SemanticIndex::new();
        idx.add_symbol(make_symbol("process_data", SymbolKind::Function, "a.rs"));
        idx.add_symbol(make_symbol("data_loader", SymbolKind::Function, "b.rs"));
        idx.add_symbol(make_symbol("run", SymbolKind::Function, "c.rs"));
        let results = idx.search_symbols("data");
        assert_eq!(results.len(), 2);
    }

    #[test]
    fn test_search_case_insensitive() {
        let mut idx = SemanticIndex::new();
        idx.add_symbol(make_symbol("MyStruct", SymbolKind::Struct, "a.rs"));
        let results = idx.search_symbols("mystruct");
        assert_eq!(results.len(), 1);
    }

    #[test]
    fn test_search_no_results() {
        let mut idx = SemanticIndex::new();
        idx.add_symbol(make_symbol("foo", SymbolKind::Function, "a.rs"));
        assert!(idx.search_symbols("zzzzz").is_empty());
    }

    #[test]
    fn test_search_by_qualified_name() {
        let mut idx = SemanticIndex::new();
        idx.add_symbol(make_symbol("run", SymbolKind::Function, "src/main.rs"));
        let results = idx.search_symbols("src/main.rs::run");
        assert_eq!(results.len(), 1);
    }

    // ── API contracts ──────────────────────────────────────────────────

    #[test]
    fn test_add_api_contract() {
        let mut idx = SemanticIndex::new();
        let contract = ApiContract {
            symbol_name: "create_user".into(),
            parameters: vec![ParamInfo {
                name: "name".into(),
                param_type: "String".into(),
                optional: false,
                default_value: None,
            }],
            return_type: Some("User".into()),
            error_types: vec!["ValidationError".into()],
            is_async: true,
        };
        idx.add_api_contract(contract.clone());
        assert_eq!(idx.get_api_contract("create_user"), Some(&contract));
    }

    #[test]
    fn test_get_api_contract_none() {
        let idx = SemanticIndex::new();
        assert!(idx.get_api_contract("nonexistent").is_none());
    }

    #[test]
    fn test_api_contract_overwrite() {
        let mut idx = SemanticIndex::new();
        let c1 = ApiContract {
            symbol_name: "f".into(),
            parameters: vec![],
            return_type: None,
            error_types: vec![],
            is_async: false,
        };
        let c2 = ApiContract {
            symbol_name: "f".into(),
            parameters: vec![],
            return_type: Some("i32".into()),
            error_types: vec![],
            is_async: true,
        };
        idx.add_api_contract(c1);
        idx.add_api_contract(c2.clone());
        assert_eq!(idx.get_api_contract("f").unwrap().is_async, true);
    }

    // ── metrics ────────────────────────────────────────────────────────

    #[test]
    fn test_metrics_initial() {
        let idx = SemanticIndex::new();
        assert_eq!(idx.metrics.total_symbols, 0);
        assert_eq!(idx.metrics.total_call_edges, 0);
        assert_eq!(idx.metrics.total_type_relations, 0);
        assert_eq!(idx.metrics.total_import_edges, 0);
        assert_eq!(idx.metrics.total_files, 0);
    }

    #[test]
    fn test_metrics_symbols_by_kind() {
        let mut idx = SemanticIndex::new();
        idx.add_symbol(make_symbol("a", SymbolKind::Function, "a.rs"));
        idx.add_symbol(make_symbol("b", SymbolKind::Function, "a.rs"));
        idx.add_symbol(make_symbol("C", SymbolKind::Struct, "a.rs"));
        assert_eq!(idx.metrics.symbols_by_kind.get("Function"), Some(&2));
        assert_eq!(idx.metrics.symbols_by_kind.get("Struct"), Some(&1));
    }

    #[test]
    fn test_metrics_symbols_by_language() {
        let mut idx = SemanticIndex::new();
        let mut ts_sym = make_symbol("X", SymbolKind::Class, "a.ts");
        ts_sym.language = Language::TypeScript;
        idx.add_symbol(make_symbol("foo", SymbolKind::Function, "a.rs"));
        idx.add_symbol(ts_sym);
        assert_eq!(idx.metrics.symbols_by_language.get("Rust"), Some(&1));
        assert_eq!(idx.metrics.symbols_by_language.get("TypeScript"), Some(&1));
    }

    #[test]
    fn test_metrics_after_removal() {
        let mut idx = SemanticIndex::new();
        idx.add_symbol(make_symbol("a", SymbolKind::Function, "a.rs"));
        idx.add_symbol(make_symbol("b", SymbolKind::Struct, "b.rs"));
        idx.remove_file("a.rs");
        assert_eq!(idx.metrics.total_symbols, 1);
        assert_eq!(idx.metrics.total_files, 1);
        assert_eq!(idx.metrics.symbols_by_kind.get("Function"), Some(&0));
    }

    // ── SimpleParser: Rust ─────────────────────────────────────────────

    #[test]
    fn test_parse_rust_fn() {
        let code = "pub fn hello(name: &str) -> String {\n    format!(\"Hi {}\", name)\n}\n";
        let syms = SimpleParser::parse_rust(code, "lib.rs");
        assert_eq!(syms.len(), 1);
        assert_eq!(syms[0].name, "hello");
        assert_eq!(syms[0].kind, SymbolKind::Function);
        assert_eq!(syms[0].visibility, Visibility::Public);
        assert!(syms[0].signature.is_some());
    }

    #[test]
    fn test_parse_rust_struct() {
        let code = "pub struct Config {\n    pub name: String,\n}\n";
        let syms = SimpleParser::parse_rust(code, "config.rs");
        assert_eq!(syms.len(), 1);
        assert_eq!(syms[0].kind, SymbolKind::Struct);
        assert_eq!(syms[0].name, "Config");
    }

    #[test]
    fn test_parse_rust_enum() {
        let code = "enum Color {\n    Red,\n    Green,\n}\n";
        let syms = SimpleParser::parse_rust(code, "color.rs");
        assert_eq!(syms.len(), 1);
        assert_eq!(syms[0].kind, SymbolKind::Enum);
        assert_eq!(syms[0].visibility, Visibility::Private);
    }

    #[test]
    fn test_parse_rust_trait() {
        let code = "pub trait Drawable {\n    fn draw(&self);\n}\n";
        let syms = SimpleParser::parse_rust(code, "draw.rs");
        assert!(syms.iter().any(|s| s.kind == SymbolKind::Trait && s.name == "Drawable"));
    }

    #[test]
    fn test_parse_rust_mod() {
        let code = "pub mod utils;\n";
        let syms = SimpleParser::parse_rust(code, "lib.rs");
        assert_eq!(syms.len(), 1);
        assert_eq!(syms[0].kind, SymbolKind::Module);
        assert_eq!(syms[0].name, "utils");
    }

    #[test]
    fn test_parse_rust_const() {
        let code = "const MAX: usize = 100;\n";
        let syms = SimpleParser::parse_rust(code, "lib.rs");
        assert_eq!(syms.len(), 1);
        assert_eq!(syms[0].kind, SymbolKind::Constant);
    }

    #[test]
    fn test_parse_rust_type_alias() {
        let code = "pub type Result<T> = std::result::Result<T, MyError>;\n";
        let syms = SimpleParser::parse_rust(code, "lib.rs");
        assert_eq!(syms.len(), 1);
        assert_eq!(syms[0].kind, SymbolKind::TypeAlias);
        assert_eq!(syms[0].name, "Result");
    }

    #[test]
    fn test_parse_rust_macro() {
        let code = "macro_rules! my_macro {\n    () => {};\n}\n";
        let syms = SimpleParser::parse_rust(code, "macros.rs");
        assert_eq!(syms.len(), 1);
        assert_eq!(syms[0].kind, SymbolKind::Macro);
        assert_eq!(syms[0].name, "my_macro");
    }

    #[test]
    fn test_parse_rust_doc_comment() {
        let code = "/// This is the doc comment.\npub fn documented() {}\n";
        let syms = SimpleParser::parse_rust(code, "lib.rs");
        assert_eq!(syms.len(), 1);
        assert!(syms[0].doc_comment.is_some());
        assert!(syms[0].doc_comment.as_ref().unwrap().contains("doc comment"));
    }

    #[test]
    fn test_parse_rust_pub_crate() {
        let code = "pub(crate) fn internal() {}\n";
        let syms = SimpleParser::parse_rust(code, "lib.rs");
        assert_eq!(syms.len(), 1);
        assert_eq!(syms[0].visibility, Visibility::Crate);
    }

    #[test]
    fn test_parse_rust_multiple() {
        let code = "pub struct Foo {}\npub fn bar() {}\nenum Baz {}\ntrait Qux {}\nmod inner;\n";
        let syms = SimpleParser::parse_rust(code, "lib.rs");
        assert_eq!(syms.len(), 5);
    }

    // ── SimpleParser: TypeScript ───────────────────────────────────────

    #[test]
    fn test_parse_typescript_function() {
        let code = "export function greet(name: string): string {\n  return name;\n}\n";
        let syms = SimpleParser::parse_typescript(code, "index.ts");
        assert_eq!(syms.len(), 1);
        assert_eq!(syms[0].name, "greet");
        assert_eq!(syms[0].kind, SymbolKind::Function);
        assert_eq!(syms[0].visibility, Visibility::Public);
    }

    #[test]
    fn test_parse_typescript_class() {
        let code = "export class UserService {\n  constructor() {}\n}\n";
        let syms = SimpleParser::parse_typescript(code, "user.ts");
        assert_eq!(syms.len(), 1);
        assert_eq!(syms[0].kind, SymbolKind::Class);
    }

    #[test]
    fn test_parse_typescript_interface() {
        let code = "export interface Config {\n  name: string;\n}\n";
        let syms = SimpleParser::parse_typescript(code, "types.ts");
        assert_eq!(syms.len(), 1);
        assert_eq!(syms[0].kind, SymbolKind::Interface);
    }

    #[test]
    fn test_parse_typescript_enum() {
        let code = "enum Status {\n  Active,\n  Inactive,\n}\n";
        let syms = SimpleParser::parse_typescript(code, "status.ts");
        assert_eq!(syms.len(), 1);
        assert_eq!(syms[0].kind, SymbolKind::Enum);
        assert_eq!(syms[0].visibility, Visibility::Private);
    }

    #[test]
    fn test_parse_typescript_const() {
        let code = "export const MAX_RETRIES = 3;\n";
        let syms = SimpleParser::parse_typescript(code, "config.ts");
        assert_eq!(syms.len(), 1);
        assert_eq!(syms[0].kind, SymbolKind::Constant);
    }

    #[test]
    fn test_parse_typescript_type_alias() {
        let code = "export type UserId = string;\n";
        let syms = SimpleParser::parse_typescript(code, "types.ts");
        assert_eq!(syms.len(), 1);
        assert_eq!(syms[0].kind, SymbolKind::TypeAlias);
    }

    // ── SimpleParser: Python ───────────────────────────────────────────

    #[test]
    fn test_parse_python_def() {
        let code = "def greet(name):\n    print(name)\n";
        let syms = SimpleParser::parse_python(code, "main.py");
        assert_eq!(syms.len(), 1);
        assert_eq!(syms[0].name, "greet");
        assert_eq!(syms[0].kind, SymbolKind::Function);
        assert_eq!(syms[0].visibility, Visibility::Public);
    }

    #[test]
    fn test_parse_python_async_def() {
        let code = "async def fetch(url):\n    pass\n";
        let syms = SimpleParser::parse_python(code, "client.py");
        assert_eq!(syms.len(), 1);
        assert_eq!(syms[0].name, "fetch");
    }

    #[test]
    fn test_parse_python_class() {
        let code = "class Animal:\n    pass\n";
        let syms = SimpleParser::parse_python(code, "models.py");
        assert_eq!(syms.len(), 1);
        assert_eq!(syms[0].kind, SymbolKind::Class);
    }

    #[test]
    fn test_parse_python_private() {
        let code = "def _internal():\n    pass\n";
        let syms = SimpleParser::parse_python(code, "util.py");
        assert_eq!(syms.len(), 1);
        assert_eq!(syms[0].visibility, Visibility::Private);
    }

    #[test]
    fn test_parse_python_doc_comment() {
        let code = "# This is a helper.\ndef helper():\n    pass\n";
        let syms = SimpleParser::parse_python(code, "util.py");
        assert_eq!(syms.len(), 1);
        assert!(syms[0].doc_comment.is_some());
    }

    #[test]
    fn test_parse_python_multiple() {
        let code = "class Foo:\n    pass\n\ndef bar():\n    pass\n\nasync def baz():\n    pass\n";
        let syms = SimpleParser::parse_python(code, "app.py");
        assert_eq!(syms.len(), 3);
    }

    // ── language detection ─────────────────────────────────────────────

    #[test]
    fn test_detect_language_rust() {
        assert_eq!(SimpleParser::detect_language("src/main.rs"), Language::Rust);
    }

    #[test]
    fn test_detect_language_typescript() {
        assert_eq!(SimpleParser::detect_language("app.ts"), Language::TypeScript);
        assert_eq!(SimpleParser::detect_language("app.tsx"), Language::TypeScript);
    }

    #[test]
    fn test_detect_language_python() {
        assert_eq!(SimpleParser::detect_language("script.py"), Language::Python);
    }

    #[test]
    fn test_detect_language_go() {
        assert_eq!(SimpleParser::detect_language("main.go"), Language::Go);
    }

    #[test]
    fn test_detect_language_various() {
        assert_eq!(SimpleParser::detect_language("X.java"), Language::Java);
        assert_eq!(SimpleParser::detect_language("X.cs"), Language::CSharp);
        assert_eq!(SimpleParser::detect_language("X.cpp"), Language::Cpp);
        assert_eq!(SimpleParser::detect_language("X.c"), Language::C);
        assert_eq!(SimpleParser::detect_language("X.rb"), Language::Ruby);
        assert_eq!(SimpleParser::detect_language("X.php"), Language::Php);
        assert_eq!(SimpleParser::detect_language("X.swift"), Language::Swift);
        assert_eq!(SimpleParser::detect_language("X.kt"), Language::Kotlin);
        assert_eq!(SimpleParser::detect_language("X.scala"), Language::Scala);
        assert_eq!(SimpleParser::detect_language("X.hs"), Language::Haskell);
        assert_eq!(SimpleParser::detect_language("X.ex"), Language::Elixir);
        assert_eq!(SimpleParser::detect_language("X.dart"), Language::Dart);
        assert_eq!(SimpleParser::detect_language("X.zig"), Language::Zig);
        assert_eq!(SimpleParser::detect_language("X.lua"), Language::Lua);
        assert_eq!(SimpleParser::detect_language("X.sh"), Language::Bash);
    }

    #[test]
    fn test_detect_language_unknown() {
        assert_eq!(SimpleParser::detect_language("file.xyz"), Language::Unknown);
    }

    // ── index_file with auto-detect ────────────────────────────────────

    #[test]
    fn test_index_file_typescript() {
        let mut idx = SemanticIndex::new();
        let code = "export function main() {}\nexport class App {}\n";
        idx.index_file("app.ts", code, Language::TypeScript);
        assert_eq!(idx.symbols.len(), 2);
    }

    #[test]
    fn test_index_file_python() {
        let mut idx = SemanticIndex::new();
        let code = "class Service:\n    pass\ndef run():\n    pass\n";
        idx.index_file("svc.py", code, Language::Python);
        assert_eq!(idx.symbols.len(), 2);
    }

    #[test]
    fn test_index_file_unknown_language() {
        let mut idx = SemanticIndex::new();
        idx.index_file("data.csv", "a,b,c\n1,2,3", Language::Unknown);
        assert_eq!(idx.symbols.len(), 0);
    }

    // ── edge cases ─────────────────────────────────────────────────────

    #[test]
    fn test_empty_index() {
        let idx = SemanticIndex::new();
        assert!(idx.symbols.is_empty());
        assert!(idx.call_graph.is_empty());
        assert!(idx.type_hierarchy_relations.is_empty());
        assert!(idx.import_graph.is_empty());
        assert!(idx.api_contracts.is_empty());
        assert!(idx.file_index.is_empty());
    }

    #[test]
    fn test_call_edge_types() {
        let mut idx = SemanticIndex::new();
        idx.add_call_edge(CallEdge {
            caller: "a".into(),
            callee: "b".into(),
            file_path: "x.rs".into(),
            line: 1,
            call_type: CallType::Async,
        });
        idx.add_call_edge(CallEdge {
            caller: "c".into(),
            callee: "d".into(),
            file_path: "x.rs".into(),
            line: 2,
            call_type: CallType::Constructor,
        });
        assert_eq!(idx.call_graph[0].call_type, CallType::Async);
        assert_eq!(idx.call_graph[1].call_type, CallType::Constructor);
    }

    #[test]
    fn test_import_types() {
        let mut idx = SemanticIndex::new();
        for itype in [ImportType::Named, ImportType::Wildcard, ImportType::Default, ImportType::Reexport] {
            idx.add_import_edge(ImportEdge {
                source_file: "a.rs".into(),
                target_file: "b.rs".into(),
                imported_symbols: vec![],
                import_type: itype,
            });
        }
        assert_eq!(idx.import_graph.len(), 4);
    }

    #[test]
    fn test_serialization_roundtrip() {
        let mut idx = SemanticIndex::new();
        idx.add_symbol(make_symbol("foo", SymbolKind::Function, "a.rs"));
        idx.add_call_edge(make_call_edge("a::x", "a::y", "a.rs", 1));
        let json = serde_json::to_string(&idx).expect("serialize");
        let idx2: SemanticIndex = serde_json::from_str(&json).expect("deserialize");
        assert_eq!(idx, idx2);
    }

    #[test]
    fn test_parse_empty_content() {
        assert!(SimpleParser::parse_rust("", "empty.rs").is_empty());
        assert!(SimpleParser::parse_typescript("", "empty.ts").is_empty());
        assert!(SimpleParser::parse_python("", "empty.py").is_empty());
    }

    #[test]
    fn test_symbol_line_numbers() {
        let code = "\n\npub fn third_line() {}\n";
        let syms = SimpleParser::parse_rust(code, "test.rs");
        assert_eq!(syms.len(), 1);
        assert_eq!(syms[0].line_start, 3);
    }
}
