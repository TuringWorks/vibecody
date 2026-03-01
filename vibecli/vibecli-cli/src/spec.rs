#![allow(dead_code)]
//! Spec-driven development system.
//!
//! Specs are markdown files with TOML front-matter stored in `.vibecli/specs/`.
//! Each spec contains user stories, acceptance criteria, technical design, and a task list.
//!
//! # Usage
//! - `/spec new <name>` — create an empty spec
//! - `/spec list`       — list all specs
//! - `/spec show <name>` — display spec with task checklist
//! - `/spec run <name>` — show pending tasks for agent
//! - `/spec done <name> <task-id>` — mark task complete

use anyhow::Result;
use serde::{Deserialize, Serialize};
use std::fmt;
use std::path::{Path, PathBuf};
use walkdir::WalkDir;

// ── SpecStatus ────────────────────────────────────────────────────────────────

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum SpecStatus {
    Draft,
    Approved,
    InProgress,
    Done,
}

impl Default for SpecStatus {
    fn default() -> Self {
        SpecStatus::Draft
    }
}

impl fmt::Display for SpecStatus {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            SpecStatus::Draft => write!(f, "draft"),
            SpecStatus::Approved => write!(f, "approved"),
            SpecStatus::InProgress => write!(f, "in-progress"),
            SpecStatus::Done => write!(f, "done"),
        }
    }
}

// ── SpecTask ──────────────────────────────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SpecTask {
    pub id: u32,
    pub description: String,
    pub done: bool,
}

impl SpecTask {
    pub fn new(id: u32, description: impl Into<String>) -> Self {
        Self { id, description: description.into(), done: false }
    }
}

// ── Spec ─────────────────────────────────────────────────────────────────────

/// A spec document loaded from `.vibecli/specs/<name>.md`.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Spec {
    pub name: String,
    pub status: SpecStatus,
    /// Original natural-language requirements.
    pub requirements: String,
    /// Generated task list.
    pub tasks: Vec<SpecTask>,
    /// Full markdown body (design, user stories, etc.).
    pub body: String,
    /// Source path.
    pub source: PathBuf,
}

impl Spec {
    /// Number of completed tasks.
    pub fn completed(&self) -> usize {
        self.tasks.iter().filter(|t| t.done).count()
    }

    /// Number of pending tasks.
    pub fn pending(&self) -> usize {
        self.tasks.iter().filter(|t| !t.done).count()
    }

    /// Build a string ready for serialization back to disk.
    pub fn to_file_content(&self) -> String {
        let tasks_md: String = self.tasks.iter().map(|t| {
            let check = if t.done { "x" } else { " " };
            format!("- [{}] **{}**: {}\n", check, t.id, t.description)
        }).collect();

        // Strip any existing ## Tasks section from body to avoid duplication
        let clean_body = if let Some(pos) = self.body.find("\n## Tasks") {
            self.body[..pos].trim_end().to_string()
        } else if self.body.trim_start().starts_with("## Tasks") {
            String::new()
        } else {
            self.body.clone()
        };

        format!(
            "---\nname: {}\nstatus: {}\nrequirements: {}\n---\n\n{}\n\n## Tasks\n\n{}",
            self.name, self.status, self.requirements, clean_body, tasks_md
        )
    }

    /// Build a context string to inject into agent system prompt.
    pub fn context_string(&self) -> String {
        let pending: Vec<String> = self.tasks.iter()
            .filter(|t| !t.done)
            .map(|t| format!("{}. {}", t.id, t.description))
            .collect();
        format!(
            "=== Spec: {} (status: {}) ===\nRequirements: {}\n\nPending tasks:\n{}\n",
            self.name, self.status, self.requirements,
            if pending.is_empty() { "All done!".to_string() } else { pending.join("\n") }
        )
    }
}

// ── SpecManager ───────────────────────────────────────────────────────────────

/// Loads and saves spec files from `.vibecli/specs/`.
pub struct SpecManager {
    specs_dir: PathBuf,
}

impl SpecManager {
    /// Create a manager rooted at `workspace_root/.vibecli/specs/`.
    pub fn for_workspace(workspace_root: &Path) -> Self {
        Self { specs_dir: workspace_root.join(".vibecli").join("specs") }
    }

    /// Create a manager rooted at a given directory.
    pub fn new(specs_dir: PathBuf) -> Self {
        Self { specs_dir }
    }

