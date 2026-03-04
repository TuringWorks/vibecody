//! Language-aware symbol extraction using regex patterns.

use regex::Regex;
use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum Language {
    Rust,
    TypeScript,
    JavaScript,
    Python,
    Go,
    Unknown,
}

impl Language {
    pub fn from_extension(ext: &str) -> Self {
        match ext.to_lowercase().as_str() {
            "rs" => Self::Rust,
            "ts" | "tsx" => Self::TypeScript,
            "js" | "jsx" | "mjs" => Self::JavaScript,
            "py" | "pyi" => Self::Python,
            "go" => Self::Go,
            _ => Self::Unknown,
        }
    }

    pub fn is_source(&self) -> bool {
        !matches!(self, Self::Unknown)
    }

    pub fn as_str(&self) -> &'static str {
        match self {
            Language::Rust => "rust",
            Language::TypeScript => "typescript",
            Language::JavaScript => "javascript",
            Language::Python => "python",
            Language::Go => "go",
            Language::Unknown => "unknown",
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum SymbolKind {
    Function,
    Method,
    Struct,
    Enum,
    Trait,
    Class,
    Interface,
    Constant,
    Type,
    Module,
    Variable,
}

impl SymbolKind {
    pub fn as_str(&self) -> &'static str {
        match self {
            SymbolKind::Function => "fn",
            SymbolKind::Method => "method",
            SymbolKind::Struct => "struct",
            SymbolKind::Enum => "enum",
            SymbolKind::Trait => "trait",
            SymbolKind::Class => "class",
            SymbolKind::Interface => "interface",
            SymbolKind::Constant => "const",
            SymbolKind::Type => "type",
            SymbolKind::Module => "mod",
            SymbolKind::Variable => "var",
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SymbolInfo {
    pub name: String,
    pub kind: SymbolKind,
    pub file: PathBuf,
    pub line: usize,
    /// First line of the definition (trimmed).
    pub signature: String,
    pub language: Language,
}

impl SymbolInfo {
    /// Format as a one-line reference suitable for context injection.
    pub fn format_ref(&self) -> String {
        format!(
            "[{}] {} {} ({}:{})",
            self.language.as_str(),
            self.kind.as_str(),
            self.name,
            self.file.display(),
            self.line,
        )
    }
}

// ── Extraction ────────────────────────────────────────────────────────────────

/// Extract symbols from `content` using language-specific regex patterns.
pub fn extract_symbols(path: &Path, content: &str, language: &Language) -> Vec<SymbolInfo> {
    match language {
        Language::Rust => extract_with_patterns(path, content, Language::Rust, RUST_PATTERNS),
        Language::TypeScript => extract_with_patterns(path, content, Language::TypeScript, TS_PATTERNS),
        Language::JavaScript => extract_with_patterns(path, content, Language::JavaScript, TS_PATTERNS),
        Language::Python => extract_with_patterns(path, content, Language::Python, PYTHON_PATTERNS),
        Language::Go => extract_with_patterns(path, content, Language::Go, GO_PATTERNS),
        Language::Unknown => vec![],
    }
}

fn extract_with_patterns(
    path: &Path,
    content: &str,
    language: Language,
    patterns: &[(&'static str, SymbolKind)],
) -> Vec<SymbolInfo>
where
    SymbolKind: Clone,
{
    let lines: Vec<&str> = content.lines().collect();
    let mut symbols: Vec<SymbolInfo> = Vec::new();
    let mut seen: std::collections::HashSet<(usize, String)> = Default::default();

    for (pattern, kind) in patterns {
        let re = match Regex::new(pattern) {
            Ok(r) => r,
            Err(_) => continue,
        };
        for (line_idx, &line) in lines.iter().enumerate() {
            if let Some(cap) = re.captures(line) {
                if let Some(name_match) = cap.get(1) {
                    let name = name_match.as_str().to_string();
                    let key = (line_idx, name.clone());
                    if !seen.contains(&key) {
                        seen.insert(key);
                        symbols.push(SymbolInfo {
                            name,
                            kind: kind.clone(),
                            file: path.to_path_buf(),
                            line: line_idx + 1,
                            signature: line.trim().chars().take(120).collect(),
                            language: language.clone(),
                        });
                    }
                }
            }
        }
    }
    symbols
}

/// Patterns are (regex, SymbolKind). Capture group 1 is the symbol name.
static RUST_PATTERNS: &[(&str, SymbolKind)] = &[
    (
        r"^(?:pub(?:\([^)]*\))?\s+)?(?:async\s+)?fn\s+(\w+)",
        SymbolKind::Function,
    ),
    (
        r"^(?:pub(?:\([^)]*\))?\s+)?struct\s+(\w+)",
        SymbolKind::Struct,
    ),
    (
        r"^(?:pub(?:\([^)]*\))?\s+)?enum\s+(\w+)",
        SymbolKind::Enum,
    ),
    (
        r"^(?:pub(?:\([^)]*\))?\s+)?trait\s+(\w+)",
        SymbolKind::Trait,
    ),
    (
        r"^(?:pub(?:\([^)]*\))?\s+)?type\s+(\w+)",
        SymbolKind::Type,
    ),
    (
        r"^(?:pub(?:\([^)]*\))?\s+)?const\s+(\w+)",
        SymbolKind::Constant,
    ),
    (
        r"^(?:pub(?:\([^)]*\))?\s+)?mod\s+(\w+)",
        SymbolKind::Module,
    ),
];

static TS_PATTERNS: &[(&str, SymbolKind)] = &[
    (
        r"^(?:export\s+)?(?:async\s+)?function\s+(\w+)",
        SymbolKind::Function,
    ),
    (
        r"^(?:export\s+)?(?:default\s+)?class\s+(\w+)",
        SymbolKind::Class,
    ),
    (
        r"^(?:export\s+)?interface\s+(\w+)",
        SymbolKind::Interface,
    ),
    (
        r"^(?:export\s+)?type\s+(\w+)\s*=",
        SymbolKind::Type,
    ),
    (
        r"^(?:export\s+)?(?:const|let|var)\s+(\w+)\s*[:=]",
        SymbolKind::Variable,
    ),
    (
        r"^\s+(?:async\s+)?(\w+)\s*\(",
        SymbolKind::Method,
    ),
];

static PYTHON_PATTERNS: &[(&str, SymbolKind)] = &[
    (r"^def\s+(\w+)\s*\(", SymbolKind::Function),
    (r"^\s{4}def\s+(\w+)\s*\(", SymbolKind::Method),
    (r"^class\s+(\w+)", SymbolKind::Class),
    (r"^(\w+)\s*=\s*", SymbolKind::Variable),
];

static GO_PATTERNS: &[(&str, SymbolKind)] = &[
    (r"^func\s+(\w+)\s*\(", SymbolKind::Function),
    (r"^func\s+\(\w+\s+\*?\w+\)\s+(\w+)\s*\(", SymbolKind::Method),
    (r"^type\s+(\w+)\s+struct", SymbolKind::Struct),
    (r"^type\s+(\w+)\s+interface", SymbolKind::Interface),
    (r"^type\s+(\w+)", SymbolKind::Type),
    (r"^const\s+(\w+)", SymbolKind::Constant),
    (r"^var\s+(\w+)", SymbolKind::Variable),
];

#[cfg(test)]
mod tests {
    use super::*;

    // ── Language ──────────────────────────────────────────────────────────

    #[test]
    fn language_from_extension() {
        assert_eq!(Language::from_extension("rs"), Language::Rust);
        assert_eq!(Language::from_extension("ts"), Language::TypeScript);
        assert_eq!(Language::from_extension("tsx"), Language::TypeScript);
        assert_eq!(Language::from_extension("js"), Language::JavaScript);
        assert_eq!(Language::from_extension("jsx"), Language::JavaScript);
        assert_eq!(Language::from_extension("mjs"), Language::JavaScript);
        assert_eq!(Language::from_extension("py"), Language::Python);
        assert_eq!(Language::from_extension("pyi"), Language::Python);
        assert_eq!(Language::from_extension("go"), Language::Go);
        assert_eq!(Language::from_extension("txt"), Language::Unknown);
        assert_eq!(Language::from_extension("md"), Language::Unknown);
    }

    #[test]
    fn language_from_extension_case_insensitive() {
        assert_eq!(Language::from_extension("RS"), Language::Rust);
        assert_eq!(Language::from_extension("Py"), Language::Python);
    }

    #[test]
    fn language_is_source() {
        assert!(Language::Rust.is_source());
        assert!(Language::TypeScript.is_source());
        assert!(Language::Python.is_source());
        assert!(Language::Go.is_source());
        assert!(!Language::Unknown.is_source());
    }

    #[test]
    fn language_as_str() {
        assert_eq!(Language::Rust.as_str(), "rust");
        assert_eq!(Language::TypeScript.as_str(), "typescript");
        assert_eq!(Language::JavaScript.as_str(), "javascript");
        assert_eq!(Language::Python.as_str(), "python");
        assert_eq!(Language::Go.as_str(), "go");
        assert_eq!(Language::Unknown.as_str(), "unknown");
    }

    // ── SymbolKind ───────────────────────────────────────────────────────

    #[test]
    fn symbol_kind_as_str() {
        assert_eq!(SymbolKind::Function.as_str(), "fn");
        assert_eq!(SymbolKind::Method.as_str(), "method");
        assert_eq!(SymbolKind::Struct.as_str(), "struct");
        assert_eq!(SymbolKind::Enum.as_str(), "enum");
        assert_eq!(SymbolKind::Trait.as_str(), "trait");
        assert_eq!(SymbolKind::Class.as_str(), "class");
        assert_eq!(SymbolKind::Interface.as_str(), "interface");
        assert_eq!(SymbolKind::Constant.as_str(), "const");
        assert_eq!(SymbolKind::Type.as_str(), "type");
        assert_eq!(SymbolKind::Module.as_str(), "mod");
        assert_eq!(SymbolKind::Variable.as_str(), "var");
    }

    // ── SymbolInfo::format_ref ───────────────────────────────────────────

    #[test]
    fn format_ref() {
        let info = SymbolInfo {
            name: "main".into(),
            kind: SymbolKind::Function,
            file: PathBuf::from("src/main.rs"),
            line: 42,
            signature: "pub fn main()".into(),
            language: Language::Rust,
        };
        let r = info.format_ref();
        assert!(r.contains("[rust]"));
        assert!(r.contains("fn main"));
        assert!(r.contains("src/main.rs:42"));
    }

    // ── extract_symbols (Rust) ───────────────────────────────────────────

    #[test]
    fn extract_rust_function() {
        let content = "pub fn hello() {\n    println!(\"hi\");\n}\n";
        let syms = extract_symbols(&PathBuf::from("lib.rs"), content, &Language::Rust);
        assert_eq!(syms.len(), 1);
        assert_eq!(syms[0].name, "hello");
        assert!(matches!(syms[0].kind, SymbolKind::Function));
        assert_eq!(syms[0].line, 1);
    }

    #[test]
    fn extract_rust_struct_enum_trait() {
        let content = "pub struct Foo {}\npub enum Bar {}\npub trait Baz {}\n";
        let syms = extract_symbols(&PathBuf::from("a.rs"), content, &Language::Rust);
        let names: Vec<_> = syms.iter().map(|s| s.name.as_str()).collect();
        assert!(names.contains(&"Foo"));
        assert!(names.contains(&"Bar"));
        assert!(names.contains(&"Baz"));
    }

    #[test]
    fn extract_rust_async_fn() {
        let content = "pub async fn fetch() {}\n";
        let syms = extract_symbols(&PathBuf::from("a.rs"), content, &Language::Rust);
        assert_eq!(syms.len(), 1);
        assert_eq!(syms[0].name, "fetch");
    }

    #[test]
    fn extract_rust_const_type_mod() {
        let content = "pub const MAX: usize = 10;\npub type Id = u64;\npub mod utils;\n";
        let syms = extract_symbols(&PathBuf::from("a.rs"), content, &Language::Rust);
        let names: Vec<_> = syms.iter().map(|s| s.name.as_str()).collect();
        assert!(names.contains(&"MAX"));
        assert!(names.contains(&"Id"));
        assert!(names.contains(&"utils"));
    }

    // ── extract_symbols (Python) ─────────────────────────────────────────

    #[test]
    fn extract_python_symbols() {
        let content = "def hello():\n    pass\n\nclass Foo:\n    def method(self):\n        pass\n";
        let syms = extract_symbols(&PathBuf::from("main.py"), content, &Language::Python);
        let names: Vec<_> = syms.iter().map(|s| s.name.as_str()).collect();
        assert!(names.contains(&"hello"));
        assert!(names.contains(&"Foo"));
        assert!(names.contains(&"method"));
    }

    // ── extract_symbols (Go) ─────────────────────────────────────────────

    #[test]
    fn extract_go_symbols() {
        let content = "func main() {}\ntype Config struct {}\nconst MaxRetries = 3\n";
        let syms = extract_symbols(&PathBuf::from("main.go"), content, &Language::Go);
        let names: Vec<_> = syms.iter().map(|s| s.name.as_str()).collect();
        assert!(names.contains(&"main"));
        assert!(names.contains(&"Config"));
        assert!(names.contains(&"MaxRetries"));
    }

    // ── extract_symbols (TypeScript) ─────────────────────────────────────

    #[test]
    fn extract_ts_symbols() {
        let content = "export function greet() {}\nexport class App {}\nexport interface Props {}\nexport type Id = string;\n";
        let syms = extract_symbols(&PathBuf::from("app.ts"), content, &Language::TypeScript);
        let names: Vec<_> = syms.iter().map(|s| s.name.as_str()).collect();
        assert!(names.contains(&"greet"));
        assert!(names.contains(&"App"));
        assert!(names.contains(&"Props"));
        assert!(names.contains(&"Id"));
    }

    // ── extract_symbols (Unknown) ────────────────────────────────────────

    #[test]
    fn extract_unknown_language_empty() {
        let content = "fn main() {}\n";
        let syms = extract_symbols(&PathBuf::from("a.txt"), content, &Language::Unknown);
        assert!(syms.is_empty());
    }

    // ── deduplication ────────────────────────────────────────────────────

    #[test]
    fn extract_no_duplicates() {
        // "pub fn" matches both the function pattern and could potentially
        // match other patterns; ensure no duplicate entries
        let content = "pub fn process() {}\n";
        let syms = extract_symbols(&PathBuf::from("a.rs"), content, &Language::Rust);
        let count = syms.iter().filter(|s| s.name == "process").count();
        assert_eq!(count, 1);
    }
}
