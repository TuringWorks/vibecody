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
//! version: "1.0.0"
//! requires.bins: [cargo, rustc]
//! requires.env: []
//! requires.os: [macos, linux]
//! install.cargo: clippy
//! ---
//!
//! When editing Rust code, always:
//! 1. Check for `unwrap()` calls that should be `?` or `expect()`
//! 2. Verify all `unsafe` blocks have a `// SAFETY:` comment
//! 3. After writing, run `cargo clippy -- -D warnings` via bash tool
//! ```

use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::sync::{Arc, RwLock};

// ── Skill ─────────────────────────────────────────────────────────────────────

/// Auto-install directive for a skill dependency.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SkillInstaller {
    /// Package manager: "brew", "npm", "cargo", "pip", "go".
    pub manager: String,
    /// Package name to install.
    pub package: String,
}

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
    /// Semver version string.
    pub version: Option<String>,
    /// Required binaries that must exist on PATH.
    #[serde(default)]
    pub requires_bins: Vec<String>,
    /// Required environment variables.
    #[serde(default)]
    pub requires_env: Vec<String>,
    /// OS filter: ["macos", "linux", "windows"]. Empty = all platforms.
    #[serde(default)]
    pub requires_os: Vec<String>,
    /// Auto-install commands when activated and deps missing.
    #[serde(default)]
    pub installers: Vec<SkillInstaller>,
    /// Per-skill config key-value pairs injected as env vars.
    #[serde(default)]
    pub config: HashMap<String, String>,
    /// Webhook trigger route (e.g., "/webhook/deploy").
    pub webhook_trigger: Option<String>,
}

impl Skill {
    /// Returns `true` if any trigger appears (case-insensitively) in `text`.
    pub fn matches(&self, text: &str) -> bool {
        let lower = text.to_lowercase();
        self.triggers
            .iter()
            .any(|t| lower.contains(&t.to_lowercase()))
    }

    /// Check if all system requirements (OS, binaries, env vars) are satisfied.
    pub fn requirements_met(&self) -> bool {
        // OS filter
        if !self.requires_os.is_empty() {
            let current_os = std::env::consts::OS;
            if !self
                .requires_os
                .iter()
                .any(|o| o.eq_ignore_ascii_case(current_os))
            {
                return false;
            }
        }
        // Binary requirements
        for bin in &self.requires_bins {
            if !binary_exists(bin) {
                return false;
            }
        }
        // Env var requirements
        for var in &self.requires_env {
            if std::env::var(var).is_err() {
                return false;
            }
        }
        true
    }

    /// Attempt to auto-install missing dependencies using the configured installers.
    /// Returns the list of successfully installed package names.
    pub fn auto_install(&self) -> Vec<String> {
        let mut installed = Vec::new();
        for installer in &self.installers {
            let cmd = match installer.manager.as_str() {
                "brew" => format!("brew install {}", installer.package),
                "npm" => format!("npm install -g {}", installer.package),
                "cargo" => format!("cargo install {}", installer.package),
                "pip" | "uv" => format!("pip install {}", installer.package),
                "go" => format!("go install {}", installer.package),
                _ => continue,
            };
            if let Ok(status) = vibe_core::shell::sh(&cmd)
                .stdout(std::process::Stdio::null())
                .stderr(std::process::Stdio::null())
                .status()
            {
                if status.success() {
                    installed.push(installer.package.clone());
                }
            }
        }
        installed
    }

    /// Inject per-skill config values into the environment.
    pub fn inject_config_env(&self) {
        for (k, v) in &self.config {
            std::env::set_var(k, v);
        }
    }
}

/// Check if a binary exists on PATH.
fn binary_exists(name: &str) -> bool {
    if let Ok(path_var) = std::env::var("PATH") {
        for dir in std::env::split_paths(&path_var) {
            let candidate = dir.join(name);
            if candidate.exists() {
                return true;
            }
        }
    }
    false
}

// ── SkillLoader ───────────────────────────────────────────────────────────────

/// Discovers and loads skill files from one or more directories.
pub struct SkillLoader {
    pub(crate) dirs: Vec<PathBuf>,
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
                            Err(e) => {
                                tracing::warn!("Failed to load skill {}: {}", path.display(), e)
                            }
                        }
                    }
                }
            }
        }
        skills
    }

    /// Return all skills whose triggers match `text`.
    pub fn matching(&self, text: &str) -> Vec<Skill> {
        self.load_all()
            .into_iter()
            .filter(|s| s.matches(text))
            .collect()
    }

    /// Return matching skills that also satisfy system requirements.
    pub fn matching_available(&self, text: &str) -> Vec<Skill> {
        self.load_all()
            .into_iter()
            .filter(|s| s.matches(text) && s.requirements_met())
            .collect()
    }
}

// ── SkillWatcher ──────────────────────────────────────────────────────────────

