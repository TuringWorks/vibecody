//! Project memory — loads VIBECLI.md / AGENTS.md / CLAUDE.md at multiple
//! hierarchy levels and merges them into a single system context, matching
//! Claude Code's CLAUDE.md hierarchical-merge behaviour.
//!
//! Loading order (lower overrides higher, all are merged):
//!   1. System:    /etc/vibecli/VIBECLI.md          (org-wide policy)
//!   2. User:      ~/.vibecli/VIBECLI.md             (personal rules)
//!   3. Project:   <git-root>/VIBECLI.md (or AGENTS.md / CLAUDE.md)
//!   4. Directory: <cwd>/VIBECLI.md (only if cwd ≠ project root)
//!
//! Global scratch pad:  ~/.vibecli/memory.md  (keeps backward compat)

use std::path::{Path, PathBuf};

// ── Memory File Names ─────────────────────────────────────────────────────────

const REPO_CANDIDATES: &[&str] = &["VIBECLI.md", "AGENTS.md", "CLAUDE.md", ".vibecli.md"];

// ── MemoryLevel ───────────────────────────────────────────────────────────────

/// A single level in the memory hierarchy.
#[derive(Debug, Clone)]
pub struct MemoryLevel {
    pub label: &'static str,
    pub path: PathBuf,
    pub content: String,
}

// ── ProjectMemory ─────────────────────────────────────────────────────────────

/// Merged view of all hierarchy levels.
#[derive(Debug, Default)]
pub struct ProjectMemory {
    /// Loaded levels in merge order (system → user → project → directory).
    pub levels: Vec<MemoryLevel>,
    /// Legacy: scratch-pad at `~/.vibecli/memory.md`.
    pub scratch: Option<String>,
}

impl ProjectMemory {
    /// Load memory from all hierarchy levels and merge them.
    pub fn load(cwd: &Path) -> Self {
        let mut levels = Vec::new();

        // 1. System-wide policy
        if let Some(level) = load_file("/etc/vibecli/VIBECLI.md", "system") {
            levels.push(level);
        }

        // 2. User-level instructions
        if let Some(home) = dirs::home_dir() {
            let user_path = home.join(".vibecli").join("VIBECLI.md");
            if let Some(level) = load_file(user_path, "user") {
                levels.push(level);
            }
        }

        // 3. Project-root level (walk up to .git boundary)
        if let Some(level) = load_repo_level(cwd, "project") {
            // 4. Directory-local (only if cwd differs from the project root)
            let project_root = level.path.parent().unwrap_or(cwd);
            if project_root != cwd {
                // cwd is a sub-directory — also look for a local file here
                if let Some(local) = load_dir_level(cwd, "directory") {
                    levels.push(level);
                    levels.push(local);
                } else {
                    levels.push(level);
                }
            } else {
                levels.push(level);
            }
        }

        // Scratch pad (backward-compat, shown separately)
        let scratch = dirs::home_dir()
            .map(|h| h.join(".vibecli").join("memory.md"))
            .and_then(|p| std::fs::read_to_string(&p).ok())
            .filter(|s| !s.trim().is_empty());

        Self { levels, scratch }
    }

    /// Return the merged memory content, or `None` if nothing was loaded.
    pub fn combined(&self) -> Option<String> {
        let mut parts: Vec<String> = self
            .levels
            .iter()
            .map(|l| format!("## {} Instructions ({})\n\n{}", title(l.label), l.path.display(), l.content))
            .collect();

        if let Some(s) = &self.scratch {
            parts.push(format!("## Personal Memory\n\n{}", s));
        }

        if parts.is_empty() {
            None
        } else {
            Some(parts.join("\n\n---\n\n"))
        }
    }

    /// Check if no memory was found at any level.
    pub fn is_empty(&self) -> bool {
        self.levels.is_empty() && self.scratch.is_none()
    }

    /// One-line summary of what was loaded.
    pub fn summary(&self) -> String {
        if self.is_empty() {
            return "No memory files found.".to_string();
        }
        let labels: Vec<String> = self
            .levels
            .iter()
            .map(|l| l.label.to_string())
            .collect();
        let mut s = format!("Memory: {} level(s) loaded [{}]", self.levels.len(), labels.join(", "));
        if self.scratch.is_some() {
            s.push_str(" + scratch pad");
        }
        s
    }

