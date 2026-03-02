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

/// Extract and highlight code blocks from markdown-style text.
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

#[cfg(test)]
mod tests {
    use super::*;

    // ── detect_language ──────────────────────────────────────────────────────

    #[test]
    fn detect_language_rust() {
        assert_eq!(
            SyntaxHighlighter::detect_language("fn main() -> () {}"),
            Some("rust".to_string())
        );
    }

    #[test]
    fn detect_language_python() {
        assert_eq!(
            SyntaxHighlighter::detect_language("def hello():\n    pass"),
            Some("python".to_string())
        );
    }

    #[test]
    fn detect_language_python_import() {
        assert_eq!(
            SyntaxHighlighter::detect_language("import os\nos.getcwd()"),
            Some("python".to_string())
        );
    }

    #[test]
    fn detect_language_javascript_function() {
        assert_eq!(
            SyntaxHighlighter::detect_language("function greet(name) { return name; }"),
            Some("javascript".to_string())
        );
    }

    #[test]
    fn detect_language_javascript_const() {
        assert_eq!(
            SyntaxHighlighter::detect_language("const x = 42;"),
            Some("javascript".to_string())
        );
    }

    #[test]
    fn detect_language_javascript_let() {
        assert_eq!(
            SyntaxHighlighter::detect_language("let y = 'hello';"),
            Some("javascript".to_string())
        );
    }

    #[test]
    fn detect_language_go() {
        assert_eq!(
            SyntaxHighlighter::detect_language("package main\nfunc main() {}"),
            Some("go".to_string())
        );
    }

    #[test]
    fn detect_language_none_for_prose() {
        assert_eq!(
            SyntaxHighlighter::detect_language("This is just some plain English text."),
            None
        );
    }

    #[test]
    fn detect_language_empty() {
        assert_eq!(SyntaxHighlighter::detect_language(""), None);
    }

    // ── SyntaxHighlighter::new / highlight ───────────────────────────────────

    #[test]
    fn highlighter_new_does_not_panic() {
        let _h = SyntaxHighlighter::new();
    }

    #[test]
    fn highlight_plain_text_returns_non_empty() {
        let h = SyntaxHighlighter::new();
        let out = h.highlight("hello world\n", None);
        assert!(out.contains("hello world"));
    }

    #[test]
    fn highlight_with_language_returns_ansi() {
        let h = SyntaxHighlighter::new();
        let out = h.highlight("fn main() {}\n", Some("rust"));
        // ANSI codes start with ESC
        assert!(out.contains('\x1b'));
    }

    #[test]
    fn highlight_unknown_language_falls_back() {
        let h = SyntaxHighlighter::new();
        // Unknown language should not panic; falls back to plain text
        let out = h.highlight("hello\n", Some("nonexistent_language_xyz"));
        assert!(out.contains("hello"));
    }

    #[test]
    fn highlight_ranges_returns_non_empty() {
        let h = SyntaxHighlighter::new();
        let ranges = h.highlight_ranges("let x = 42;\n", Some("js"));
        assert!(!ranges.is_empty());
    }

    // ── highlight_code_blocks ────────────────────────────────────────────────

    #[test]
    fn highlight_code_blocks_no_code() {
        let input = "Just some text\nand more text\n";
        let out = highlight_code_blocks(input);
        assert!(out.contains("Just some text"));
        assert!(out.contains("and more text"));
    }

    #[test]
    fn highlight_code_blocks_with_fenced_block() {
        let input = "Before\n```rust\nfn foo() {}\n```\nAfter\n";
        let out = highlight_code_blocks(input);
        assert!(out.contains("Before"));
        assert!(out.contains("After"));
        // The code block should be highlighted (contains ANSI reset at minimum)
        assert!(out.contains("\x1b[0m"));
    }

    #[test]
    fn highlight_code_blocks_no_language_tag() {
        let input = "```\nhello world\n```\n";
        let out = highlight_code_blocks(input);
        assert!(out.contains("hello world") || out.contains('\x1b'));
    }

    #[test]
    fn highlight_code_blocks_unclosed_fence_no_panic() {
        let input = "```rust\nfn main() {}\n";
        // Should not panic even if the fence is never closed
        let _out = highlight_code_blocks(input);
    }

    #[test]
    fn highlight_code_blocks_empty_input() {
        let out = highlight_code_blocks("");
        assert!(out.is_empty());
    }

    #[test]
    fn highlight_code_blocks_multiple_blocks() {
        let input = "Text\n```py\nx = 1\n```\nMiddle\n```js\nvar y = 2;\n```\nEnd\n";
        let out = highlight_code_blocks(input);
        assert!(out.contains("Text"));
        assert!(out.contains("Middle"));
        assert!(out.contains("End"));
    }
}
