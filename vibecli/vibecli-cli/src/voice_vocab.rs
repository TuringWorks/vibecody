#![allow(dead_code)]
//! Codebase-vocabulary voice recognition enhancement.
//!
//! Extracts symbols (functions, structs, constants, …) from Rust/Python/JS
//! source text, builds phonetic hints from camelCase / snake_case names, and
//! produces a `HotWordsConfig` that can be injected into a Whisper-compatible
//! voice recogniser to improve transcription accuracy on domain-specific terms.

use serde::{Deserialize, Serialize};
use std::collections::HashMap;

// ─── Enums ───────────────────────────────────────────────────────────────────

/// The kind of source-code symbol.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum SymbolKind {
    Function,
    Class,
    Struct,
    Constant,
    Variable,
    File,
    Directory,
    Module,
    TypeAlias,
}

impl std::fmt::Display for SymbolKind {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Function => write!(f, "fn"),
            Self::Class => write!(f, "class"),
            Self::Struct => write!(f, "struct"),
            Self::Constant => write!(f, "const"),
            Self::Variable => write!(f, "var"),
            Self::File => write!(f, "file"),
            Self::Directory => write!(f, "dir"),
            Self::Module => write!(f, "mod"),
            Self::TypeAlias => write!(f, "type"),
        }
    }
}

// ─── Structs ─────────────────────────────────────────────────────────────────

/// A symbol extracted from a codebase with usage frequency.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VocabSymbol {
    pub name: String,
    pub kind: SymbolKind,
    pub frequency: u32,
    pub file_path: String,
    /// Human-readable phonetic hint, e.g. "calculate user session timeout".
    pub phonetic_hint: Option<String>,
}

impl VocabSymbol {
    fn new(name: impl Into<String>, kind: SymbolKind, file_path: impl Into<String>) -> Self {
        let name = name.into();
        let phonetic_hint = Some(normalize_symbol_name(&name));
        Self {
            name,
            kind,
            frequency: 1,
            file_path: file_path.into(),
            phonetic_hint,
        }
    }
}

/// Configuration ready for injection into a voice recogniser.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HotWordsConfig {
    /// All top symbols joined with a space — suitable as Whisper `initial_prompt`.
    pub initial_prompt: String,
    /// Top symbols as individual strings.
    pub hotwords: Vec<String>,
}

/// Word-error-rate improvement metrics after vocab injection.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WerMetrics {
    pub before_wer: f32,
    pub after_wer: f32,
    pub improvement_pct: f32,
    pub sample_count: u32,
}

impl WerMetrics {
    pub fn compute(before: f32, after: f32) -> Self {
        let improvement_pct = if before > 0.0 {
            (before - after) / before * 100.0
        } else {
            0.0
        };
        Self {
            before_wer: before,
            after_wer: after,
            improvement_pct,
            sample_count: 0,
        }
    }
}

/// Aggregate statistics for an extracted vocabulary.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VocabStats {
    pub total_symbols: usize,
    pub unique_kinds: usize,
    pub top_symbol: Option<String>,
    pub avg_frequency: f32,
}

impl VocabStats {
    pub fn from_symbols(symbols: &[VocabSymbol]) -> Self {
        let total_symbols = symbols.len();
        let unique_kinds = symbols
            .iter()
            .map(|s| s.kind.to_string())
            .collect::<std::collections::HashSet<_>>()
            .len();
        let top_symbol = symbols
            .iter()
            .max_by_key(|s| s.frequency)
            .map(|s| s.name.clone());
        let avg_frequency = if total_symbols == 0 {
            0.0
        } else {
            symbols.iter().map(|s| s.frequency as f32).sum::<f32>() / total_symbols as f32
        };
        Self {
            total_symbols,
            unique_kinds,
            top_symbol,
            avg_frequency,
        }
    }
}

// ─── Conversion helpers ───────────────────────────────────────────────────────

/// Convert a camelCase identifier to space-separated words.
/// `"calculateUserTimeout"` → `"calculate user timeout"`
pub fn camel_to_words(name: &str) -> String {
    if name.is_empty() {
        return String::new();
    }
    let mut words = Vec::new();
    let mut current = String::new();

    for (i, ch) in name.chars().enumerate() {
        if i > 0 && ch.is_uppercase() {
            if !current.is_empty() {
                words.push(current.to_lowercase());
                current = String::new();
            }
        }
        current.push(ch);
    }
    if !current.is_empty() {
        words.push(current.to_lowercase());
    }
    words.join(" ")
}

