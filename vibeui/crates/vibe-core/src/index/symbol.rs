//! Language-aware symbol extraction using regex patterns.

use regex::Regex;
use serde::{Deserialize, Serialize};
use std::path::PathBuf;

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
pub fn extract_symbols(path: &PathBuf, content: &str, language: &Language) -> Vec<SymbolInfo> {
    match language {
        Language::Rust => extract_with_patterns(path, content, Language::Rust, RUST_PATTERNS),
        Language::TypeScript => extract_with_patterns(path, content, Language::TypeScript, TS_PATTERNS),
        Language::JavaScript => extract_with_patterns(path, content, Language::JavaScript, TS_PATTERNS),
        Language::Python => extract_with_patterns(path, content, Language::Python, PYTHON_PATTERNS),
        Language::Go => extract_with_patterns(path, content, Language::Go, GO_PATTERNS),
        Language::Unknown => vec![],
    }
}

fn extract_with_patterns<'a>(
    path: &PathBuf,
    content: &str,
    language: Language,
    patterns: &'a [(&'static str, SymbolKind)],
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
                            file: path.clone(),
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
