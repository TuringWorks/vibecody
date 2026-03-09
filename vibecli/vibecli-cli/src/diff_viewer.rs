//! Diff viewing and application

use vibe_core::DiffEngine;
use crate::syntax::highlight_code_blocks;
use anyhow::Result;
use std::path::Path;
use std::fs;

pub struct DiffViewer;

impl DiffViewer {
    pub fn show_diff(file_path: &str, original: &str, modified: &str) -> Result<()> {
        let path = Path::new(file_path);
        let hunks = DiffEngine::generate_diff(original, modified);
        let diff_text = DiffEngine::format_unified_diff(&hunks, path, path);
        
        println!("\n📊 Diff for: {}\n", file_path);
        println!("{}", colorize_diff(&diff_text));
        println!();
        
        Ok(())
    }
    
    pub fn show_file_diff(file_path: &str) -> Result<()> {
        let path = Path::new(file_path);
        
        if !path.exists() {
            anyhow::bail!("File not found: {}", file_path);
        }
        
        let current_content = fs::read_to_string(path)?;
        
        // For now, we'll show the file content
        // In a real implementation, we'd compare with a previous version or AI suggestion
        println!("\n📄 Current content of: {}\n", file_path);
        println!("{}", highlight_code_blocks(&current_content));
        println!("\n💡 Tip: Use /generate to create modified version, then /diff to compare\n");
        
        Ok(())
    }
}

fn colorize_diff(diff: &str) -> String {
    let mut result = String::new();

    for line in diff.lines() {
        if line.starts_with('+') && !line.starts_with("+++") {
            result.push_str(&format!("\x1b[32m{}\x1b[0m\n", line)); // Green
        } else if line.starts_with('-') && !line.starts_with("---") {
            result.push_str(&format!("\x1b[31m{}\x1b[0m\n", line)); // Red
        } else if line.starts_with("@@") {
            result.push_str(&format!("\x1b[36m{}\x1b[0m\n", line)); // Cyan
        } else {
            result.push_str(&format!("{}\n", line));
        }
    }

    result
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn colorize_added_line_is_green() {
        let out = colorize_diff("+added line");
        assert!(out.contains("\x1b[32m+added line\x1b[0m"));
    }

    #[test]
    fn colorize_removed_line_is_red() {
        let out = colorize_diff("-removed line");
        assert!(out.contains("\x1b[31m-removed line\x1b[0m"));
    }

    #[test]
    fn colorize_hunk_header_is_cyan() {
        let out = colorize_diff("@@ -1,3 +1,4 @@");
        assert!(out.contains("\x1b[36m@@ -1,3 +1,4 @@\x1b[0m"));
    }

    #[test]
    fn colorize_plus_header_not_colored_green() {
        let out = colorize_diff("+++ b/file.rs");
        // +++ should NOT get green coloring, it's a file header
        assert!(!out.contains("\x1b[32m"));
        assert!(out.contains("+++ b/file.rs"));
    }

    #[test]
    fn colorize_minus_header_not_colored_red() {
        let out = colorize_diff("--- a/file.rs");
        assert!(!out.contains("\x1b[31m"));
        assert!(out.contains("--- a/file.rs"));
    }

    #[test]
    fn colorize_context_line_no_color() {
        let out = colorize_diff(" unchanged line");
        assert!(!out.contains("\x1b[32m"));
        assert!(!out.contains("\x1b[31m"));
        assert!(!out.contains("\x1b[36m"));
        assert!(out.contains(" unchanged line\n"));
    }

    #[test]
    fn colorize_empty_input() {
        let out = colorize_diff("");
        assert!(out.is_empty());
    }

    #[test]
    fn colorize_mixed_diff() {
        let diff = "--- a/foo.rs\n+++ b/foo.rs\n@@ -1,2 +1,3 @@\n context\n-old\n+new\n+added";
        let out = colorize_diff(diff);
        assert!(out.contains("\x1b[36m@@"));   // hunk cyan
        assert!(out.contains("\x1b[31m-old"));  // removed red
        assert!(out.contains("\x1b[32m+new"));  // added green
        assert!(out.contains("\x1b[32m+added")); // added green
    }

    #[test]
    fn show_diff_returns_ok() {
        let result = DiffViewer::show_diff("test.rs", "hello\n", "hello\nworld\n");
        assert!(result.is_ok());
    }

    #[test]
    fn show_diff_identical_content() {
        let result = DiffViewer::show_diff("same.rs", "same\n", "same\n");
        assert!(result.is_ok());
    }

    #[test]
    fn show_diff_empty_original() {
        let result = DiffViewer::show_diff("new.rs", "", "new content\n");
        assert!(result.is_ok());
    }

    #[test]
    fn show_diff_empty_modified() {
        let result = DiffViewer::show_diff("del.rs", "old content\n", "");
        assert!(result.is_ok());
    }

    #[test]
    fn show_diff_both_empty() {
        let result = DiffViewer::show_diff("empty.rs", "", "");
        assert!(result.is_ok());
    }

    #[test]
    fn colorize_multiple_hunk_headers() {
        let diff = "@@ -1,2 +1,2 @@\n context\n-old\n+new\n@@ -10,3 +10,4 @@\n more context";
        let out = colorize_diff(diff);
        // Two hunk headers, both cyan
        let cyan_count = out.matches("\x1b[36m@@").count();
        assert_eq!(cyan_count, 2);
    }

    #[test]
    fn colorize_only_plus_sign_line() {
        // A line that is just "+" should be colored green
        let out = colorize_diff("+");
        assert!(out.contains("\x1b[32m+\x1b[0m"));
    }

    #[test]
    fn colorize_only_minus_sign_line() {
        // A line that is just "-" should be colored red
        let out = colorize_diff("-");
        assert!(out.contains("\x1b[31m-\x1b[0m"));
    }

    #[test]
    fn show_file_diff_nonexistent_file() {
        let result = DiffViewer::show_file_diff("/nonexistent/path/to/file.rs");
        assert!(result.is_err());
        let err_msg = result.unwrap_err().to_string();
        assert!(err_msg.contains("File not found"));
    }

    #[test]
    fn colorize_preserves_line_count() {
        let diff = "line1\n+added\n-removed\n@@ hunk @@\ncontext";
        let out = colorize_diff(diff);
        // Each input line becomes a line in output (with \n appended)
        let output_lines: Vec<&str> = out.lines().collect();
        assert_eq!(output_lines.len(), 5);
    }
}
