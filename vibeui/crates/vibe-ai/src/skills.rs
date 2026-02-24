//! Skills system — context-aware capability definitions that activate
//! automatically when a task matches their trigger keywords.
//!
//! Skills live in `.vibecli/skills/` (repo-local) or `~/.vibecli/skills/`
//! (global). Each skill is a Markdown file with a YAML front-matter header.
//!
//! # Example skill file (`.vibecli/skills/rust-safety.md`)
//!
//! ```markdown
//! ---
//! name: rust-safety
//! description: Activated when working on Rust code safety, memory, or correctness
//! triggers: ["unsafe", "memory", "panic", "lifetime", "borrow"]
//! tools_allowed: [read_file, write_file, bash]
//! ---
//!
//! When editing Rust code, always:
//! 1. Check for `unwrap()` calls that should be `?` or `expect()`
//! 2. Verify all `unsafe` blocks have a `// SAFETY:` comment
//! 3. After writing, run `cargo clippy -- -D warnings` via bash tool
//! ```

use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};

// ── Skill ─────────────────────────────────────────────────────────────────────

/// A single skill definition loaded from a `.md` file.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Skill {
    /// Canonical name (from front-matter or filename).
    pub name: String,
    /// Human-readable description.
    pub description: String,
    /// Keywords/phrases that trigger this skill.
    pub triggers: Vec<String>,
    /// Tool names this skill permits (empty = no restriction).
    pub tools_allowed: Vec<String>,
    /// The skill body (everything after the front-matter separator `---`).
    pub content: String,
    /// Path this skill was loaded from.
    pub source: PathBuf,
}

impl Skill {
    /// Returns `true` if any trigger appears (case-insensitively) in `text`.
    pub fn matches(&self, text: &str) -> bool {
        let lower = text.to_lowercase();
        self.triggers.iter().any(|t| lower.contains(&t.to_lowercase()))
    }
}

// ── SkillLoader ───────────────────────────────────────────────────────────────

/// Discovers and loads skill files from one or more directories.
pub struct SkillLoader {
    dirs: Vec<PathBuf>,
}

impl SkillLoader {
    /// Create a loader that searches the standard locations:
    /// - `<workspace>/.vibecli/skills/`
    /// - `~/.vibecli/skills/` (global)
    pub fn new(workspace_root: &Path) -> Self {
        let mut search_dirs = vec![workspace_root.join(".vibecli").join("skills")];
        if let Ok(home) = std::env::var("HOME") {
            search_dirs.push(PathBuf::from(home).join(".vibecli").join("skills"));
        }
        Self { dirs: search_dirs }
    }

    /// Create a loader with explicit directories (for testing).
    pub fn with_dirs(dirs: Vec<PathBuf>) -> Self {
        Self { dirs }
    }

    /// Load all skill files from the configured directories.
    pub fn load_all(&self) -> Vec<Skill> {
        let mut skills = Vec::new();
        for dir in &self.dirs {
            if let Ok(entries) = std::fs::read_dir(dir) {
                for entry in entries.flatten() {
                    let path = entry.path();
                    if path.extension().and_then(|e| e.to_str()) == Some("md") {
                        match load_skill_file(&path) {
                            Ok(skill) => skills.push(skill),
                            Err(e) => tracing::warn!("Failed to load skill {}: {}", path.display(), e),
                        }
                    }
                }
            }
        }
        skills
    }

    /// Return all skills whose triggers match `text`.
    pub fn matching(&self, text: &str) -> Vec<Skill> {
        self.load_all().into_iter().filter(|s| s.matches(text)).collect()
    }
}

// ── File parser ───────────────────────────────────────────────────────────────

fn load_skill_file(path: &Path) -> anyhow::Result<Skill> {
    let raw = std::fs::read_to_string(path)?;

    // Split optional YAML front-matter between `---` delimiters
    let (front_matter_lines, body) = split_front_matter(&raw);

    // Parse front-matter with a minimal key: value parser (avoids serde_yaml dep)
    let mut fm_name = String::new();
    let mut fm_description = String::new();
    let mut fm_triggers: Vec<String> = Vec::new();
    let mut fm_tools_allowed: Vec<String> = Vec::new();

    for line in &front_matter_lines {
        let line = line.trim();
        if let Some(rest) = line.strip_prefix("name:") {
            fm_name = rest.trim().trim_matches('"').to_string();
        } else if let Some(rest) = line.strip_prefix("description:") {
            fm_description = rest.trim().trim_matches('"').to_string();
        } else if let Some(rest) = line.strip_prefix("triggers:") {
            fm_triggers = parse_yaml_list(rest.trim());
        } else if let Some(rest) = line.strip_prefix("tools_allowed:") {
            fm_tools_allowed = parse_yaml_list(rest.trim());
        }
    }

    // Use filename stem as fallback name
    let name = if fm_name.is_empty() {
        path.file_stem()
            .and_then(|s| s.to_str())
            .unwrap_or("unknown")
            .to_string()
    } else {
        fm_name
    };

    Ok(Skill {
        name,
        description: fm_description,
        triggers: fm_triggers,
        tools_allowed: fm_tools_allowed,
        content: body.trim().to_string(),
        source: path.to_path_buf(),
    })
}

