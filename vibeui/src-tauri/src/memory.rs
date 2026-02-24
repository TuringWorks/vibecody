//! Workspace and global AI rules for VibeUI.
//!
//! - Project rules: `<workspace>/.vibeui.md` (committed alongside code)
//! - Global rules:  `~/.vibeui/rules.md`   (personal defaults)

use std::path::{Path, PathBuf};

const WORKSPACE_RULES_FILE: &str = ".vibeui.md";
const GLOBAL_RULES_DIR: &str = ".vibeui";
const GLOBAL_RULES_FILE: &str = "rules.md";

fn home_dir() -> PathBuf {
    std::env::var("HOME")
        .or_else(|_| std::env::var("USERPROFILE"))
        .map(PathBuf::from)
        .unwrap_or_else(|_| PathBuf::from("."))
}

fn global_rules_path() -> PathBuf {
    home_dir().join(GLOBAL_RULES_DIR).join(GLOBAL_RULES_FILE)
}

/// Load project-level rules from `<workspace>/.vibeui.md`.
pub fn load_workspace_rules(workspace_root: &Path) -> String {
    std::fs::read_to_string(workspace_root.join(WORKSPACE_RULES_FILE)).unwrap_or_default()
}

/// Load global rules from `~/.vibeui/rules.md`.
pub fn load_global_rules() -> String {
    std::fs::read_to_string(global_rules_path()).unwrap_or_default()
}

/// Combined rules injected into every AI system prompt.
pub fn combined_rules(workspace_root: &Path) -> String {
    let global = load_global_rules();
    let workspace = load_workspace_rules(workspace_root);
    let mut out = String::new();
    if !global.is_empty() {
        out.push_str("## Global AI Rules\n");
        out.push_str(&global);
        out.push('\n');
    }
    if !workspace.is_empty() {
        out.push_str("## Project AI Rules\n");
        out.push_str(&workspace);
        out.push('\n');
    }
    out
}

/// Save project rules to `<workspace>/.vibeui.md`.
pub fn save_workspace_rules(workspace_root: &Path, content: &str) -> std::io::Result<()> {
    std::fs::write(workspace_root.join(WORKSPACE_RULES_FILE), content)
}

/// Save global rules to `~/.vibeui/rules.md` (creates directory if needed).
pub fn save_global_rules(content: &str) -> std::io::Result<()> {
    let path = global_rules_path();
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent)?;
    }
    std::fs::write(path, content)
}
