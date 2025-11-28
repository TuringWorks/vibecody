//! Syntax highlighting for code blocks

use syntect::easy::HighlightLines;
use syntect::highlighting::{ThemeSet, Style};
use syntect::parsing::SyntaxSet;
use syntect::util::{as_24_bit_terminal_escaped, LinesWithEndings};

pub struct SyntaxHighlighter {
    syntax_set: SyntaxSet,
    theme_set: ThemeSet,
}

impl SyntaxHighlighter {
    pub fn new() -> Self {
        Self {
            syntax_set: SyntaxSet::load_defaults_newlines(),
            theme_set: ThemeSet::load_defaults(),
        }
    }

    pub fn highlight(&self, code: &str, language: Option<&str>) -> String {
        let syntax = if let Some(lang) = language {
            self.syntax_set
                .find_syntax_by_token(lang)
                .or_else(|| self.syntax_set.find_syntax_by_extension(lang))
                .unwrap_or_else(|| self.syntax_set.find_syntax_plain_text())
        } else {
            self.syntax_set.find_syntax_plain_text()
        };

        let theme = &self.theme_set.themes["base16-ocean.dark"];
        let mut highlighter = HighlightLines::new(syntax, theme);
        
        let mut highlighted = String::new();
        for line in LinesWithEndings::from(code) {
            let ranges: Vec<(Style, &str)> = highlighter
                .highlight_line(line, &self.syntax_set)
                .unwrap_or_default();
            let escaped = as_24_bit_terminal_escaped(&ranges[..], false);
            highlighted.push_str(&escaped);
        }
        
        highlighted
    }

    #[allow(dead_code)]
    pub fn highlight_ranges(&self, code: &str, language: Option<&str>) -> Vec<(Style, String)> {
        let syntax = if let Some(lang) = language {
            self.syntax_set
                .find_syntax_by_token(lang)
                .or_else(|| self.syntax_set.find_syntax_by_extension(lang))
                .unwrap_or_else(|| self.syntax_set.find_syntax_plain_text())
        } else {
            self.syntax_set.find_syntax_plain_text()
        };

        let theme = &self.theme_set.themes["base16-ocean.dark"];
        let mut highlighter = HighlightLines::new(syntax, theme);
        
        let mut ranges = Vec::new();
        for line in LinesWithEndings::from(code) {
            let line_ranges: Vec<(Style, &str)> = highlighter
                .highlight_line(line, &self.syntax_set)
                .unwrap_or_default();
            
            for (style, text) in line_ranges {
                ranges.push((style, text.to_string()));
            }
        }
        
        ranges
    }

    #[allow(dead_code)]
    pub fn detect_language(code: &str) -> Option<String> {
        // Simple heuristics to detect language
        if code.contains("fn ") && code.contains("->") {
            Some("rust".to_string())
        } else if code.contains("def ") || code.contains("import ") {
            Some("python".to_string())
        } else if code.contains("function ") || code.contains("const ") || code.contains("let ") {
            Some("javascript".to_string())
        } else if code.contains("package ") && code.contains("func ") {
            Some("go".to_string())
        } else {
            None
        }
    }
}

impl Default for SyntaxHighlighter {
    fn default() -> Self {
        Self::new()
    }
}

/// Extract and highlight code blocks from markdown-style text
pub fn highlight_code_blocks(text: &str) -> String {
    let highlighter = SyntaxHighlighter::new();
    let mut result = String::new();
    let mut in_code_block = false;
    let mut code_buffer = String::new();
    let mut language: Option<String> = None;

    for line in text.lines() {
        if line.starts_with("```") {
            if in_code_block {
                // End of code block - highlight and add
                let highlighted = highlighter.highlight(&code_buffer, language.as_deref());
                result.push_str(&highlighted);
                result.push_str("\x1b[0m\n"); // Reset color
                code_buffer.clear();
                language = None;
                in_code_block = false;
            } else {
                // Start of code block
                in_code_block = true;
                language = line.strip_prefix("```").map(|s| s.trim().to_string());
                if language.as_ref().map(|s| s.is_empty()).unwrap_or(true) {
                    language = None;
                }
            }
        } else if in_code_block {
            code_buffer.push_str(line);
            code_buffer.push('\n');
        } else {
            result.push_str(line);
            result.push('\n');
        }
    }

    result
}