/// Parse a YAML-style inline list: `[foo, bar, "baz"]` → `["foo","bar","baz"]`.
fn parse_yaml_list(s: &str) -> Vec<String> {
    let inner = s.trim_start_matches('[').trim_end_matches(']');
    inner
        .split(',')
        .map(|item| item.trim().trim_matches('"').trim_matches('\'').to_string())
        .filter(|s| !s.is_empty())
        .collect()
}

/// Split `---\n<front-matter>\n---\n<body>` format.
///
/// Returns `(front_matter_lines, body_str)`.
fn split_front_matter(raw: &str) -> (Vec<String>, String) {
    let lines: Vec<&str> = raw.lines().collect();
    if lines.first().map(|l| l.trim()) != Some("---") {
        return (vec![], raw.to_string());
    }

    // Find the closing `---`
    let close = lines[1..].iter().position(|l| l.trim() == "---");
    if let Some(close_idx) = close {
        let fm: Vec<String> = lines[1..close_idx + 1].iter().map(|s| s.to_string()).collect();
        let body = lines[close_idx + 2..].join("\n");
        (fm, body)
    } else {
        (vec![], raw.to_string())
    }
}

// ── Tests ─────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    fn make_skill(triggers: &[&str], content: &str) -> Skill {
        Skill {
            name: "test".to_string(),
            description: "test skill".to_string(),
            triggers: triggers.iter().map(|s| s.to_string()).collect(),
            tools_allowed: vec![],
            content: content.to_string(),
            source: PathBuf::from("test.md"),
        }
    }

    #[test]
    fn skill_matches_trigger_case_insensitive() {
        let skill = make_skill(&["unsafe", "memory"], "content");
        assert!(skill.matches("Fix the unsafe block"));
        assert!(skill.matches("Memory leak in allocation"));
        assert!(!skill.matches("Add new feature"));
    }

    #[test]
    fn skill_no_match_when_triggers_empty() {
        let skill = make_skill(&[], "content");
        assert!(!skill.matches("anything"));
    }

    #[test]
    fn split_front_matter_valid() {
        let raw = "---\nname: test\ntriggers: [foo]\n---\nbody text";
        let (fm, body) = split_front_matter(raw);
        assert!(!fm.is_empty(), "expected front-matter lines");
        assert_eq!(body.trim(), "body text");
    }

    #[test]
    fn split_front_matter_no_delimiter() {
        let raw = "just plain content\nno front matter";
        let (fm, body) = split_front_matter(raw);
        assert!(fm.is_empty(), "expected no front-matter");
        assert_eq!(body, raw);
    }

    #[test]
    fn load_skill_file_from_tempdir() {
        let dir = tempfile::tempdir().unwrap();
        let skill_path = dir.path().join("my-skill.md");
        std::fs::write(
            &skill_path,
            "---\nname: my-skill\ndescription: A test skill\ntriggers: [test, unit]\ntools_allowed: [read_file]\n---\nAlways write tests.",
        ).unwrap();

        let skill = load_skill_file(&skill_path).unwrap();
        assert_eq!(skill.name, "my-skill");
        assert_eq!(skill.triggers, vec!["test", "unit"]);
        assert!(skill.content.contains("Always write tests"));
    }

    #[test]
    fn skill_loader_loads_from_dir() {
        let dir = tempfile::tempdir().unwrap();
        let skills_dir = dir.path().join("skills");
        std::fs::create_dir(&skills_dir).unwrap();
        std::fs::write(
            skills_dir.join("rust-safety.md"),
            "---\nname: rust-safety\ntriggers: [unsafe, lifetime]\n---\nCheck for unsafe blocks.",
        ).unwrap();

        let loader = SkillLoader::with_dirs(vec![skills_dir]);
        let skills = loader.load_all();
        assert_eq!(skills.len(), 1);
        assert_eq!(skills[0].name, "rust-safety");

        let matching = loader.matching("Fix unsafe code");
        assert_eq!(matching.len(), 1);

        let non_matching = loader.matching("Add a new feature");
        assert_eq!(non_matching.len(), 0);
    }
}