    /// Return the path for the repo-level memory file (existing or default).
    pub fn default_repo_path(cwd: &Path) -> PathBuf {
        cwd.join("VIBECLI.md")
    }

    /// Return the path for the global memory file (scratch pad).
    #[allow(dead_code)]
    pub fn global_path() -> Option<PathBuf> {
        dirs::home_dir().map(|h| h.join(".vibecli").join("memory.md"))
    }

    // ── Legacy accessors (backward compatibility) ──────────────────────────

    /// The first project-level content found, for backward-compat callers.
    #[allow(dead_code)]
    pub fn repo_content(&self) -> Option<&str> {
        self.levels.iter().find(|l| l.label == "project" || l.label == "directory")
            .map(|l| l.content.as_str())
    }

    /// The first project-level path found, for backward-compat callers.
    #[allow(dead_code)]
    pub fn repo_path(&self) -> Option<&Path> {
        self.levels.iter().find(|l| l.label == "project" || l.label == "directory")
            .map(|l| l.path.as_path())
    }
}

// ── Loading Helpers ───────────────────────────────────────────────────────────

fn title(label: &str) -> &str {
    match label {
        "system"    => "System",
        "user"      => "User",
        "project"   => "Project",
        "directory" => "Directory",
        _           => "Custom",
    }
}

fn load_file(path: impl Into<PathBuf>, label: &'static str) -> Option<MemoryLevel> {
    let path = path.into();
    let content = std::fs::read_to_string(&path).ok().filter(|s| !s.trim().is_empty())?;
    Some(MemoryLevel { label, path, content })
}

/// Walk up the directory tree from `cwd` looking for a repo memory file.
fn load_repo_level(cwd: &Path, label: &'static str) -> Option<MemoryLevel> {
    let mut dir = cwd.to_path_buf();
    loop {
        for candidate in REPO_CANDIDATES {
            let path = dir.join(candidate);
            if let Ok(content) = std::fs::read_to_string(&path) {
                if !content.trim().is_empty() {
                    return Some(MemoryLevel { label, path, content });
                }
            }
        }
        // Stop at .git boundary (that's the repo root)
        if dir.join(".git").exists() {
            break;
        }
        if !dir.pop() {
            break;
        }
    }
    None
}

