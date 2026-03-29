//! VS Code extension compatibility layer for VibeCody.
//!
//! Loads a subset of VS Code extensions (TextMate grammars, themes, snippets,
//! language configurations) in VibeUI alongside native WASM extensions. Closes
//! the gap vs Trae/PearAI which run standard `.vsix` extensions.
//!
//! REPL commands: `/extension search|install|uninstall|list|enable|disable|theme`

use std::collections::HashMap;

// === Error ===

#[derive(Debug, Clone, PartialEq)]
pub enum CompatError {
    ExtensionNotFound,
    UnsupportedCategory,
    ParseError(String),
    InstallFailed(String),
    MaxExtensionsReached,
    MarketplaceError(String),
    DuplicateExtension,
    InvalidManifest(String),
}

impl std::fmt::Display for CompatError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::ExtensionNotFound => write!(f, "extension not found"),
            Self::UnsupportedCategory => write!(f, "unsupported extension category"),
            Self::ParseError(msg) => write!(f, "parse error: {msg}"),
            Self::InstallFailed(msg) => write!(f, "install failed: {msg}"),
            Self::MaxExtensionsReached => write!(f, "maximum extensions limit reached"),
            Self::MarketplaceError(msg) => write!(f, "marketplace error: {msg}"),
            Self::DuplicateExtension => write!(f, "extension already installed"),
            Self::InvalidManifest(msg) => write!(f, "invalid manifest: {msg}"),
        }
    }
}

// === Enums ===

#[derive(Debug, Clone, PartialEq)]
pub enum ExtensionCategory {
    LanguageGrammar,
    ColorTheme,
    Snippet,
    LanguageConfiguration,
    IconTheme,
    KeybindingSet,
}

impl ExtensionCategory {
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::LanguageGrammar => "language-grammar",
            Self::ColorTheme => "color-theme",
            Self::Snippet => "snippet",
            Self::LanguageConfiguration => "language-configuration",
            Self::IconTheme => "icon-theme",
            Self::KeybindingSet => "keybinding-set",
        }
    }
}

impl std::fmt::Display for ExtensionCategory {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.as_str())
    }
}

#[derive(Debug, Clone, PartialEq)]
pub enum FileType {
    TmLanguage,
    TmTheme,
    SnippetJson,
    LanguageConfig,
    PackageJson,
    Other,
}

impl FileType {
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::TmLanguage => "tmLanguage",
            Self::TmTheme => "tmTheme",
            Self::SnippetJson => "snippets.json",
            Self::LanguageConfig => "language-configuration.json",
            Self::PackageJson => "package.json",
            Self::Other => "other",
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub enum ThemeType {
    Light,
    Dark,
    HighContrast,
}

impl std::fmt::Display for ThemeType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Light => write!(f, "light"),
            Self::Dark => write!(f, "dark"),
            Self::HighContrast => write!(f, "high-contrast"),
        }
    }
}

// === Data Structures ===

#[derive(Debug, Clone)]
pub struct CompatConfig {
    pub extensions_dir: String,
    pub marketplace_url: String,
    pub max_extensions: usize,
    pub supported_categories: Vec<ExtensionCategory>,
}

impl Default for CompatConfig {
    fn default() -> Self {
        Self {
            extensions_dir: ".vibecody/extensions".to_string(),
            marketplace_url: "https://marketplace.visualstudio.com".to_string(),
            max_extensions: 100,
            supported_categories: vec![
                ExtensionCategory::LanguageGrammar,
                ExtensionCategory::ColorTheme,
                ExtensionCategory::Snippet,
                ExtensionCategory::LanguageConfiguration,
                ExtensionCategory::IconTheme,
                ExtensionCategory::KeybindingSet,
            ],
        }
    }
}

#[derive(Debug, Clone)]
pub struct ExtensionFile {
    pub path: String,
    pub file_type: FileType,
    pub content: Option<String>,
}

#[derive(Debug, Clone)]
pub struct VsCodeExtension {
    pub id: String,
    pub name: String,
    pub publisher: String,
    pub version: String,
    pub description: String,
    pub categories: Vec<ExtensionCategory>,
    pub files: Vec<ExtensionFile>,
    pub installed_at: u64,
    pub enabled: bool,
    pub size_bytes: u64,
}

#[derive(Debug, Clone)]
pub struct TextMateGrammar {
    pub scope_name: String,
    pub language_id: String,
    pub file_extensions: Vec<String>,
    pub patterns: Vec<GrammarPattern>,
}

#[derive(Debug, Clone)]
pub struct GrammarPattern {
    pub name: String,
    pub match_pattern: Option<String>,
    pub begin: Option<String>,
    pub end: Option<String>,
    pub captures: Vec<String>,
}

#[derive(Debug, Clone)]
pub struct ThemeDefinition {
    pub name: String,
    pub theme_type: ThemeType,
    pub colors: Vec<ThemeColor>,
    pub token_colors: Vec<TokenColor>,
}

#[derive(Debug, Clone)]
pub struct ThemeColor {
    pub scope: String,
    pub foreground: Option<String>,
    pub background: Option<String>,
    pub font_style: Option<String>,
}

#[derive(Debug, Clone)]
pub struct TokenColor {
    pub name: String,
    pub scope: Vec<String>,
    pub foreground: Option<String>,
    pub font_style: Option<String>,
}

#[derive(Debug, Clone)]
pub struct SnippetDefinition {
    pub name: String,
    pub prefix: String,
    pub body: Vec<String>,
    pub description: String,
    pub language: String,
}