    /// Ensure the specs directory exists.
    pub fn init(&self) -> Result<()> {
        std::fs::create_dir_all(&self.specs_dir)?;
        Ok(())
    }

    /// List all spec names (without extension).
    pub fn list(&self) -> Vec<String> {
        if !self.specs_dir.is_dir() {
            return vec![];
        }
        let mut names: Vec<String> = WalkDir::new(&self.specs_dir)
            .max_depth(1)
            .into_iter()
            .filter_map(|e| e.ok())
            .filter(|e| {
                e.file_type().is_file()
                    && e.path().extension().and_then(|x| x.to_str()) == Some("md")
            })
            .filter_map(|e| {
                e.path()
                    .file_stem()
                    .and_then(|s| s.to_str())
                    .map(|s| s.to_string())
            })
            .collect();
        names.sort();
        names
    }

    /// Load a spec by name.
    pub fn load(&self, name: &str) -> Result<Spec> {
        let path = self.specs_dir.join(format!("{}.md", name));
        let raw = std::fs::read_to_string(&path)
            .map_err(|e| anyhow::anyhow!("Cannot read spec '{}': {}", name, e))?;
        Self::parse(&path, name, &raw)
    }

    /// Save a spec back to disk.
    pub fn save(&self, spec: &Spec) -> Result<()> {
        std::fs::create_dir_all(&self.specs_dir)?;
        let path = self.specs_dir.join(format!("{}.md", spec.name));
        std::fs::write(&path, spec.to_file_content())?;
        Ok(())
    }

    /// Mark a task as done and save.
    pub fn complete_task(&self, name: &str, task_id: u32) -> Result<()> {
        let mut spec = self.load(name)?;
        if let Some(task) = spec.tasks.iter_mut().find(|t| t.id == task_id) {
            task.done = true;
        } else {
            anyhow::bail!("Task {} not found in spec '{}'", task_id, name);
        }
        // Update status if all done
        if spec.tasks.iter().all(|t| t.done) {
            spec.status = SpecStatus::Done;
        } else if spec.tasks.iter().any(|t| t.done) && spec.status == SpecStatus::Approved {
            spec.status = SpecStatus::InProgress;
        }
        self.save(&spec)
    }

    /// Create a new empty spec.
    pub fn create_empty(&self, name: &str, requirements: &str) -> Result<Spec> {
        let spec = Spec {
            name: name.to_string(),
            status: SpecStatus::Draft,
            requirements: requirements.to_string(),
            tasks: vec![],
            body: String::new(),
            source: self.specs_dir.join(format!("{}.md", name)),
        };
        self.save(&spec)?;
        Ok(spec)
    }

    /// Parse a spec from raw file contents.
    fn parse(path: &Path, name: &str, raw: &str) -> Result<Spec> {
        let mut status = SpecStatus::Draft;
        let mut requirements = String::new();
        let mut body = raw.to_string();
        let mut tasks: Vec<SpecTask> = vec![];

        // Parse front-matter
        if raw.starts_with("---") {
            let after_open = raw[3..].trim_start_matches('\n');
            if let Some(close_pos) = after_open.find("\n---") {
                let fm = &after_open[..close_pos];
                body = after_open[close_pos..].trim_start_matches("\n---").trim_start().to_string();
                for line in fm.lines() {
                    if let Some((k, v)) = line.split_once(':') {
                        let val = v.trim().trim_matches('"').trim_matches('\'');
                        match k.trim() {
                            "status" => {
                                status = match val {
                                    "approved" => SpecStatus::Approved,
                                    "in-progress" => SpecStatus::InProgress,
                                    "done" => SpecStatus::Done,
                                    _ => SpecStatus::Draft,
                                }
                            }
                            "requirements" => requirements = val.to_string(),
                            _ => {}
                        }
                    }
                }
            }
        }

        // Parse task list from body
        for line in body.lines() {
            let line = line.trim();
            let (rest, done) = if let Some(r) = line.strip_prefix("- [x] ") {
                (r.trim(), true)
            } else if let Some(r) = line.strip_prefix("- [ ] ") {
                (r.trim(), false)
            } else {
                continue;
            };
            if rest.is_empty() { continue; }
            let (id, desc) = if let Some(stripped) = rest.strip_prefix("**") {
                if let Some((id_part, desc_part)) = stripped.split_once("**:") {
                    (id_part.parse::<u32>().unwrap_or(tasks.len() as u32 + 1), desc_part.trim().to_string())
                } else {
                    (tasks.len() as u32 + 1, rest.to_string())
                }
            } else {
                (tasks.len() as u32 + 1, rest.to_string())
            };
            tasks.push(SpecTask { id, description: desc, done });
        }

        Ok(Spec {
            name: name.to_string(),
            status,
            requirements,
            tasks,
            body,
            source: path.to_path_buf(),
        })
    }
}

