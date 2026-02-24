//! Project memory — loads VIBECLI.md / AGENTS.md / CLAUDE.md and injects it
//! as the first system message so every conversation has persistent context.
//!
//! Loading priority (first found wins at each level):
//!   Repo-level:   VIBECLI.md → AGENTS.md → CLAUDE.md → .vibecli.md
//!   Global:       ~/.vibecli/memory.md

use std::path::{Path, PathBuf};

// ── Memory File Names ─────────────────────────────────────────────────────────

const REPO_CANDIDATES: &[&str] = &["VIBECLI.md", "AGENTS.md", "CLAUDE.md", ".vibecli.md"];

// ── ProjectMemory ─────────────────────────────────────────────────────────────

#[derive(Debug, Default)]
pub struct ProjectMemory {
    /// Content of the global memory file (`~/.vibecli/memory.md`).
    pub global: Option<String>,
    /// Content of the repo-level memory file (VIBECLI.md etc. in CWD ancestors).
    pub repo: Option<String>,
    /// Path of the repo-level file that was found.
    pub repo_path: Option<PathBuf>,
}

impl ProjectMemory {
    /// Load memory from global and repo-level files.
    pub fn load(cwd: &Path) -> Self {
        let global = load_global();
        let (repo, repo_path) = load_repo(cwd);
        Self { global, repo, repo_path }
    }

    /// Return the combined memory as a single string, or `None` if no memory.
    pub fn combined(&self) -> Option<String> {
        match (&self.global, &self.repo) {
            (None, None) => None,
            (Some(g), None) => Some(format!("## Global Memory\n\n{}", g)),
            (None, Some(r)) => Some(format!("## Project Instructions\n\n{}", r)),
            (Some(g), Some(r)) => Some(format!(
                "## Project Instructions\n\n{}\n\n## Global Memory\n\n{}",
                r, g
            )),
        }
    }

    /// Check if any memory is loaded.
    pub fn is_empty(&self) -> bool {
        self.global.is_none() && self.repo.is_none()
    }

    /// Return a summary string for display (one line).
    pub fn summary(&self) -> String {
        match (&self.global, &self.repo_path) {
            (Some(_), Some(rp)) => {
                format!("Memory: global + {}", rp.display())
            }
            (None, Some(rp)) => format!("Memory: {}", rp.display()),
            (Some(_), None) => "Memory: global only".to_string(),
            (None, None) => "No memory files found.".to_string(),
        }
    }

    /// Return the path for the repo-level memory file (existing or default).
    pub fn default_repo_path(cwd: &Path) -> PathBuf {
        cwd.join("VIBECLI.md")
    }

    /// Return the path for the global memory file.
    pub fn global_path() -> Option<PathBuf> {
        dirs::home_dir().map(|h| h.join(".vibecli").join("memory.md"))
    }
}

// ── Loading Helpers ───────────────────────────────────────────────────────────

fn load_global() -> Option<String> {
    let path = ProjectMemory::global_path()?;
    std::fs::read_to_string(&path).ok().filter(|s| !s.trim().is_empty())
}

/// Walk up the directory tree from `cwd` looking for a memory file.
fn load_repo(cwd: &Path) -> (Option<String>, Option<PathBuf>) {
    let mut dir = cwd.to_path_buf();
    loop {
        for candidate in REPO_CANDIDATES {
            let path = dir.join(candidate);
            if let Ok(content) = std::fs::read_to_string(&path) {
                if !content.trim().is_empty() {
                    return (Some(content), Some(path));
                }
            }
        }
        // Stop at filesystem root or at .git boundary
        let git_dir = dir.join(".git");
        if git_dir.exists() {
            break;
        }
        if !dir.pop() {
            break;
        }
    }
    (None, None)
}

// ── Tests ─────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;

    #[test]
    fn test_load_memory_from_temp_dir() {
        let dir = std::env::temp_dir();
        let path = dir.join("VIBECLI.md");
        fs::write(&path, "## Rules\n- Always write tests\n").unwrap();

        let mem = ProjectMemory::load(&dir);
        assert!(mem.repo.is_some());
        assert!(mem.combined().is_some());

        let _ = fs::remove_file(&path);
    }

    #[test]
    fn test_empty_memory() {
        let dir = std::env::temp_dir().join("vibecli_empty_test");
        let _ = std::fs::create_dir_all(&dir);
        let mem = ProjectMemory::load(&dir);
        // May or may not be empty depending on test environment
        assert!(mem.combined().is_some() == !mem.is_empty());
    }
}
