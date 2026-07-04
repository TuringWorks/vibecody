//! Symbol model — the rich node payload for code symbols.

use serde::{Deserialize, Serialize};

/// Kind of source-code symbol.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum SymbolKind {
    /// A free function.
    Function,
    /// A method associated with a type.
    Method,
    /// A class (OOP).
    Class,
    /// A struct (Rust/C/C++).
    Struct,
    /// An enum.
    Enum,
    /// An interface / protocol.
    Interface,
    /// A trait (Rust) or mixin.
    Trait,
    /// A module / namespace.
    Module,
    /// A constant.
    Constant,
    /// A variable / field.
    Variable,
    /// A type alias.
    TypeAlias,
    /// A macro.
    Macro,
}

impl SymbolKind {
    /// Lowercase stable string used for storage + filtering.
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Function => "function",
            Self::Method => "method",
            Self::Class => "class",
            Self::Struct => "struct",
            Self::Enum => "enum",
            Self::Interface => "interface",
            Self::Trait => "trait",
            Self::Module => "module",
            Self::Constant => "constant",
            Self::Variable => "variable",
            Self::TypeAlias => "type_alias",
            Self::Macro => "macro",
        }
    }
}

/// Visibility of a symbol.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Visibility {
    /// `pub` / exported.
    Public,
    /// Private to the declaring scope.
    Private,
    /// `protected` (OOP).
    Protected,
    /// Internal to a package / assembly.
    Internal,
    /// `pub(crate)` (Rust).
    Crate,
}

/// Source language detected from file extension. Kept broad so consumers can map to
/// their own tooling; only Rust/TypeScript/Python/Go are parsed by the tree-sitter
/// backbone in v0.1 — others fall back to coarse file-level nodes.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Language {
    /// Rust
    Rust,
    /// TypeScript / TSX
    TypeScript,
    /// JavaScript
    JavaScript,
    /// Python
    Python,
    /// Go
    Go,
    /// Java
    Java,
    /// C#
    CSharp,
    /// C++
    Cpp,
    /// C
    C,
    /// Ruby
    Ruby,
    /// PHP
    Php,
    /// Swift
    Swift,
    /// Kotlin
    Kotlin,
    /// Scala
    Scala,
    /// Haskell
    Haskell,
    /// Elixir
    Elixir,
    /// Dart
    Dart,
    /// Zig
    Zig,
    /// Lua
    Lua,
    /// Bash
    Bash,
    /// Unknown / unsupported by the active parser tier.
    Unknown,
}

impl Language {
    /// Detect a language from a file extension.
    pub fn from_extension(ext: &str) -> Self {
        match ext.to_ascii_lowercase().as_str() {
            "rs" => Self::Rust,
            "ts" | "tsx" => Self::TypeScript,
            "js" | "jsx" | "mjs" | "cjs" => Self::JavaScript,
            "py" | "pyi" => Self::Python,
            "go" => Self::Go,
            "java" => Self::Java,
            "cs" => Self::CSharp,
            "cpp" | "cc" | "cxx" | "hpp" | "hh" => Self::Cpp,
            "c" | "h" => Self::C,
            "rb" => Self::Ruby,
            "php" => Self::Php,
            "swift" => Self::Swift,
            "kt" | "kts" => Self::Kotlin,
            "scala" | "sc" => Self::Scala,
            "hs" => Self::Haskell,
            "ex" | "exs" => Self::Elixir,
            "dart" => Self::Dart,
            "zig" => Self::Zig,
            "lua" => Self::Lua,
            "sh" | "bash" => Self::Bash,
            _ => Self::Unknown,
        }
    }

    /// Stable lowercase string.
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Rust => "rust",
            Self::TypeScript => "typescript",
            Self::JavaScript => "javascript",
            Self::Python => "python",
            Self::Go => "go",
            Self::Java => "java",
            Self::CSharp => "csharp",
            Self::Cpp => "cpp",
            Self::C => "c",
            Self::Ruby => "ruby",
            Self::Php => "php",
            Self::Swift => "swift",
            Self::Kotlin => "kotlin",
            Self::Scala => "scala",
            Self::Haskell => "haskell",
            Self::Elixir => "elixir",
            Self::Dart => "dart",
            Self::Zig => "zig",
            Self::Lua => "lua",
            Self::Bash => "bash",
            Self::Unknown => "unknown",
        }
    }

    /// Whether the tree-sitter backbone can parse this language in v0.1.
    pub fn supported_by_treesitter(self) -> bool {
        matches!(self, Self::Rust | Self::TypeScript | Self::Python | Self::Go)
    }
}

/// A source-code symbol with full metadata. Forms the rich payload of a graph node.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Symbol {
    /// Unqualified name, e.g. `build_temp_provider`.
    pub name: String,
    /// Kind (function / struct / trait ...).
    pub kind: SymbolKind,
    /// Fully-qualified name where known, e.g. `kodegraph::builder::CodeGraphBuilder::build`.
    pub qualified_name: String,
    /// Absolute or workspace-relative file path.
    pub file_path: String,
    /// 1-based starting line.
    pub line_start: usize,
    /// 1-based ending line (inclusive).
    pub line_end: usize,
    /// Signature text, if extractable.
    pub signature: Option<String>,
    /// Doc comment (first paragraph) if present.
    pub doc_comment: Option<String>,
    /// Visibility.
    pub visibility: Visibility,
    /// Source language.
    pub language: Language,
}

impl Symbol {
    /// Build a stable, deterministic node key (`file_path:qualified_name`).
    pub fn node_key(&self) -> String {
        format!("{}:{}", self.file_path, self.qualified_name)
    }
}