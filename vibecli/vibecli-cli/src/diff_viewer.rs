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