#[derive(Debug, Clone)]
pub struct LanguageConfig {
    pub language_id: String,
    pub extensions: Vec<String>,
    pub comment_line: Option<String>,
    pub comment_block_start: Option<String>,
    pub comment_block_end: Option<String>,
    pub brackets: Vec<(String, String)>,
    pub auto_closing_pairs: Vec<(String, String)>,
}

#[derive(Debug, Clone)]
pub struct MarketplaceSearchResult {
    pub extensions: Vec<MarketplaceEntry>,
    pub total_count: usize,
}

#[derive(Debug, Clone)]
pub struct MarketplaceEntry {
    pub id: String,
    pub name: String,
    pub publisher: String,
    pub description: String,
    pub version: String,
    pub install_count: u64,
    pub rating: f32,
    pub categories: Vec<String>,
}

#[derive(Debug, Clone)]
pub struct CompatStats {
    pub total: usize,
    pub enabled: usize,
    pub by_category: HashMap<String, usize>,
    pub total_size: u64,
}

// === Manager ===

pub struct ExtensionCompatManager {
    config: CompatConfig,
    extensions: Vec<VsCodeExtension>,
    grammars: Vec<TextMateGrammar>,
    themes: Vec<ThemeDefinition>,
    snippets: Vec<SnippetDefinition>,
    active_theme_id: Option<String>,
}

impl ExtensionCompatManager {
    pub fn new(config: CompatConfig) -> Self {
        Self {
            config,
            extensions: Vec::new(),
            grammars: Vec::new(),
            themes: Vec::new(),
            snippets: Vec::new(),
            active_theme_id: None,
        }
    }

    /// Install a VS Code extension into the manager.
    pub fn install_extension(&mut self, ext: VsCodeExtension) -> Result<(), CompatError> {
        if self.extensions.len() >= self.config.max_extensions {
            return Err(CompatError::MaxExtensionsReached);
        }
        if self.extensions.iter().any(|e| e.id == ext.id) {
            return Err(CompatError::DuplicateExtension);
        }
        for cat in &ext.categories {
            if !self.is_category_supported(cat) {
                return Err(CompatError::UnsupportedCategory);
            }
        }
        if ext.id.is_empty() || !ext.id.contains('.') {
            return Err(CompatError::InvalidManifest(
                "extension id must be publisher.name".to_string(),
            ));
        }

        // Auto-parse files on install
        for file in &ext.files {
            if let Some(ref content) = file.content {
                match file.file_type {
                    FileType::TmLanguage => {
                        if let Ok(grammar) = Self::parse_tm_grammar(content) {
                            self.grammars.push(grammar);
                        }
                    }
                    FileType::TmTheme => {
                        if let Ok(theme) = Self::parse_theme(content) {
                            self.themes.push(theme);
                        }
                    }
                    FileType::SnippetJson => {
                        // Derive language from file path or first category
                        let lang = file
                            .path
                            .split('/')
                            .next_back()
                            .and_then(|f| f.strip_suffix(".json"))
                            .unwrap_or("unknown");
                        if let Ok(snips) = Self::parse_snippets(content, lang) {
                            self.snippets.extend(snips);
                        }
                    }
                    _ => {}
                }
            }
        }

        self.extensions.push(ext);
        Ok(())
    }

    /// Uninstall an extension by id.
    pub fn uninstall_extension(&mut self, id: &str) -> Result<(), CompatError> {
        let idx = self
            .extensions
            .iter()
            .position(|e| e.id == id)
            .ok_or(CompatError::ExtensionNotFound)?;
        self.extensions.remove(idx);
        Ok(())
    }

    /// Get an extension by id.
    pub fn get_extension(&self, id: &str) -> Option<&VsCodeExtension> {
        self.extensions.iter().find(|e| e.id == id)
    }

    /// List all installed extensions.
    pub fn list_extensions(&self) -> Vec<&VsCodeExtension> {
        self.extensions.iter().collect()
    }

    /// Enable an extension.
    pub fn enable_extension(&mut self, id: &str) -> Result<(), CompatError> {
        let ext = self
            .extensions
            .iter_mut()
            .find(|e| e.id == id)
            .ok_or(CompatError::ExtensionNotFound)?;
        ext.enabled = true;
        Ok(())
    }

    /// Disable an extension.
    pub fn disable_extension(&mut self, id: &str) -> Result<(), CompatError> {
        let ext = self
            .extensions
            .iter_mut()
            .find(|e| e.id == id)
            .ok_or(CompatError::ExtensionNotFound)?;
        ext.enabled = false;
        Ok(())
    }

    /// Parse a TextMate grammar from JSON content.
    pub fn parse_tm_grammar(content: &str) -> Result<TextMateGrammar, CompatError> {
        // Basic JSON parsing without serde — look for key fields
        let scope_name = Self::extract_json_string(content, "scopeName")
            .ok_or_else(|| CompatError::ParseError("missing scopeName".to_string()))?;
        let language_id = Self::extract_json_string(content, "languageId")
            .or_else(|| Self::extract_json_string(content, "name"))
            .unwrap_or_default();
        let file_extensions = Self::extract_json_string_array(content, "fileTypes");

        let mut patterns = Vec::new();
        // Extract patterns array entries (simplified)
        if let Some(pat_start) = content.find("\"patterns\"") {
            let rest = &content[pat_start..];
            // Collect pattern names
            let mut search = rest;
            while let Some(pos) = search.find("\"name\"") {
                let after = &search[pos + 7..];
                if let Some(name) = Self::extract_next_string_value(after) {
                    let match_pat = Self::extract_json_string(
                        &search[pos..std::cmp::min(pos + 500, search.len())],
                        "match",
                    );
                    let begin = Self::extract_json_string(
                        &search[pos..std::cmp::min(pos + 500, search.len())],
                        "begin",
                    );
                    let end = Self::extract_json_string(
                        &search[pos..std::cmp::min(pos + 500, search.len())],
                        "end",
                    );
                    patterns.push(GrammarPattern {
                        name,
                        match_pattern: match_pat,
                        begin,
                        end,
                        captures: Vec::new(),
                    });
                }
                if pos + 7 >= search.len() {
                    break;
                }
                search = &search[pos + 7..];
            }
        }

        Ok(TextMateGrammar {
            scope_name,
            language_id,
            file_extensions,
            patterns,
        })
    }