/// Look for a directory-local memory file directly in `cwd` (not walking up).
fn load_dir_level(cwd: &Path, label: &'static str) -> Option<MemoryLevel> {
    for candidate in REPO_CANDIDATES {
        let path = cwd.join(candidate);
        if let Ok(content) = std::fs::read_to_string(&path) {
            if !content.trim().is_empty() {
                return Some(MemoryLevel { label, path, content });
            }
        }
    }
    None
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
        assert!(!mem.is_empty());
        assert!(mem.combined().is_some());

        let _ = fs::remove_file(&path);
    }

    #[test]
    fn test_empty_memory() {
        let dir = std::env::temp_dir().join("vibecli_empty_test");
        let _ = std::fs::create_dir_all(&dir);
        let mem = ProjectMemory::load(&dir);
        assert!(mem.combined().is_some() == !mem.is_empty());
    }

    #[test]
    fn test_combined_has_level_headers() {
        let dir = tempfile::tempdir().unwrap();
        let git_dir = dir.path().join(".git");
        fs::create_dir(&git_dir).unwrap();
        let md = dir.path().join("VIBECLI.md");
        fs::write(&md, "- test rule\n").unwrap();

        let mem = ProjectMemory::load(dir.path());
        let combined = mem.combined().unwrap();
        assert!(combined.contains("Project Instructions"));
        assert!(combined.contains("test rule"));
    }

    #[test]
    fn test_summary_shows_levels() {
        let dir = tempfile::tempdir().unwrap();
        let md = dir.path().join("VIBECLI.md");
        fs::write(&md, "rules\n").unwrap();

        let mem = ProjectMemory::load(dir.path());
        if !mem.is_empty() {
            assert!(mem.summary().contains("level"));
        }
    }

    #[test]
    fn test_default_project_memory_is_empty() {
        let mem = ProjectMemory::default();
        assert!(mem.is_empty());
        assert!(mem.levels.is_empty());
        assert!(mem.scratch.is_none());
    }

    #[test]
    fn test_default_combined_returns_none() {
        let mem = ProjectMemory::default();
        assert!(mem.combined().is_none());
    }

    #[test]
    fn test_default_summary_when_empty() {
        let mem = ProjectMemory::default();
        assert_eq!(mem.summary(), "No memory files found.");
    }

    #[test]
    fn test_default_repo_path() {
        let cwd = Path::new("/tmp/myproject");
        let path = ProjectMemory::default_repo_path(cwd);
        assert_eq!(path, PathBuf::from("/tmp/myproject/VIBECLI.md"));
    }

    #[test]
    fn test_global_path_returns_some() {
        // global_path returns Some on most systems (home dir exists)
        let path = ProjectMemory::global_path();
        if let Some(p) = path {
            assert!(p.to_string_lossy().contains("memory.md"));
        }
    }

    #[test]
    fn test_memory_level_struct() {
        let level = MemoryLevel {
            label: "project",
            path: PathBuf::from("/test/VIBECLI.md"),
            content: "some rules".to_string(),
        };
        assert_eq!(level.label, "project");
        assert_eq!(level.content, "some rules");
    }

    #[test]
    fn test_memory_level_clone() {
        let level = MemoryLevel {
            label: "user",
            path: PathBuf::from("/home/user/.vibecli/VIBECLI.md"),
            content: "user rules".to_string(),
        };
        let cloned = level.clone();
        assert_eq!(cloned.label, level.label);
        assert_eq!(cloned.path, level.path);
        assert_eq!(cloned.content, level.content);
    }

    #[test]
    fn test_repo_content_returns_none_when_empty() {
        let mem = ProjectMemory::default();
        assert!(mem.repo_content().is_none());
    }

    #[test]
    fn test_repo_path_returns_none_when_empty() {
        let mem = ProjectMemory::default();
        assert!(mem.repo_path().is_none());
    }

    #[test]
    fn test_combined_with_scratch_only() {
        let mem = ProjectMemory {
            levels: vec![],
            scratch: Some("My notes".to_string()),
        };
        assert!(!mem.is_empty());
        let combined = mem.combined().unwrap();
        assert!(combined.contains("Personal Memory"));
        assert!(combined.contains("My notes"));
    }

    #[test]
    fn test_combined_with_multiple_levels() {
        let mem = ProjectMemory {
            levels: vec![
                MemoryLevel {
                    label: "system",
                    path: PathBuf::from("/etc/vibecli/VIBECLI.md"),
                    content: "system rule".to_string(),
                },
                MemoryLevel {
                    label: "project",
                    path: PathBuf::from("/proj/VIBECLI.md"),
                    content: "project rule".to_string(),
                },
            ],
            scratch: None,
        };
        let combined = mem.combined().unwrap();
        assert!(combined.contains("System Instructions"));
        assert!(combined.contains("Project Instructions"));
        assert!(combined.contains("system rule"));
        assert!(combined.contains("project rule"));
        assert!(combined.contains("---"));
    }

    #[test]
    fn test_summary_with_scratch() {
        let mem = ProjectMemory {
            levels: vec![MemoryLevel {
                label: "user",
                path: PathBuf::from("/home/.vibecli/VIBECLI.md"),
                content: "stuff".to_string(),
            }],
            scratch: Some("notes".to_string()),
        };
        let summary = mem.summary();
        assert!(summary.contains("1 level(s)"));
        assert!(summary.contains("scratch pad"));
    }

    #[test]
    fn test_load_with_agents_md() {
        let dir = tempfile::tempdir().unwrap();
        let git_dir = dir.path().join(".git");
        fs::create_dir(&git_dir).unwrap();
        let md = dir.path().join("AGENTS.md");
        fs::write(&md, "agents rules\n").unwrap();

        let mem = ProjectMemory::load(dir.path());
        assert!(!mem.is_empty());
        let combined = mem.combined().unwrap();
        assert!(combined.contains("agents rules"));
    }

    #[test]
    fn test_load_with_claude_md() {
        let dir = tempfile::tempdir().unwrap();
        let git_dir = dir.path().join(".git");
        fs::create_dir(&git_dir).unwrap();
        let md = dir.path().join("CLAUDE.md");
        fs::write(&md, "claude instructions\n").unwrap();

        let mem = ProjectMemory::load(dir.path());
        assert!(!mem.is_empty());
        let combined = mem.combined().unwrap();
        assert!(combined.contains("claude instructions"));
    }
}
