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

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::Path;

    #[test]
    fn generate_diff_identical_files() {
        let hunks = DiffEngine::generate_diff("hello\n", "hello\n");
        // All lines are Equal — one hunk with only equal lines
        assert!(!hunks.is_empty());
        for hunk in &hunks {
            for line in &hunk.lines {
                assert_eq!(line.tag, DiffTag::Equal);
            }
        }
    }

    #[test]
    fn generate_diff_single_line_change() {
        let hunks = DiffEngine::generate_diff("aaa\n", "bbb\n");
        assert_eq!(hunks.len(), 1);
        let has_delete = hunks[0].lines.iter().any(|l| l.tag == DiffTag::Delete);
        let has_insert = hunks[0].lines.iter().any(|l| l.tag == DiffTag::Insert);
        assert!(has_delete);
        assert!(has_insert);
    }

    #[test]
    fn generate_diff_added_line() {
        let original = "line1\nline2\n";
        let modified = "line1\nline2\nline3\n";
        let hunks = DiffEngine::generate_diff(original, modified);
        let inserts: Vec<_> = hunks.iter()
            .flat_map(|h| h.lines.iter())
            .filter(|l| l.tag == DiffTag::Insert)
            .collect();
        assert_eq!(inserts.len(), 1);
        assert!(inserts[0].content.contains("line3"));
    }

    #[test]
    fn generate_diff_removed_line() {
        let original = "line1\nline2\nline3\n";
        let modified = "line1\nline3\n";
        let hunks = DiffEngine::generate_diff(original, modified);
        let deletes: Vec<_> = hunks.iter()
            .flat_map(|h| h.lines.iter())
            .filter(|l| l.tag == DiffTag::Delete)
            .collect();
        assert_eq!(deletes.len(), 1);
        assert!(deletes[0].content.contains("line2"));
    }

    #[test]
    fn generate_diff_empty_to_content() {
        let hunks = DiffEngine::generate_diff("", "new\n");
        let inserts: Vec<_> = hunks.iter()
            .flat_map(|h| h.lines.iter())
            .filter(|l| l.tag == DiffTag::Insert)
            .collect();
        assert_eq!(inserts.len(), 1);
    }

    #[test]
    fn generate_diff_content_to_empty() {
        let hunks = DiffEngine::generate_diff("old\n", "");
        let deletes: Vec<_> = hunks.iter()
            .flat_map(|h| h.lines.iter())
            .filter(|l| l.tag == DiffTag::Delete)
            .collect();
        assert_eq!(deletes.len(), 1);
    }

    #[test]
    fn format_unified_diff_has_header() {
        let hunks = DiffEngine::generate_diff("a\n", "b\n");
        let output = DiffEngine::format_unified_diff(&hunks, Path::new("old.rs"), Path::new("new.rs"));
        assert!(output.contains("--- old.rs"));
        assert!(output.contains("+++ new.rs"));
        assert!(output.contains("@@"));
    }

    #[test]
    fn format_unified_diff_shows_plus_minus() {
        let hunks = DiffEngine::generate_diff("old\n", "new\n");
        let output = DiffEngine::format_unified_diff(&hunks, Path::new("a"), Path::new("b"));
        assert!(output.contains("-old"));
        assert!(output.contains("+new"));
    }

    #[test]
    fn apply_diff_produces_modified_content() {
        let original = "line1\nline2\nline3\n";
        let modified = "line1\nchanged\nline3\n";
        let hunks = DiffEngine::generate_diff(original, modified);
        let result = DiffEngine::apply_diff(original, &hunks).unwrap();
        assert_eq!(result, modified);
    }

    #[test]
    fn apply_diff_add_line() {
        let original = "a\nb\n";
        let modified = "a\nb\nc\n";
        let hunks = DiffEngine::generate_diff(original, modified);
        let result = DiffEngine::apply_diff(original, &hunks).unwrap();
        assert_eq!(result, modified);
    }

    #[test]
    fn apply_diff_remove_line() {
        let original = "a\nb\nc\n";
        let modified = "a\nc\n";
        let hunks = DiffEngine::generate_diff(original, modified);
        let result = DiffEngine::apply_diff(original, &hunks).unwrap();
        assert_eq!(result, modified);
    }

    #[test]
    fn diff_hunk_line_counts() {
        let original = "a\nb\nc\n";
        let modified = "a\nB\nc\n";
        let hunks = DiffEngine::generate_diff(original, modified);
        assert_eq!(hunks.len(), 1);
        let h = &hunks[0];
        assert_eq!(h.old_count, 3); // a, b, c
        assert_eq!(h.new_count, 3); // a, B, c
    }
}