/// Convert a snake_case identifier to space-separated words.
/// `"calculate_user_timeout"` → `"calculate user timeout"`
pub fn snake_to_words(name: &str) -> String {
    name.split('_')
        .filter(|p| !p.is_empty())
        .collect::<Vec<_>>()
        .join(" ")
        .to_lowercase()
}

/// Detect whether `name` is camelCase or snake_case and apply the appropriate
/// conversion.  Falls back to returning the name unchanged if neither style is
/// detected.
pub fn normalize_symbol_name(name: &str) -> String {
    if name.contains('_') {
        // Treat as snake_case (even if there are also uppercase letters)
        snake_to_words(name)
    } else if name.chars().any(|c| c.is_uppercase()) {
        camel_to_words(name)
    } else {
        name.to_lowercase()
    }
}

// ─── VocabExtractor ──────────────────────────────────────────────────────────

/// Scans source text for symbol declarations and accumulates a frequency-ranked
/// vocabulary.
pub struct VocabExtractor {
    /// `name` → `VocabSymbol`
    symbols_map: HashMap<String, VocabSymbol>,
}

impl VocabExtractor {
    pub fn new() -> Self {
        Self {
            symbols_map: HashMap::new(),
        }
    }

    /// Scan `source` for symbol declarations and update the internal table.
    ///
    /// Recognises:
    /// - `fn NAME` / `pub fn NAME`
    /// - `struct NAME` / `pub struct NAME`
    /// - `const NAME` / `pub const NAME`
    /// - `type NAME` / `pub type NAME`
    /// - `impl NAME` (captures the struct name used in the impl)
    pub fn extract_from_source(&mut self, source: &str, file_path: &str) {
        for line in source.lines() {
            let trimmed = line.trim();
            if let Some(name) = Self::try_extract(trimmed, &["pub fn ", "fn "], file_path) {
                self.upsert(name, SymbolKind::Function, file_path);
            } else if let Some(name) =
                Self::try_extract(trimmed, &["pub struct ", "struct "], file_path)
            {
                self.upsert(name, SymbolKind::Struct, file_path);
            } else if let Some(name) =
                Self::try_extract(trimmed, &["pub const ", "const "], file_path)
            {
                self.upsert(name, SymbolKind::Constant, file_path);
            } else if let Some(name) =
                Self::try_extract(trimmed, &["pub type ", "type "], file_path)
            {
                self.upsert(name, SymbolKind::TypeAlias, file_path);
            } else if let Some(name) = Self::try_extract(trimmed, &["impl "], file_path) {
                // Only keep the impl target if it looks like a concrete type (starts uppercase)
                if name.chars().next().map(|c| c.is_uppercase()).unwrap_or(false) {
                    self.upsert(name, SymbolKind::Struct, file_path);
                }
            }
        }
    }

    /// Split `path` by `/`, `-`, and `_` and add each component as a Module
    /// symbol.
    pub fn extract_from_path_components(&mut self, path: &str) {
        let components: Vec<&str> = path
            .split(|c| c == '/' || c == '-' || c == '_')
            .filter(|p| !p.is_empty() && !p.contains('.'))
            .collect();
        for component in components {
            self.upsert(component.to_string(), SymbolKind::Module, path);
        }
    }

    /// All symbols sorted by frequency descending.
    pub fn symbols(&self) -> Vec<&VocabSymbol> {
        let mut v: Vec<&VocabSymbol> = self.symbols_map.values().collect();
        v.sort_by(|a, b| b.frequency.cmp(&a.frequency).then(a.name.cmp(&b.name)));
        v
    }

    /// Top `n` symbols by frequency.
    pub fn top_n(&self, n: usize) -> Vec<&VocabSymbol> {
        self.symbols().into_iter().take(n).collect()
    }

    pub fn symbol_count(&self) -> usize {
        self.symbols_map.len()
    }

    // ── private helpers ───────────────────────────────────────────────────

    /// Try each prefix in order; if the trimmed line starts with one, extract
    /// the identifier that follows (up to whitespace, `<`, `(`, `:`, `{`).
    fn try_extract(line: &str, prefixes: &[&str], _file_path: &str) -> Option<String> {
        for prefix in prefixes {
            if line.starts_with(prefix) {
                let rest = &line[prefix.len()..];
                let name: String = rest
                    .chars()
                    .take_while(|c| c.is_alphanumeric() || *c == '_')
                    .collect();
                if !name.is_empty() {
                    return Some(name);
                }
            }
        }
        None
    }

    fn upsert(&mut self, name: impl Into<String>, kind: SymbolKind, file_path: &str) {
        let name = name.into();
        if let Some(sym) = self.symbols_map.get_mut(&name) {
            sym.frequency += 1;
        } else {
            self.symbols_map
                .insert(name.clone(), VocabSymbol::new(name, kind, file_path));
        }
    }
}