    /// Parse a theme definition from JSON content.
    pub fn parse_theme(content: &str) -> Result<ThemeDefinition, CompatError> {
        let name = Self::extract_json_string(content, "name")
            .ok_or_else(|| CompatError::ParseError("missing theme name".to_string()))?;

        let type_str = Self::extract_json_string(content, "type").unwrap_or_default();
        let theme_type = match type_str.as_str() {
            "light" => ThemeType::Light,
            "hc" | "hcDark" | "hcLight" | "high-contrast" => ThemeType::HighContrast,
            _ => ThemeType::Dark,
        };

        let mut colors = Vec::new();
        // Extract colors object entries
        if let Some(col_start) = content.find("\"colors\"") {
            let rest = &content[col_start..];
            if let Some(brace) = rest.find('{') {
                let inner = &rest[brace + 1..];
                if let Some(end_brace) = inner.find('}') {
                    let block = &inner[..end_brace];
                    for line in block.lines() {
                        let line = line.trim().trim_end_matches(',');
                        if let Some((key, val)) = line.split_once(':') {
                            let scope = key.trim().trim_matches('"').to_string();
                            let fg = val.trim().trim_matches('"').to_string();
                            if !scope.is_empty() && !fg.is_empty() {
                                colors.push(ThemeColor {
                                    scope,
                                    foreground: Some(fg),
                                    background: None,
                                    font_style: None,
                                });
                            }
                        }
                    }
                }
            }
        }

        let mut token_colors = Vec::new();
        // Extract tokenColors entries (simplified)
        if let Some(tc_start) = content.find("\"tokenColors\"") {
            let rest = &content[tc_start..];
            let mut search = rest;
            while let Some(pos) = search.find("\"name\"") {
                let after = &search[pos + 7..];
                if let Some(tc_name) = Self::extract_next_string_value(after) {
                    let chunk = &search[pos..std::cmp::min(pos + 500, search.len())];
                    let fg = Self::extract_json_string(chunk, "foreground");
                    let fs = Self::extract_json_string(chunk, "fontStyle");
                    token_colors.push(TokenColor {
                        name: tc_name,
                        scope: Vec::new(),
                        foreground: fg,
                        font_style: fs,
                    });
                }
                if pos + 7 >= search.len() {
                    break;
                }
                search = &search[pos + 7..];
            }
        }

        Ok(ThemeDefinition {
            name,
            theme_type,
            colors,
            token_colors,
        })
    }

    /// Parse snippet definitions from JSON content.
    pub fn parse_snippets(
        content: &str,
        language: &str,
    ) -> Result<Vec<SnippetDefinition>, CompatError> {
        let mut snippets = Vec::new();
        // Snippets format: { "Name": { "prefix": "...", "body": [...], "description": "..." } }
        let trimmed = content.trim();
        if !trimmed.starts_with('{') {
            return Err(CompatError::ParseError("expected JSON object".to_string()));
        }

        // Simplified: find top-level keys that have "prefix" inside
        let mut search = trimmed;
        while let Some(prefix_pos) = search.find("\"prefix\"") {
            // Walk back to find the snippet name
            let before = &search[..prefix_pos];
            let snippet_name = before
                .rfind('"')
                .and_then(|end| {
                    let slice = &before[..end];
                    slice.rfind('"').map(|start| &before[start + 1..end])
                })
                .unwrap_or("unknown");

            let chunk_end = std::cmp::min(prefix_pos + 1000, search.len());
            let chunk = &search[prefix_pos..chunk_end];

            let prefix = Self::extract_json_string(chunk, "prefix").unwrap_or_default();
            let description =
                Self::extract_json_string(chunk, "description").unwrap_or_default();

            // Extract body (may be string or array)
            let body = if let Some(body_start) = chunk.find("\"body\"") {
                let after_body = &chunk[body_start + 6..];
                let after_colon = after_body
                    .find(':')
                    .map(|i| &after_body[i + 1..])
                    .unwrap_or(after_body);
                let trimmed_body = after_colon.trim_start();
                if trimmed_body.starts_with('[') {
                    Self::extract_json_string_array_inline(trimmed_body)
                } else if trimmed_body.starts_with('"') {
                    Self::extract_next_string_value(trimmed_body)
                        .map(|s| vec![s])
                        .unwrap_or_default()
                } else {
                    Vec::new()
                }
            } else {
                Vec::new()
            };

            if !prefix.is_empty() {
                snippets.push(SnippetDefinition {
                    name: snippet_name.to_string(),
                    prefix,
                    body,
                    description,
                    language: language.to_string(),
                });
            }

            if prefix_pos + 8 >= search.len() {
                break;
            }
            search = &search[prefix_pos + 8..];
        }

        Ok(snippets)
    }

