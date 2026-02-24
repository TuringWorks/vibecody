//! Smart context builder for AI requests.
//!
//! Replaces the naive "inject full git diff" approach with a ranked,
//! token-budget-aware context that combines git status, relevant symbols,
//! and open file content — ordered by task relevance.

use crate::index::CodebaseIndex;
use std::path::Path;

/// Approximate characters per token (conservative estimate for LLM context).
const CHARS_PER_TOKEN: usize = 4;

// ── ContextBuilder ────────────────────────────────────────────────────────────

pub struct ContextBuilder<'a> {
    index: Option<&'a CodebaseIndex>,
    git_branch: Option<&'a str>,
    git_diff: Option<&'a str>,
    git_changed_files: Vec<String>,
    open_files: Vec<&'a Path>,
    /// Target token budget for the context block.
    token_budget: usize,
}

impl<'a> ContextBuilder<'a> {
    pub fn new() -> Self {
        Self {
            index: None,
            git_branch: None,
            git_diff: None,
            git_changed_files: Vec::new(),
            open_files: Vec::new(),
            token_budget: 8_000,
        }
    }

    pub fn with_index(mut self, index: &'a CodebaseIndex) -> Self {
        self.index = Some(index);
        self
    }

    pub fn with_git_branch(mut self, branch: &'a str) -> Self {
        self.git_branch = Some(branch);
        self
    }

    pub fn with_git_diff(mut self, diff: &'a str) -> Self {
        self.git_diff = Some(diff);
        self
    }

    pub fn with_git_changed_files(mut self, files: Vec<String>) -> Self {
        self.git_changed_files = files;
        self
    }

    pub fn with_open_files(mut self, files: Vec<&'a Path>) -> Self {
        self.open_files = files;
        self
    }

    pub fn with_token_budget(mut self, tokens: usize) -> Self {
        self.token_budget = tokens;
        self
    }

    /// Build a context string optimised for `task`.
    ///
    /// Priority order (stops when budget is exhausted):
    /// 1. Git branch + changed file list (always included)
    /// 2. Git diff (up to 25% of budget)
    /// 3. Top-ranked symbols relevant to the task (up to 30% of budget)
    /// 4. Contents of open files (remaining budget)
    pub fn build_for_task(&self, task: &str) -> String {
        let char_budget = self.token_budget * CHARS_PER_TOKEN;
        let mut parts: Vec<String> = Vec::new();
        let mut used_chars = 0usize;

        // ── 1. Git branch + changed files ─────────────────────────────────────
        if let Some(branch) = self.git_branch {
            let mut git_header = format!("## Git Context\nBranch: {}\n", branch);
            if !self.git_changed_files.is_empty() {
                git_header.push_str("Changed files:\n");
                for f in &self.git_changed_files {
                    git_header.push_str(&format!("  - {}\n", f));
                }
            }
            used_chars += git_header.len();
            parts.push(git_header);
        }

        // ── 2. Git diff (capped at 25% of budget) ────────────────────────────
        if let Some(diff) = self.git_diff {
            if !diff.is_empty() {
                let diff_budget = char_budget / 4;
                let diff_slice = if diff.len() > diff_budget {
                    // Truncate at last complete hunk boundary
                    let truncated = &diff[..diff_budget];
                    let last_newline = truncated.rfind('\n').unwrap_or(diff_budget);
                    &diff[..last_newline]
                } else {
                    diff
                };

                let diff_block = format!(
                    "\n## Git Diff{}\n```diff\n{}\n```\n",
                    if diff.len() > diff_budget { " (truncated)" } else { "" },
                    diff_slice
                );
                used_chars += diff_block.len();
                parts.push(diff_block);
            }
        }

        // ── 3. Relevant symbols from index ────────────────────────────────────
        if let Some(index) = self.index {
            let symbol_budget = char_budget * 30 / 100;
            let remaining = char_budget.saturating_sub(used_chars);
            let sym_limit = symbol_budget.min(remaining);

            if sym_limit > 200 {
                let symbols = index.relevant_symbols(task, 50);
                if !symbols.is_empty() {
                    let mut sym_block = String::from("\n## Relevant Symbols\n");
                    for sym in symbols {
                        let line = format!("  {}\n", sym.format_ref());
                        if sym_block.len() + line.len() > sym_limit {
                            break;
                        }
                        sym_block.push_str(&line);
                    }
                    let index_summary = format!(
                        "Index: {} files, {} symbols indexed.\n",
                        index.file_count(),
                        index.symbol_count()
                    );
                    sym_block.push_str(&index_summary);
                    used_chars += sym_block.len();
                    parts.push(sym_block);
                }
            }
        }

        // ── 4. Open file contents (remaining budget) ──────────────────────────
        if !self.open_files.is_empty() {
            let remaining = char_budget.saturating_sub(used_chars);
            if remaining > 200 {
                let per_file = remaining / self.open_files.len().max(1);
                let mut files_block = String::from("\n## Open Files\n");
                for path in &self.open_files {
                    if let Ok(content) = std::fs::read_to_string(path) {
                        let snippet = if content.len() > per_file {
                            // Take beginning + end to cover imports and exports
                            let half = per_file / 2;
                            let start = content.chars().take(half).collect::<String>();
                            let end_start = content.len().saturating_sub(half);
                            let end = &content[end_start..];
                            format!("{}\n...\n{}", start, end)
                        } else {
                            content
                        };
                        files_block.push_str(&format!(
                            "\n### {}\n```\n{}\n```\n",
                            path.display(),
                            snippet
                        ));
                        if files_block.len() >= remaining {
                            break;
                        }
                    }
                }
                if files_block.len() > "\n## Open Files\n".len() {
                    parts.push(files_block);
                }
            }
        }

        parts.join("")
    }
}

impl<'a> Default for ContextBuilder<'a> {
    fn default() -> Self {
        Self::new()
    }
}

// ── Tests ─────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_context_with_git() {
        let ctx = ContextBuilder::new()
            .with_git_branch("main")
            .with_git_diff("--- a/foo.rs\n+++ b/foo.rs\n@@ -1,1 +1,1 @@\n-old\n+new")
            .with_git_changed_files(vec!["src/foo.rs".to_string()])
            .build_for_task("refactor foo");

        assert!(ctx.contains("Branch: main"));
        assert!(ctx.contains("foo.rs"));
        assert!(ctx.contains("Git Diff"));
    }

    #[test]
    fn test_empty_context() {
        let ctx = ContextBuilder::new().build_for_task("do something");
        assert!(ctx.is_empty());
    }

    #[test]
    fn test_diff_truncation() {
        let big_diff = "x".repeat(100_000);
        let ctx = ContextBuilder::new()
            .with_git_branch("main")
            .with_git_diff(&big_diff)
            .with_token_budget(1_000)
            .build_for_task("task");
        // Output must be well under budget
        assert!(ctx.len() < 6_000);
    }
}