impl Default for VocabExtractor {
    fn default() -> Self {
        Self::new()
    }
}

// ─── VocabInjector ───────────────────────────────────────────────────────────

/// Builds voice-recogniser configuration from an extracted vocabulary.
pub struct VocabInjector;

impl VocabInjector {
    pub fn new() -> Self {
        Self
    }

    /// Build a `HotWordsConfig` from the top `top_n` symbols.
    pub fn build_config(&self, symbols: &[&VocabSymbol], top_n: usize) -> HotWordsConfig {
        let top: Vec<&VocabSymbol> = symbols.iter().copied().take(top_n).collect();
        let hotwords: Vec<String> = top.iter().map(|s| s.name.clone()).collect();
        let initial_prompt = hotwords.join(" ");
        HotWordsConfig {
            initial_prompt,
            hotwords,
        }
    }

    /// Wrapper around `normalize_symbol_name`.
    pub fn phonetic_normalization(&self, name: &str) -> String {
        normalize_symbol_name(name)
    }
}

impl Default for VocabInjector {
    fn default() -> Self {
        Self::new()
    }
}

// ─── Tests ───────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    // ── camel_to_words ───────────────────────────────────────────────────

    #[test]
    fn test_camel_simple() {
        assert_eq!(camel_to_words("calculateUserTimeout"), "calculate user timeout");
    }

    #[test]
    fn test_camel_single_word() {
        assert_eq!(camel_to_words("hello"), "hello");
    }

    #[test]
    fn test_camel_all_caps_first() {
        assert_eq!(camel_to_words("MyStruct"), "my struct");
    }

    #[test]
    fn test_camel_empty_string() {
        assert_eq!(camel_to_words(""), "");
    }

    #[test]
    fn test_camel_two_words() {
        assert_eq!(camel_to_words("getUserName"), "get user name");
    }

    #[test]
    fn test_camel_long_name() {
        assert_eq!(
            camel_to_words("calculateUserSessionTimeout"),
            "calculate user session timeout"
        );
    }

    // ── snake_to_words ───────────────────────────────────────────────────

    #[test]
    fn test_snake_simple() {
        assert_eq!(snake_to_words("calculate_user_timeout"), "calculate user timeout");
    }

    #[test]
    fn test_snake_single_word() {
        assert_eq!(snake_to_words("hello"), "hello");
    }

    #[test]
    fn test_snake_double_underscore_ignored() {
        assert_eq!(snake_to_words("a__b"), "a b");
    }

    #[test]
    fn test_snake_empty() {
        assert_eq!(snake_to_words(""), "");
    }

    #[test]
    fn test_snake_uppercase_lowered() {
        assert_eq!(snake_to_words("MAX_RETRIES"), "max retries");
    }

    // ── normalize_symbol_name ────────────────────────────────────────────

    #[test]
    fn test_normalize_snake_detected() {
        assert_eq!(
            normalize_symbol_name("fetch_user_data"),
            "fetch user data"
        );
    }

    #[test]
    fn test_normalize_camel_detected() {
        assert_eq!(normalize_symbol_name("fetchUserData"), "fetch user data");
    }

    #[test]
    fn test_normalize_plain_lowercase() {
        assert_eq!(normalize_symbol_name("hello"), "hello");
    }

    #[test]
    fn test_normalize_const_name() {
        assert_eq!(normalize_symbol_name("MAX_BUFFER_SIZE"), "max buffer size");
    }

    // ── VocabExtractor::extract_from_source ──────────────────────────────

    #[test]
    fn test_extract_fn() {
        let mut ex = VocabExtractor::new();
        ex.extract_from_source("fn my_function(x: u32) {}", "main.rs");
        assert!(ex.symbols_map.contains_key("my_function"));
        assert_eq!(ex.symbols_map["my_function"].kind, SymbolKind::Function);
    }

    #[test]
    fn test_extract_pub_fn() {
        let mut ex = VocabExtractor::new();
        ex.extract_from_source("pub fn public_func() -> u32 { 0 }", "lib.rs");
        assert!(ex.symbols_map.contains_key("public_func"));
    }

    #[test]
    fn test_extract_struct() {
        let mut ex = VocabExtractor::new();
        ex.extract_from_source("struct MyStruct { x: u32 }", "types.rs");
        assert!(ex.symbols_map.contains_key("MyStruct"));
        assert_eq!(ex.symbols_map["MyStruct"].kind, SymbolKind::Struct);
    }

    #[test]
    fn test_extract_pub_struct() {
        let mut ex = VocabExtractor::new();
        ex.extract_from_source("pub struct Config { }", "config.rs");
        assert!(ex.symbols_map.contains_key("Config"));
    }

    #[test]
    fn test_extract_const() {
        let mut ex = VocabExtractor::new();
        ex.extract_from_source("const MAX_SIZE: usize = 100;", "constants.rs");
        assert!(ex.symbols_map.contains_key("MAX_SIZE"));
        assert_eq!(ex.symbols_map["MAX_SIZE"].kind, SymbolKind::Constant);
    }

    #[test]
    fn test_extract_pub_const() {
        let mut ex = VocabExtractor::new();
        ex.extract_from_source("pub const VERSION: &str = \"1.0\";", "lib.rs");
        assert!(ex.symbols_map.contains_key("VERSION"));
    }

    #[test]
    fn test_extract_type_alias() {
        let mut ex = VocabExtractor::new();
        ex.extract_from_source("type Result<T> = std::result::Result<T, Error>;", "lib.rs");
        assert!(ex.symbols_map.contains_key("Result"));
        assert_eq!(ex.symbols_map["Result"].kind, SymbolKind::TypeAlias);
    }

    #[test]
    fn test_extract_impl_captures_struct_name() {
        let mut ex = VocabExtractor::new();
        ex.extract_from_source("impl MyEngine {", "engine.rs");
        assert!(ex.symbols_map.contains_key("MyEngine"));
    }

    #[test]
    fn test_extract_impl_trait_skipped_if_lowercase() {
        let mut ex = VocabExtractor::new();
        // "impl fmt::Display for Foo" — the token after "impl " is "fmt" (lowercase), skip
        ex.extract_from_source("impl fmt::Display for MyStruct {", "lib.rs");
        // "fmt" starts lowercase — should not be inserted
        assert!(!ex.symbols_map.contains_key("fmt"));
    }

    #[test]
    fn test_frequency_accumulation() {
        let mut ex = VocabExtractor::new();
        let src = "fn my_func() {}\nfn my_func() {}";
        ex.extract_from_source(src, "lib.rs");
        assert_eq!(ex.symbols_map["my_func"].frequency, 2);
    }

    #[test]
    fn test_symbol_count() {
        let mut ex = VocabExtractor::new();
        ex.extract_from_source("fn a() {}\nfn b() {}\nstruct C {}", "lib.rs");
        assert_eq!(ex.symbol_count(), 3);
    }

    #[test]
    fn test_symbols_sorted_by_frequency_desc() {
        let mut ex = VocabExtractor::new();
        let src = "fn alpha() {}\nfn beta() {}\nfn beta() {}";
        ex.extract_from_source(src, "lib.rs");
        let syms = ex.symbols();
        assert_eq!(syms[0].name, "beta");
        assert_eq!(syms[0].frequency, 2);
    }

    #[test]
    fn test_top_n_returns_correct_count() {
        let mut ex = VocabExtractor::new();
        ex.extract_from_source("fn a(){}\nfn b(){}\nfn c(){}\nfn d(){}", "lib.rs");
        assert_eq!(ex.top_n(2).len(), 2);
    }

    #[test]
    fn test_top_n_larger_than_count() {
        let mut ex = VocabExtractor::new();
        ex.extract_from_source("fn a(){}", "lib.rs");
        assert_eq!(ex.top_n(100).len(), 1);
    }

    // ── extract_from_path_components ─────────────────────────────────────

    #[test]
    fn test_path_components_slash() {
        let mut ex = VocabExtractor::new();
        ex.extract_from_path_components("src/utils/helpers");
        assert!(ex.symbols_map.contains_key("src"));
        assert!(ex.symbols_map.contains_key("utils"));
        assert!(ex.symbols_map.contains_key("helpers"));
    }

    #[test]
    fn test_path_components_hyphen() {
        let mut ex = VocabExtractor::new();
        ex.extract_from_path_components("my-project-name");
        assert!(ex.symbols_map.contains_key("my"));
        assert!(ex.symbols_map.contains_key("project"));
        assert!(ex.symbols_map.contains_key("name"));
    }

    #[test]
    fn test_path_components_underscore() {
        let mut ex = VocabExtractor::new();
        ex.extract_from_path_components("voice_vocab");
        assert!(ex.symbols_map.contains_key("voice"));
        assert!(ex.symbols_map.contains_key("vocab"));
    }

    // ── HotWordsConfig ───────────────────────────────────────────────────

    #[test]
    fn test_build_config_initial_prompt_joined() {
        let injector = VocabInjector::new();
        let a = VocabSymbol::new("alpha", SymbolKind::Function, "lib.rs");
        let b = VocabSymbol::new("beta", SymbolKind::Struct, "lib.rs");
        let refs: Vec<&VocabSymbol> = vec![&a, &b];
        let cfg = injector.build_config(&refs, 2);
        assert_eq!(cfg.initial_prompt, "alpha beta");
        assert_eq!(cfg.hotwords, vec!["alpha", "beta"]);
    }

    #[test]
    fn test_build_config_respects_top_n() {
        let injector = VocabInjector::new();
        let a = VocabSymbol::new("a", SymbolKind::Function, "lib.rs");
        let b = VocabSymbol::new("b", SymbolKind::Function, "lib.rs");
        let c = VocabSymbol::new("c", SymbolKind::Function, "lib.rs");
        let refs: Vec<&VocabSymbol> = vec![&a, &b, &c];
        let cfg = injector.build_config(&refs, 2);
        assert_eq!(cfg.hotwords.len(), 2);
    }

    #[test]
    fn test_phonetic_normalization_snake() {
        let inj = VocabInjector::new();
        assert_eq!(inj.phonetic_normalization("my_func"), "my func");
    }

    #[test]
    fn test_phonetic_normalization_camel() {
        let inj = VocabInjector::new();
        assert_eq!(inj.phonetic_normalization("myFunc"), "my func");
    }

    // ── WerMetrics ───────────────────────────────────────────────────────

    #[test]
    fn test_wer_metrics_improvement() {
        let m = WerMetrics::compute(0.20, 0.10);
        assert!((m.improvement_pct - 50.0).abs() < 0.01);
    }

    #[test]
    fn test_wer_metrics_no_improvement() {
        let m = WerMetrics::compute(0.20, 0.20);
        assert!((m.improvement_pct - 0.0).abs() < 0.01);
    }

    #[test]
    fn test_wer_metrics_zero_before_no_panic() {
        let m = WerMetrics::compute(0.0, 0.0);
        assert_eq!(m.improvement_pct, 0.0);
    }

    #[test]
    fn test_wer_metrics_fields() {
        let m = WerMetrics::compute(0.40, 0.10);
        assert!((m.before_wer - 0.40).abs() < 0.001);
        assert!((m.after_wer - 0.10).abs() < 0.001);
        assert!((m.improvement_pct - 75.0).abs() < 0.01);
    }

    // ── VocabStats ───────────────────────────────────────────────────────

    #[test]
    fn test_vocab_stats_empty() {
        let s = VocabStats::from_symbols(&[]);
        assert_eq!(s.total_symbols, 0);
        assert_eq!(s.avg_frequency, 0.0);
        assert!(s.top_symbol.is_none());
    }

    #[test]
    fn test_vocab_stats_total() {
        let syms = vec![
            VocabSymbol::new("a", SymbolKind::Function, "f.rs"),
            VocabSymbol::new("b", SymbolKind::Struct, "f.rs"),
        ];
        let s = VocabStats::from_symbols(&syms);
        assert_eq!(s.total_symbols, 2);
    }

    #[test]
    fn test_vocab_stats_unique_kinds() {
        let syms = vec![
            VocabSymbol::new("a", SymbolKind::Function, "f.rs"),
            VocabSymbol::new("b", SymbolKind::Function, "f.rs"),
            VocabSymbol::new("c", SymbolKind::Struct, "f.rs"),
        ];
        let s = VocabStats::from_symbols(&syms);
        assert_eq!(s.unique_kinds, 2);
    }

    #[test]
    fn test_vocab_stats_top_symbol_highest_frequency() {
        let mut a = VocabSymbol::new("a", SymbolKind::Function, "f.rs");
        let b = VocabSymbol::new("b", SymbolKind::Function, "f.rs");
        a.frequency = 5;
        let syms = vec![a, b];
        let s = VocabStats::from_symbols(&syms);
        assert_eq!(s.top_symbol, Some("a".to_string()));
    }

    #[test]
    fn test_vocab_stats_avg_frequency() {
        let mut a = VocabSymbol::new("a", SymbolKind::Function, "f.rs");
        let mut b = VocabSymbol::new("b", SymbolKind::Function, "f.rs");
        a.frequency = 4;
        b.frequency = 2;
        let syms = vec![a, b];
        let s = VocabStats::from_symbols(&syms);
        assert!((s.avg_frequency - 3.0).abs() < 0.01);
    }

    #[test]
    fn test_phonetic_hint_set_on_new() {
        let sym = VocabSymbol::new("calculateUserTimeout", SymbolKind::Function, "lib.rs");
        assert_eq!(
            sym.phonetic_hint,
            Some("calculate user timeout".to_string())
        );
    }
}