    /// Parse a language configuration from JSON content.
    pub fn parse_language_config(content: &str) -> Result<LanguageConfig, CompatError> {
        let language_id = Self::extract_json_string(content, "languageId")
            .or_else(|| Self::extract_json_string(content, "id"))
            .ok_or_else(|| {
                CompatError::ParseError("missing languageId or id".to_string())
            })?;

        let extensions = Self::extract_json_string_array(content, "extensions");

        // Comments
        let comment_line = Self::extract_nested_string(content, "comments", "lineComment");
        let comment_block_start =
            Self::extract_nested_string(content, "comments", "blockComment");
        let comment_block_end = comment_block_start.as_ref().and_then(|_| {
            // Look for second string in blockComment array
            Self::extract_nested_second_string(content, "blockComment")
        });

        // Brackets
        let brackets = Self::extract_pair_array(content, "brackets");
        let auto_closing_pairs = Self::extract_pair_array(content, "autoClosingPairs");

        Ok(LanguageConfig {
            language_id,
            extensions,
            comment_line,
            comment_block_start,
            comment_block_end,
            brackets,
            auto_closing_pairs,
        })
    }

    /// Get a grammar by file extension (e.g. "rs", "py").
    pub fn get_grammar_for_extension(&self, ext: &str) -> Option<&TextMateGrammar> {
        self.grammars
            .iter()
            .find(|g| g.file_extensions.iter().any(|e| e == ext))
    }

    /// Get all snippets for a given language.
    pub fn get_snippets_for_language(&self, lang: &str) -> Vec<&SnippetDefinition> {
        self.snippets
            .iter()
            .filter(|s| s.language == lang)
            .collect()
    }

    /// Get the currently active theme.
    pub fn get_active_theme(&self) -> Option<&ThemeDefinition> {
        self.active_theme_id.as_ref().and_then(|id| {
            // Match by name against installed themes
            self.themes.iter().find(|t| t.name == *id)
        })
    }

    /// Set the active theme by extension id (uses theme name).
    pub fn set_active_theme(&mut self, theme_name: &str) -> Result<(), CompatError> {
        if !self.themes.iter().any(|t| t.name == theme_name) {
            return Err(CompatError::ExtensionNotFound);
        }
        self.active_theme_id = Some(theme_name.to_string());
        Ok(())
    }

    /// Simulated marketplace search.
    pub fn search_marketplace(&self, query: &str) -> MarketplaceSearchResult {
        // Return simulated results based on query
        let mut results = Vec::new();
        let catalog = Self::simulated_catalog();

        for entry in catalog {
            let haystack = format!(
                "{} {} {}",
                entry.name.to_lowercase(),
                entry.publisher.to_lowercase(),
                entry.description.to_lowercase()
            );
            if haystack.contains(&query.to_lowercase()) {
                results.push(entry);
            }
        }

        let total_count = results.len();
        MarketplaceSearchResult {
            extensions: results,
            total_count,
        }
    }

    /// Check if a category is supported by this manager.
    pub fn is_category_supported(&self, cat: &ExtensionCategory) -> bool {
        self.config.supported_categories.contains(cat)
    }

    /// Return statistics about installed extensions.
    pub fn get_stats(&self) -> CompatStats {
        let mut by_category: HashMap<String, usize> = HashMap::new();
        let mut total_size: u64 = 0;
        let mut enabled = 0;

        for ext in &self.extensions {
            if ext.enabled {
                enabled += 1;
            }
            total_size += ext.size_bytes;
            for cat in &ext.categories {
                *by_category.entry(cat.as_str().to_string()).or_insert(0) += 1;
            }
        }

        CompatStats {
            total: self.extensions.len(),
            enabled,
            by_category,
            total_size,
        }
    }

    // === Private helpers ===

    fn simulated_catalog() -> Vec<MarketplaceEntry> {
        vec![
            MarketplaceEntry {
                id: "rust-lang.rust-analyzer".to_string(),
                name: "rust-analyzer".to_string(),
                publisher: "rust-lang".to_string(),
                description: "Rust language support".to_string(),
                version: "0.4.1".to_string(),
                install_count: 5_000_000,
                rating: 4.8,
                categories: vec!["Programming Languages".to_string()],
            },
            MarketplaceEntry {
                id: "dracula-theme.theme-dracula".to_string(),
                name: "Dracula Official".to_string(),
                publisher: "dracula-theme".to_string(),
                description: "Dark theme for VS Code".to_string(),
                version: "2.25.1".to_string(),
                install_count: 10_000_000,
                rating: 4.7,
                categories: vec!["Themes".to_string()],
            },
            MarketplaceEntry {
                id: "ms-python.python".to_string(),
                name: "Python".to_string(),
                publisher: "Microsoft".to_string(),
                description: "Python language support".to_string(),
                version: "2024.1.0".to_string(),
                install_count: 100_000_000,
                rating: 4.5,
                categories: vec!["Programming Languages".to_string()],
            },
        ]
    }

    /// Extract a simple JSON string value for a key: "key": "value"
    fn extract_json_string(content: &str, key: &str) -> Option<String> {
        let needle = format!("\"{}\"", key);
        let pos = content.find(&needle)?;
        let after_key = &content[pos + needle.len()..];
        let after_colon = after_key.find(':').map(|i| &after_key[i + 1..])?;
        Self::extract_next_string_value(after_colon)
    }

    /// Extract a string array: "key": ["a", "b"]
    fn extract_json_string_array(content: &str, key: &str) -> Vec<String> {
        let needle = format!("\"{}\"", key);
        if let Some(pos) = content.find(&needle) {
            let after_key = &content[pos + needle.len()..];
            if let Some(colon) = after_key.find(':') {
                let after_colon = after_key[colon + 1..].trim_start();
                if after_colon.starts_with('[') {
                    return Self::extract_json_string_array_inline(after_colon);
                }
            }
        }
        Vec::new()
    }