// ── Spec generation prompt ────────────────────────────────────────────────────

/// Build the LLM prompt to generate a spec from requirements.
pub fn spec_generation_prompt(requirements: &str) -> String {
    format!(
        r#"You are a software architect. Generate a spec document for the following requirements.

Requirements: {requirements}

Output a markdown document with TOML front-matter followed by these sections:
1. ## User Stories (Given/When/Then format)
2. ## Acceptance Criteria (bullet list)
3. ## Technical Design (architecture decisions, key files to change)
4. ## Tasks (numbered, atomic, implementable — each prefixed like: `- [ ] **1**: task description`)

Front-matter format:
```
---
name: <snake_case_name>
status: draft
requirements: {requirements}
---
```

Be concise. Generate 5-10 atomic tasks that fully cover the requirements."#
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    #[test]
    fn spec_to_file_content_roundtrip() {
        let tmp = TempDir::new().unwrap();
        let mgr = SpecManager::new(tmp.path().to_path_buf());

        let mut spec = mgr.create_empty("my_feature", "Add dark mode").unwrap();
        spec.tasks.push(SpecTask::new(1, "Update CSS variables"));
        spec.tasks.push(SpecTask::new(2, "Add toggle button"));
        spec.body = "## Design\nUse CSS custom properties.".to_string();
        mgr.save(&spec).unwrap();

        let loaded = mgr.load("my_feature").unwrap();
        assert_eq!(loaded.name, "my_feature");
        assert_eq!(loaded.tasks.len(), 2);
        assert!(!loaded.tasks[0].done);
    }

    #[test]
    fn complete_task_updates_status() {
        let tmp = TempDir::new().unwrap();
        let mgr = SpecManager::new(tmp.path().to_path_buf());

        let mut spec = mgr.create_empty("feature", "Test").unwrap();
        spec.status = SpecStatus::Approved;
        spec.tasks.push(SpecTask::new(1, "Step one"));
        mgr.save(&spec).unwrap();

        mgr.complete_task("feature", 1).unwrap();
        let loaded = mgr.load("feature").unwrap();
        assert!(loaded.tasks[0].done);
        assert_eq!(loaded.status, SpecStatus::Done);
    }

    #[test]
    fn spec_list() {
        let tmp = TempDir::new().unwrap();
        let mgr = SpecManager::new(tmp.path().to_path_buf());
        mgr.create_empty("alpha", "A").unwrap();
        mgr.create_empty("beta", "B").unwrap();

        let names = mgr.list();
        assert_eq!(names, vec!["alpha", "beta"]);
    }

    #[test]
    fn context_string_shows_pending() {
        let spec = Spec {
            name: "feat".to_string(),
            status: SpecStatus::Approved,
            requirements: "Do X".to_string(),
            tasks: vec![
                SpecTask { id: 1, description: "Step 1".to_string(), done: true },
                SpecTask { id: 2, description: "Step 2".to_string(), done: false },
            ],
            body: String::new(),
            source: PathBuf::from("feat.md"),
        };
        let ctx = spec.context_string();
        assert!(ctx.contains("Step 2"));
        assert!(!ctx.contains("Step 1")); // completed task not shown
    }

    #[test]
    fn pending_count() {
        let spec = Spec {
            name: "x".to_string(),
            status: SpecStatus::InProgress,
            requirements: String::new(),
            tasks: vec![
                SpecTask { id: 1, description: "a".to_string(), done: true },
                SpecTask { id: 2, description: "b".to_string(), done: false },
                SpecTask { id: 3, description: "c".to_string(), done: false },
            ],
            body: String::new(),
            source: PathBuf::from("x.md"),
        };
        assert_eq!(spec.completed(), 1);
        assert_eq!(spec.pending(), 2);
    }
}
