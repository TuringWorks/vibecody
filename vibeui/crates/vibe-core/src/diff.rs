//! Diff generation and application

use anyhow::Result;
use similar::{ChangeTag, TextDiff};
use std::path::Path;

#[derive(Debug, Clone)]
pub struct DiffHunk {
    pub old_start: usize,
    pub old_count: usize,
    pub new_start: usize,
    pub new_count: usize,
    pub lines: Vec<DiffLine>,
}

#[derive(Debug, Clone)]
pub struct DiffLine {
    pub tag: DiffTag,
    pub content: String,
}

#[derive(Debug, Clone, PartialEq)]
pub enum DiffTag {
    Equal,
    Insert,
    Delete,
}

pub struct DiffEngine;

impl DiffEngine {
    pub fn generate_diff(original: &str, modified: &str) -> Vec<DiffHunk> {
        let diff = TextDiff::from_lines(original, modified);
        let mut hunks = Vec::new();
        let mut current_hunk: Option<DiffHunk> = None;
        
        let mut old_line = 1;
        let mut new_line = 1;

        for change in diff.iter_all_changes() {
            let tag = match change.tag() {
                ChangeTag::Equal => DiffTag::Equal,
                ChangeTag::Insert => DiffTag::Insert,
                ChangeTag::Delete => DiffTag::Delete,
            };

            let line = DiffLine {
                tag: tag.clone(),
                content: change.to_string(),
            };

            if current_hunk.is_none() {
                current_hunk = Some(DiffHunk {
                    old_start: old_line,
                    old_count: 0,
                    new_start: new_line,
                    new_count: 0,
                    lines: Vec::new(),
                });
            }

            if let Some(ref mut hunk) = current_hunk {
                hunk.lines.push(line);
                
                match tag {
                    DiffTag::Equal => {
                        hunk.old_count += 1;
                        hunk.new_count += 1;
                        old_line += 1;
                        new_line += 1;
                    }
                    DiffTag::Delete => {
                        hunk.old_count += 1;
                        old_line += 1;
                    }
                    DiffTag::Insert => {
                        hunk.new_count += 1;
                        new_line += 1;
                    }
                }
            }
        }

        if let Some(hunk) = current_hunk {
            hunks.push(hunk);
        }

        hunks
    }

    pub fn format_unified_diff(hunks: &[DiffHunk], old_path: &Path, new_path: &Path) -> String {
        let mut output = String::new();
        
        output.push_str(&format!("--- {}\n", old_path.display()));
        output.push_str(&format!("+++ {}\n", new_path.display()));

        for hunk in hunks {
            output.push_str(&format!(
                "@@ -{},{} +{},{} @@\n",
                hunk.old_start, hunk.old_count, hunk.new_start, hunk.new_count
            ));

            for line in &hunk.lines {
                let prefix = match line.tag {
                    DiffTag::Equal => " ",
                    DiffTag::Insert => "+",
                    DiffTag::Delete => "-",
                };
                output.push_str(&format!("{}{}", prefix, line.content));
            }
        }

        output
    }

    pub fn apply_diff(_original: &str, hunks: &[DiffHunk]) -> Result<String> {
        // Simple implementation: rebuild from hunks
        let mut new_content = String::new();
        for hunk in hunks {
            for line in &hunk.lines {
                match line.tag {
                    DiffTag::Equal | DiffTag::Insert => {
                        new_content.push_str(&line.content);
                    }
                    DiffTag::Delete => {
                        // Skip deleted lines
                    }
                }
            }
        }

        Ok(new_content)
    }
}