    /// Extract strings from an inline array starting with '['
    fn extract_json_string_array_inline(content: &str) -> Vec<String> {
        let mut results = Vec::new();
        if let Some(end) = content.find(']') {
            let inner = &content[1..end];
            let mut search = inner;
            while let Some(start) = search.find('"') {
                let rest = &search[start + 1..];
                if let Some(end_q) = rest.find('"') {
                    results.push(rest[..end_q].to_string());
                    search = &rest[end_q + 1..];
                } else {
                    break;
                }
            }
        }
        results
    }

    /// Extract the next quoted string value from content.
    fn extract_next_string_value(content: &str) -> Option<String> {
        let start = content.find('"')?;
        let rest = &content[start + 1..];
        let end = rest.find('"')?;
        Some(rest[..end].to_string())
    }

    /// Extract a nested string: "outer": { "inner": "value" }
    fn extract_nested_string(content: &str, outer: &str, inner: &str) -> Option<String> {
        let needle = format!("\"{}\"", outer);
        let pos = content.find(&needle)?;
        let after = &content[pos..];
        let brace = after.find('{')?;
        let block = &after[brace..];
        let end_brace = block.find('}')?;
        let inner_block = &block[..end_brace];
        Self::extract_json_string(inner_block, inner)
    }

    /// Extract second string from a JSON array at a given key.
    fn extract_nested_second_string(content: &str, key: &str) -> Option<String> {
        let needle = format!("\"{}\"", key);
        let pos = content.find(&needle)?;
        let after = &content[pos..];
        let bracket = after.find('[')?;
        let arr_content = &after[bracket..];
        let end_bracket = arr_content.find(']')?;
        let inner = &arr_content[1..end_bracket];
        // Find second string
        let first_end = inner.find('"').and_then(|s| {
            let rest = &inner[s + 1..];
            rest.find('"').map(|e| s + 1 + e + 1)
        })?;
        let rest = &inner[first_end..];
        Self::extract_next_string_value(rest)
    }

    /// Extract pairs from a JSON array of 2-element arrays.
    fn extract_pair_array(content: &str, key: &str) -> Vec<(String, String)> {
        let mut pairs = Vec::new();
        let needle = format!("\"{}\"", key);
        if let Some(pos) = content.find(&needle) {
            let after = &content[pos..];
            if let Some(bracket) = after.find('[') {
                let arr = &after[bracket + 1..];
                // Find inner arrays [" ", " "]
                let mut search = arr;
                while let Some(inner_start) = search.find('[') {
                    let inner = &search[inner_start + 1..];
                    if let Some(inner_end) = inner.find(']') {
                        let pair_str = &inner[..inner_end];
                        let items = Self::extract_json_string_array_inline(
                            &format!("[{}]", pair_str),
                        );
                        if items.len() >= 2 {
                            pairs.push((items[0].clone(), items[1].clone()));
                        }
                        search = &inner[inner_end + 1..];
                    } else {
                        break;
                    }
                }
            }
        }
        pairs
    }
}

// === Tests ===

#[cfg(test)]
mod tests {
    use super::*;

    fn default_manager() -> ExtensionCompatManager {
        ExtensionCompatManager::new(CompatConfig::default())
    }

    fn sample_extension(id: &str) -> VsCodeExtension {
        VsCodeExtension {
            id: id.to_string(),
            name: id.split('.').last().unwrap_or(id).to_string(),
            publisher: id.split('.').next().unwrap_or("test").to_string(),
            version: "1.0.0".to_string(),
            description: "Test extension".to_string(),
            categories: vec![ExtensionCategory::LanguageGrammar],
            files: Vec::new(),
            installed_at: 1700000000,
            enabled: true,
            size_bytes: 1024,
        }
    }

    #[test]
    fn test_default_config() {
        let config = CompatConfig::default();
        assert_eq!(config.extensions_dir, ".vibecody/extensions");
        assert_eq!(config.max_extensions, 100);
        assert_eq!(config.supported_categories.len(), 6);
    }

    #[test]
    fn test_install_extension() {
        let mut mgr = default_manager();
        let ext = sample_extension("test.grammar");
        assert!(mgr.install_extension(ext).is_ok());
        assert_eq!(mgr.list_extensions().len(), 1);
    }

    #[test]
    fn test_install_duplicate_extension() {
        let mut mgr = default_manager();
        mgr.install_extension(sample_extension("test.grammar")).unwrap();
        let result = mgr.install_extension(sample_extension("test.grammar"));
        assert_eq!(result.unwrap_err(), CompatError::DuplicateExtension);
    }

    #[test]
    fn test_install_max_extensions_reached() {
        let config = CompatConfig {
            max_extensions: 2,
            ..CompatConfig::default()
        };
        let mut mgr = ExtensionCompatManager::new(config);
        mgr.install_extension(sample_extension("a.one")).unwrap();
        mgr.install_extension(sample_extension("b.two")).unwrap();
        let result = mgr.install_extension(sample_extension("c.three"));
        assert_eq!(result.unwrap_err(), CompatError::MaxExtensionsReached);
    }

    #[test]
    fn test_install_invalid_id() {
        let mut mgr = default_manager();
        let mut ext = sample_extension("test.grammar");
        ext.id = "nope".to_string(); // no dot
        let result = mgr.install_extension(ext);
        assert!(matches!(result.unwrap_err(), CompatError::InvalidManifest(_)));
    }

    #[test]
    fn test_install_empty_id() {
        let mut mgr = default_manager();
        let mut ext = sample_extension("test.grammar");
        ext.id = String::new();
        let result = mgr.install_extension(ext);
        assert!(matches!(result.unwrap_err(), CompatError::InvalidManifest(_)));
    }