/// Watches skill directories for changes and caches loaded skills.
pub struct SkillWatcher {
    dirs: Vec<PathBuf>,
    cache: Arc<RwLock<Vec<Skill>>>,
}

impl SkillWatcher {
    /// Create a watcher from a SkillLoader's configuration.
    pub fn new(loader: &SkillLoader) -> Self {
        let initial = loader.load_all();
        Self {
            dirs: loader.dirs.clone(),
            cache: Arc::new(RwLock::new(initial)),
        }
    }

    /// Reload all skills from disk (called when a file change is detected).
    pub fn reload(&self) {
        let loader = SkillLoader::with_dirs(self.dirs.clone());
        let skills = loader.load_all();
        if let Ok(mut cache) = self.cache.write() {
            tracing::info!("[skills] Reloaded {} skills", skills.len());
            *cache = skills;
        }
    }

    /// Get the current cached skills snapshot.
    pub fn cached_skills(&self) -> Vec<Skill> {
        self.cache.read().map(|s| s.clone()).unwrap_or_default()
    }

    /// Get the directories being watched.
    pub fn watched_dirs(&self) -> &[PathBuf] {
        &self.dirs
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
    let mut fm_version: Option<String> = None;
    let mut fm_requires_bins: Vec<String> = Vec::new();
    let mut fm_requires_env: Vec<String> = Vec::new();
    let mut fm_requires_os: Vec<String> = Vec::new();
    let mut fm_installers: Vec<SkillInstaller> = Vec::new();
    let mut fm_config: HashMap<String, String> = HashMap::new();
    let mut fm_webhook_trigger: Option<String> = None;

    // The list key whose block-form items are currently being collected.
    //
    // YAML allows a list to be written inline (`triggers: [a, b]`) or as
    // indented `- item` lines, and the second form is the one most authors
    // reach for. Only the inline form was understood here, so a skill written
    // the ordinary way loaded with an empty trigger list and could never
    // activate — silently, because an empty list is also what "this skill
    // declares no triggers" looks like. Every skill in the shipped plugin
    // catalog was in that state.
    let mut open_list: Option<&str> = None;

    for line in &front_matter_lines {
        let line = line.trim();
        if line.is_empty() {
            continue;
        }

        if let Some(item) = line.strip_prefix("- ") {
            let item = item.trim().trim_matches('"').trim_matches('\'');
            if !item.is_empty() {
                match open_list {
                    Some("triggers") => fm_triggers.push(item.to_string()),
                    Some("tools_allowed") => fm_tools_allowed.push(item.to_string()),
                    Some("requires.bins") => fm_requires_bins.push(item.to_string()),
                    Some("requires.env") => fm_requires_env.push(item.to_string()),
                    Some("requires.os") => fm_requires_os.push(item.to_string()),
                    // An item under a key this parser does not model, or a
                    // stray dash. Skipped, like any unknown key.
                    _ => {}
                }
            }
            continue;
        }
        // Any non-item line ends the block.
        open_list = None;

        if let Some(rest) = line.strip_prefix("name:") {
            fm_name = rest.trim().trim_matches('"').to_string();
        } else if let Some(rest) = line.strip_prefix("description:") {
            fm_description = rest.trim().trim_matches('"').to_string();
        } else if let Some(rest) = line.strip_prefix("triggers:") {
            fm_triggers = parse_yaml_list(rest.trim());
            open_list = list_continues("triggers", rest);
        } else if let Some(rest) = line.strip_prefix("tools_allowed:") {
            fm_tools_allowed = parse_yaml_list(rest.trim());
            open_list = list_continues("tools_allowed", rest);
        } else if let Some(rest) = line.strip_prefix("version:") {
            fm_version = Some(rest.trim().trim_matches('"').to_string());
        } else if let Some(rest) = line.strip_prefix("requires.bins:") {
            fm_requires_bins = parse_yaml_list(rest.trim());
            open_list = list_continues("requires.bins", rest);
        } else if let Some(rest) = line.strip_prefix("requires.env:") {
            fm_requires_env = parse_yaml_list(rest.trim());
            open_list = list_continues("requires.env", rest);
        } else if let Some(rest) = line.strip_prefix("requires.os:") {
            fm_requires_os = parse_yaml_list(rest.trim());
            open_list = list_continues("requires.os", rest);
        } else if let Some(rest) = line.strip_prefix("webhook_trigger:") {
            fm_webhook_trigger = Some(rest.trim().trim_matches('"').to_string());
        } else if let Some(rest) = line.strip_prefix("install.brew:") {
            fm_installers.push(SkillInstaller {
                manager: "brew".into(),
                package: rest.trim().trim_matches('"').to_string(),
            });
        } else if let Some(rest) = line.strip_prefix("install.npm:") {
            fm_installers.push(SkillInstaller {
                manager: "npm".into(),
                package: rest.trim().trim_matches('"').to_string(),
            });
        } else if let Some(rest) = line.strip_prefix("install.cargo:") {
            fm_installers.push(SkillInstaller {
                manager: "cargo".into(),
                package: rest.trim().trim_matches('"').to_string(),
            });
        } else if let Some(rest) = line.strip_prefix("install.pip:") {
            fm_installers.push(SkillInstaller {
                manager: "pip".into(),
                package: rest.trim().trim_matches('"').to_string(),
            });
        } else if let Some(rest) = line.strip_prefix("install.go:") {
            fm_installers.push(SkillInstaller {
                manager: "go".into(),
                package: rest.trim().trim_matches('"').to_string(),
            });
        } else if let Some(rest) = line.strip_prefix("config.") {
            // config.KEY: VALUE
            if let Some(colon_pos) = rest.find(':') {
                let key = rest[..colon_pos].trim().to_string();
                let value = rest[colon_pos + 1..].trim().trim_matches('"').to_string();
                fm_config.insert(key, value);
            }
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
        version: fm_version,
        requires_bins: fm_requires_bins,
        requires_env: fm_requires_env,
        requires_os: fm_requires_os,
        installers: fm_installers,
        config: fm_config,
        webhook_trigger: fm_webhook_trigger,
    })
}

/// `Some(key)` when a list key's value is empty, meaning its items follow as
/// indented `- item` lines; `None` when the value was inline and complete.
fn list_continues(key: &'static str, rest: &str) -> Option<&'static str> {
    rest.trim().is_empty().then_some(key)
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
        let fm: Vec<String> = lines[1..close_idx + 1]
            .iter()
            .map(|s| s.to_string())
            .collect();
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
            version: None,
            requires_bins: vec![],
            requires_env: vec![],
            requires_os: vec![],
            installers: vec![],
            config: HashMap::new(),
            webhook_trigger: None,
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

    // Every skill in the shipped plugin catalog is written this way, and every
    // one of them loaded with an empty trigger list — so none could ever
    // activate, while still being counted as a live component by the panel.
    #[test]
    fn load_skill_file_reads_block_form_lists() {
        let dir = tempfile::tempdir().unwrap();
        let skill_path = dir.path().join("test-first.md");
        std::fs::write(
            &skill_path,
            "---\ncategory: testing\ntriggers:\n  - write a test\n  - failing test\nrequires.bins:\n  - cargo\n---\n\n# Test first\n\nWatch it fail.",
        )
        .unwrap();

        let skill = load_skill_file(&skill_path).unwrap();
        // No `name:` in front matter — the stem is authoritative.
        assert_eq!(skill.name, "test-first");
        assert_eq!(skill.triggers, vec!["write a test", "failing test"]);
        assert_eq!(skill.requires_bins, vec!["cargo"]);
        assert!(skill.matches("please write a test for the parser"));
        assert!(!skill.matches("rename a variable"));
    }

    #[test]
    fn load_skill_file_block_list_ends_at_the_next_key() {
        let dir = tempfile::tempdir().unwrap();
        let skill_path = dir.path().join("scoped.md");
        std::fs::write(
            &skill_path,
            "---\ntriggers:\n  - alpha\ndescription: not a trigger\ntools_allowed:\n  - read_file\n---\nbody",
        )
        .unwrap();

        let skill = load_skill_file(&skill_path).unwrap();
        assert_eq!(
            skill.triggers,
            vec!["alpha"],
            "a later key must close the list"
        );
        assert_eq!(skill.description, "not a trigger");
        assert_eq!(skill.tools_allowed, vec!["read_file"]);
    }

    #[test]
    fn load_skill_file_still_reads_inline_lists() {
        let dir = tempfile::tempdir().unwrap();
        let skill_path = dir.path().join("inline.md");
        std::fs::write(
            &skill_path,
            "---\nname: inline\ntriggers: [alpha, \"beta gamma\"]\n---\nbody",
        )
        .unwrap();

        let skill = load_skill_file(&skill_path).unwrap();
        assert_eq!(skill.triggers, vec!["alpha", "beta gamma"]);
    }

    #[test]
    fn skill_loader_loads_from_dir() {
        let dir = tempfile::tempdir().unwrap();
        let skills_dir = dir.path().join("skills");
        std::fs::create_dir(&skills_dir).unwrap();
        std::fs::write(
            skills_dir.join("rust-safety.md"),
            "---\nname: rust-safety\ntriggers: [unsafe, lifetime]\n---\nCheck for unsafe blocks.",
        )
        .unwrap();

        let loader = SkillLoader::with_dirs(vec![skills_dir]);
        let skills = loader.load_all();
        assert_eq!(skills.len(), 1);
        assert_eq!(skills[0].name, "rust-safety");

        let matching = loader.matching("Fix unsafe code");
        assert_eq!(matching.len(), 1);

        let non_matching = loader.matching("Add a new feature");
        assert_eq!(non_matching.len(), 0);
    }

    #[test]
    fn requirements_met_no_requirements() {
        let skill = make_skill(&["test"], "content");
        assert!(skill.requirements_met());
    }

    #[test]
    fn requirements_met_os_filter_current() {
        let mut skill = make_skill(&["test"], "content");
        skill.requires_os = vec![std::env::consts::OS.to_string()];
        assert!(skill.requirements_met());
    }

    #[test]
    fn requirements_met_os_filter_wrong() {
        let mut skill = make_skill(&["test"], "content");
        skill.requires_os = vec!["nonexistent_os".to_string()];
        assert!(!skill.requirements_met());
    }

    #[test]
    fn requirements_met_env_var_present() {
        std::env::set_var("VIBECLI_TEST_SKILL_REQ", "1");
        let mut skill = make_skill(&["test"], "content");
        skill.requires_env = vec!["VIBECLI_TEST_SKILL_REQ".to_string()];
        assert!(skill.requirements_met());
        std::env::remove_var("VIBECLI_TEST_SKILL_REQ");
    }

    #[test]
    fn requirements_met_env_var_missing() {
        let mut skill = make_skill(&["test"], "content");
        skill.requires_env = vec!["VIBECLI_DEFINITELY_NOT_SET_XYZ".to_string()];
        assert!(!skill.requirements_met());
    }

    #[test]
    fn requirements_met_bin_exists() {
        let mut skill = make_skill(&["test"], "content");
        skill.requires_bins = vec!["sh".to_string()]; // sh always exists
        assert!(skill.requirements_met());
    }

    #[test]
    fn requirements_met_bin_missing() {
        let mut skill = make_skill(&["test"], "content");
        skill.requires_bins = vec!["nonexistent_binary_xyz_12345".to_string()];
        assert!(!skill.requirements_met());
    }

    #[test]
    fn load_skill_with_extended_frontmatter() {
        let dir = tempfile::tempdir().unwrap();
        let skill_path = dir.path().join("extended.md");
        std::fs::write(
            &skill_path,
            "---\nname: extended\ntriggers: [deploy]\nversion: \"2.1.0\"\nrequires.bins: [docker, kubectl]\nrequires.os: [linux, macos]\ninstall.brew: kubectl\nwebhook_trigger: /webhook/deploy\nconfig.AWS_REGION: us-east-1\n---\nDeploy instructions.",
        ).unwrap();

        let skill = load_skill_file(&skill_path).unwrap();
        assert_eq!(skill.name, "extended");
        assert_eq!(skill.version.as_deref(), Some("2.1.0"));
        assert_eq!(skill.requires_bins, vec!["docker", "kubectl"]);
        assert_eq!(skill.requires_os, vec!["linux", "macos"]);
        assert_eq!(skill.installers.len(), 1);
        assert_eq!(skill.installers[0].manager, "brew");
        assert_eq!(skill.installers[0].package, "kubectl");
        assert_eq!(skill.webhook_trigger.as_deref(), Some("/webhook/deploy"));
        assert_eq!(
            skill.config.get("AWS_REGION").map(|s| s.as_str()),
            Some("us-east-1")
        );
    }

    #[test]
    fn skill_watcher_caches() {
        let dir = tempfile::tempdir().unwrap();
        let skills_dir = dir.path().join("skills");
        std::fs::create_dir(&skills_dir).unwrap();
        std::fs::write(
            skills_dir.join("test.md"),
            "---\nname: test\ntriggers: [test]\n---\nContent.",
        )
        .unwrap();

        let loader = SkillLoader::with_dirs(vec![skills_dir]);
        let watcher = SkillWatcher::new(&loader);
        let skills = watcher.cached_skills();
        assert_eq!(skills.len(), 1);
        assert_eq!(skills[0].name, "test");
    }

    #[test]
    fn matching_available_filters_requirements() {
        let dir = tempfile::tempdir().unwrap();
        let skills_dir = dir.path().join("skills");
        std::fs::create_dir(&skills_dir).unwrap();
        // Skill with met requirements
        std::fs::write(
            skills_dir.join("available.md"),
            "---\nname: available\ntriggers: [deploy]\nrequires.bins: [sh]\n---\nOk.",
        )
        .unwrap();
        // Skill with unmet requirements
        std::fs::write(
            skills_dir.join("unavailable.md"),
            "---\nname: unavailable\ntriggers: [deploy]\nrequires.bins: [nonexistent_xyz_999]\n---\nNo.",
        ).unwrap();

        let loader = SkillLoader::with_dirs(vec![skills_dir]);
        let available = loader.matching_available("deploy this");
        assert_eq!(available.len(), 1);
        assert_eq!(available[0].name, "available");
    }
}