    #[test]
    fn test_uninstall_extension() {
        let mut mgr = default_manager();
        mgr.install_extension(sample_extension("test.grammar")).unwrap();
        assert!(mgr.uninstall_extension("test.grammar").is_ok());
        assert_eq!(mgr.list_extensions().len(), 0);
    }

    #[test]
    fn test_uninstall_not_found() {
        let mut mgr = default_manager();
        let result = mgr.uninstall_extension("nope.nope");
        assert_eq!(result.unwrap_err(), CompatError::ExtensionNotFound);
    }

    #[test]
    fn test_get_extension() {
        let mut mgr = default_manager();
        mgr.install_extension(sample_extension("pub.name")).unwrap();
        let ext = mgr.get_extension("pub.name");
        assert!(ext.is_some());
        assert_eq!(ext.unwrap().name, "name");
    }

    #[test]
    fn test_get_extension_not_found() {
        let mgr = default_manager();
        assert!(mgr.get_extension("nope.nope").is_none());
    }

    #[test]
    fn test_enable_extension() {
        let mut mgr = default_manager();
        let mut ext = sample_extension("test.ext");
        ext.enabled = false;
        mgr.install_extension(ext).unwrap();
        mgr.enable_extension("test.ext").unwrap();
        assert!(mgr.get_extension("test.ext").unwrap().enabled);
    }

    #[test]
    fn test_disable_extension() {
        let mut mgr = default_manager();
        mgr.install_extension(sample_extension("test.ext")).unwrap();
        mgr.disable_extension("test.ext").unwrap();
        assert!(!mgr.get_extension("test.ext").unwrap().enabled);
    }

    #[test]
    fn test_enable_not_found() {
        let mut mgr = default_manager();
        assert_eq!(
            mgr.enable_extension("nope.nope").unwrap_err(),
            CompatError::ExtensionNotFound
        );
    }

    #[test]
    fn test_disable_not_found() {
        let mut mgr = default_manager();
        assert_eq!(
            mgr.disable_extension("nope.nope").unwrap_err(),
            CompatError::ExtensionNotFound
        );
    }

    #[test]
    fn test_parse_tm_grammar() {
        let json = r#"{
            "scopeName": "source.rust",
            "languageId": "rust",
            "fileTypes": ["rs"],
            "patterns": [
                { "name": "comment.line", "match": "//.*$" },
                { "name": "string.quoted", "begin": "\"", "end": "\"" }
            ]
        }"#;
        let grammar = ExtensionCompatManager::parse_tm_grammar(json).unwrap();
        assert_eq!(grammar.scope_name, "source.rust");
        assert_eq!(grammar.language_id, "rust");
        assert_eq!(grammar.file_extensions, vec!["rs"]);
        assert!(grammar.patterns.len() >= 2);
    }

    #[test]
    fn test_parse_tm_grammar_missing_scope() {
        let json = r#"{ "languageId": "test" }"#;
        let result = ExtensionCompatManager::parse_tm_grammar(json);
        assert!(matches!(result.unwrap_err(), CompatError::ParseError(_)));
    }

    #[test]
    fn test_parse_theme_dark() {
        let json = r##"{
            "name": "My Dark Theme",
            "type": "dark",
            "colors": {
                "editor.background": "#1e1e1e",
                "editor.foreground": "#d4d4d4"
            },
            "tokenColors": [
                { "name": "Comments", "scope": ["comment"], "settings": { "foreground": "#6A9955" } }
            ]
        }"##;
        let theme = ExtensionCompatManager::parse_theme(json).unwrap();
        assert_eq!(theme.name, "My Dark Theme");
        assert_eq!(theme.theme_type, ThemeType::Dark);
        assert!(!theme.colors.is_empty());
    }

    #[test]
    fn test_parse_theme_light() {
        let json = r#"{ "name": "Light", "type": "light", "colors": {} }"#;
        let theme = ExtensionCompatManager::parse_theme(json).unwrap();
        assert_eq!(theme.theme_type, ThemeType::Light);
    }

    #[test]
    fn test_parse_theme_high_contrast() {
        let json = r#"{ "name": "HC", "type": "hc", "colors": {} }"#;
        let theme = ExtensionCompatManager::parse_theme(json).unwrap();
        assert_eq!(theme.theme_type, ThemeType::HighContrast);
    }

    #[test]
    fn test_parse_theme_missing_name() {
        let json = r#"{ "type": "dark" }"#;
        let result = ExtensionCompatManager::parse_theme(json);
        assert!(matches!(result.unwrap_err(), CompatError::ParseError(_)));
    }

    #[test]
    fn test_parse_snippets() {
        let json = r#"{
            "Print": {
                "prefix": "println",
                "body": ["println!(\"$1\");"],
                "description": "Print line macro"
            },
            "Main": {
                "prefix": "main",
                "body": ["fn main() {", "    $0", "}"],
                "description": "Main function"
            }
        }"#;
        let snippets = ExtensionCompatManager::parse_snippets(json, "rust").unwrap();
        assert_eq!(snippets.len(), 2);
        assert_eq!(snippets[0].language, "rust");
        assert!(!snippets[0].prefix.is_empty());
    }

    #[test]
    fn test_parse_snippets_invalid_json() {
        let result = ExtensionCompatManager::parse_snippets("not json", "rust");
        assert!(matches!(result.unwrap_err(), CompatError::ParseError(_)));
    }

    #[test]
    fn test_parse_language_config() {
        let json = r#"{
            "languageId": "rust",
            "extensions": [".rs"],
            "comments": {
                "lineComment": "//",
                "blockComment": ["/*", "*/"]
            },
            "brackets": [
                ["{", "}"],
                ["[", "]"],
                ["(", ")"]
            ],
            "autoClosingPairs": [
                ["{", "}"],
                ["(", ")"]
            ]
        }"#;
        let config = ExtensionCompatManager::parse_language_config(json).unwrap();
        assert_eq!(config.language_id, "rust");
        assert_eq!(config.extensions, vec![".rs"]);
        assert_eq!(config.comment_line, Some("//".to_string()));
        assert!(!config.brackets.is_empty());
    }

    #[test]
    fn test_parse_language_config_missing_id() {
        let json = r#"{ "extensions": [".txt"] }"#;
        let result = ExtensionCompatManager::parse_language_config(json);
        assert!(matches!(result.unwrap_err(), CompatError::ParseError(_)));
    }

    #[test]
    fn test_get_grammar_for_extension() {
        let mut mgr = default_manager();
        mgr.grammars.push(TextMateGrammar {
            scope_name: "source.python".to_string(),
            language_id: "python".to_string(),
            file_extensions: vec!["py".to_string(), "pyi".to_string()],
            patterns: Vec::new(),
        });
        assert!(mgr.get_grammar_for_extension("py").is_some());
        assert!(mgr.get_grammar_for_extension("pyi").is_some());
        assert!(mgr.get_grammar_for_extension("rs").is_none());
    }

    #[test]
    fn test_get_snippets_for_language() {
        let mut mgr = default_manager();
        mgr.snippets.push(SnippetDefinition {
            name: "test".to_string(),
            prefix: "tst".to_string(),
            body: vec!["test()".to_string()],
            description: "Test".to_string(),
            language: "python".to_string(),
        });
        mgr.snippets.push(SnippetDefinition {
            name: "log".to_string(),
            prefix: "log".to_string(),
            body: vec!["console.log()".to_string()],
            description: "Log".to_string(),
            language: "javascript".to_string(),
        });
        assert_eq!(mgr.get_snippets_for_language("python").len(), 1);
        assert_eq!(mgr.get_snippets_for_language("javascript").len(), 1);
        assert_eq!(mgr.get_snippets_for_language("rust").len(), 0);
    }

    #[test]
    fn test_set_active_theme() {
        let mut mgr = default_manager();
        mgr.themes.push(ThemeDefinition {
            name: "Dracula".to_string(),
            theme_type: ThemeType::Dark,
            colors: Vec::new(),
            token_colors: Vec::new(),
        });
        assert!(mgr.get_active_theme().is_none());
        mgr.set_active_theme("Dracula").unwrap();
        assert_eq!(mgr.get_active_theme().unwrap().name, "Dracula");
    }

    #[test]
    fn test_set_active_theme_not_found() {
        let mut mgr = default_manager();
        assert_eq!(
            mgr.set_active_theme("Nope").unwrap_err(),
            CompatError::ExtensionNotFound
        );
    }

    #[test]
    fn test_search_marketplace_match() {
        let mgr = default_manager();
        let results = mgr.search_marketplace("rust");
        assert!(results.total_count >= 1);
        assert!(results.extensions.iter().any(|e| e.name.contains("rust")));
    }

    #[test]
    fn test_search_marketplace_no_match() {
        let mgr = default_manager();
        let results = mgr.search_marketplace("zzz_nonexistent_zzz");
        assert_eq!(results.total_count, 0);
    }

    #[test]
    fn test_search_marketplace_theme() {
        let mgr = default_manager();
        let results = mgr.search_marketplace("dracula");
        assert!(results.total_count >= 1);
    }

    #[test]
    fn test_is_category_supported() {
        let mgr = default_manager();
        assert!(mgr.is_category_supported(&ExtensionCategory::LanguageGrammar));
        assert!(mgr.is_category_supported(&ExtensionCategory::ColorTheme));
        assert!(mgr.is_category_supported(&ExtensionCategory::IconTheme));
    }

    #[test]
    fn test_is_category_supported_custom_config() {
        let config = CompatConfig {
            supported_categories: vec![ExtensionCategory::ColorTheme],
            ..CompatConfig::default()
        };
        let mgr = ExtensionCompatManager::new(config);
        assert!(mgr.is_category_supported(&ExtensionCategory::ColorTheme));
        assert!(!mgr.is_category_supported(&ExtensionCategory::LanguageGrammar));
    }

    #[test]
    fn test_get_stats_empty() {
        let mgr = default_manager();
        let stats = mgr.get_stats();
        assert_eq!(stats.total, 0);
        assert_eq!(stats.enabled, 0);
        assert_eq!(stats.total_size, 0);
    }

    #[test]
    fn test_get_stats_with_extensions() {
        let mut mgr = default_manager();
        mgr.install_extension(sample_extension("a.one")).unwrap();
        let mut ext2 = sample_extension("b.two");
        ext2.enabled = false;
        ext2.categories = vec![ExtensionCategory::ColorTheme];
        ext2.size_bytes = 2048;
        mgr.install_extension(ext2).unwrap();

        let stats = mgr.get_stats();
        assert_eq!(stats.total, 2);
        assert_eq!(stats.enabled, 1);
        assert_eq!(stats.total_size, 1024 + 2048);
        assert_eq!(
            *stats.by_category.get("language-grammar").unwrap_or(&0),
            1
        );
        assert_eq!(*stats.by_category.get("color-theme").unwrap_or(&0), 1);
    }

    #[test]
    fn test_extension_category_as_str() {
        assert_eq!(ExtensionCategory::LanguageGrammar.as_str(), "language-grammar");
        assert_eq!(ExtensionCategory::ColorTheme.as_str(), "color-theme");
        assert_eq!(ExtensionCategory::Snippet.as_str(), "snippet");
        assert_eq!(
            ExtensionCategory::LanguageConfiguration.as_str(),
            "language-configuration"
        );
        assert_eq!(ExtensionCategory::IconTheme.as_str(), "icon-theme");
        assert_eq!(ExtensionCategory::KeybindingSet.as_str(), "keybinding-set");
    }

    #[test]
    fn test_file_type_as_str() {
        assert_eq!(FileType::TmLanguage.as_str(), "tmLanguage");
        assert_eq!(FileType::TmTheme.as_str(), "tmTheme");
        assert_eq!(FileType::PackageJson.as_str(), "package.json");
        assert_eq!(FileType::Other.as_str(), "other");
    }

    #[test]
    fn test_compat_error_display() {
        assert_eq!(CompatError::ExtensionNotFound.to_string(), "extension not found");
        assert_eq!(CompatError::DuplicateExtension.to_string(), "extension already installed");
        assert_eq!(CompatError::MaxExtensionsReached.to_string(), "maximum extensions limit reached");
        assert_eq!(
            CompatError::ParseError("bad".to_string()).to_string(),
            "parse error: bad"
        );
    }

    #[test]
    fn test_theme_type_display() {
        assert_eq!(ThemeType::Dark.to_string(), "dark");
        assert_eq!(ThemeType::Light.to_string(), "light");
        assert_eq!(ThemeType::HighContrast.to_string(), "high-contrast");
    }

    #[test]
    fn test_install_with_auto_parse_grammar() {
        let mut mgr = default_manager();
        let grammar_json = r#"{ "scopeName": "source.go", "languageId": "go", "fileTypes": ["go"] }"#;
        let ext = VsCodeExtension {
            id: "golang.go-grammar".to_string(),
            name: "go-grammar".to_string(),
            publisher: "golang".to_string(),
            version: "1.0.0".to_string(),
            description: "Go grammar".to_string(),
            categories: vec![ExtensionCategory::LanguageGrammar],
            files: vec![ExtensionFile {
                path: "syntaxes/go.tmLanguage.json".to_string(),
                file_type: FileType::TmLanguage,
                content: Some(grammar_json.to_string()),
            }],
            installed_at: 1700000000,
            enabled: true,
            size_bytes: 512,
        };
        mgr.install_extension(ext).unwrap();
        assert!(mgr.get_grammar_for_extension("go").is_some());
    }

    #[test]
    fn test_install_with_auto_parse_theme() {
        let mut mgr = default_manager();
        let theme_json = r#"{ "name": "Nord", "type": "dark", "colors": {} }"#;
        let ext = VsCodeExtension {
            id: "nord.nord-theme".to_string(),
            name: "nord-theme".to_string(),
            publisher: "nord".to_string(),
            version: "1.0.0".to_string(),
            description: "Nord theme".to_string(),
            categories: vec![ExtensionCategory::ColorTheme],
            files: vec![ExtensionFile {
                path: "themes/nord.json".to_string(),
                file_type: FileType::TmTheme,
                content: Some(theme_json.to_string()),
            }],
            installed_at: 1700000000,
            enabled: true,
            size_bytes: 256,
        };
        mgr.install_extension(ext).unwrap();
        mgr.set_active_theme("Nord").unwrap();
        assert_eq!(mgr.get_active_theme().unwrap().name, "Nord");
    }

    #[test]
    fn test_install_with_auto_parse_snippets() {
        let mut mgr = default_manager();
        let snip_json = r#"{ "Log": { "prefix": "log", "body": ["console.log($1)"], "description": "Log" } }"#;
        let ext = VsCodeExtension {
            id: "snip.js-snippets".to_string(),
            name: "js-snippets".to_string(),
            publisher: "snip".to_string(),
            version: "1.0.0".to_string(),
            description: "JS snippets".to_string(),
            categories: vec![ExtensionCategory::Snippet],
            files: vec![ExtensionFile {
                path: "snippets/javascript.json".to_string(),
                file_type: FileType::SnippetJson,
                content: Some(snip_json.to_string()),
            }],
            installed_at: 1700000000,
            enabled: true,
            size_bytes: 128,
        };
        mgr.install_extension(ext).unwrap();
        assert_eq!(mgr.get_snippets_for_language("javascript").len(), 1);
    }

    #[test]
    fn test_install_unsupported_category() {
        let config = CompatConfig {
            supported_categories: vec![ExtensionCategory::ColorTheme],
            ..CompatConfig::default()
        };
        let mut mgr = ExtensionCompatManager::new(config);
        let ext = sample_extension("test.grammar"); // has LanguageGrammar
        assert_eq!(
            mgr.install_extension(ext).unwrap_err(),
            CompatError::UnsupportedCategory
        );
    }

    #[test]
    fn test_list_multiple_extensions() {
        let mut mgr = default_manager();
        mgr.install_extension(sample_extension("a.one")).unwrap();
        mgr.install_extension(sample_extension("b.two")).unwrap();
        mgr.install_extension(sample_extension("c.three")).unwrap();
        assert_eq!(mgr.list_extensions().len(), 3);
    }

    #[test]
    fn test_marketplace_entry_fields() {
        let mgr = default_manager();
        let results = mgr.search_marketplace("python");
        assert!(!results.extensions.is_empty());
        let entry = &results.extensions[0];
        assert!(!entry.id.is_empty());
        assert!(entry.install_count > 0);
        assert!(entry.rating > 0.0);
    }

    #[test]
    fn test_extension_category_display() {
        assert_eq!(
            format!("{}", ExtensionCategory::LanguageGrammar),
            "language-grammar"
        );
        assert_eq!(format!("{}", ExtensionCategory::Snippet), "snippet");
    }
}
