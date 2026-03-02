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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn combined_rules_both_empty() {
        let dir = std::env::temp_dir().join("vibeui_test_empty_rules");
        let _ = std::fs::create_dir_all(&dir);
        // No .vibeui.md file → empty workspace rules
        let out = combined_rules(&dir);
        // May or may not have global rules depending on environment,
        // but the workspace section should be absent
        assert!(!out.contains("## Project AI Rules"));
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn combined_rules_workspace_only() {
        let dir = std::env::temp_dir().join("vibeui_test_ws_rules");
        let _ = std::fs::create_dir_all(&dir);
        std::fs::write(dir.join(".vibeui.md"), "Always use Rust").unwrap();
        let out = combined_rules(&dir);
        assert!(out.contains("## Project AI Rules"));
        assert!(out.contains("Always use Rust"));
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn save_and_load_workspace_rules_roundtrip() {
        let dir = std::env::temp_dir().join("vibeui_test_roundtrip");
        let _ = std::fs::create_dir_all(&dir);
        save_workspace_rules(&dir, "test content").unwrap();
        let loaded = load_workspace_rules(&dir);
        assert_eq!(loaded, "test content");
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn load_workspace_rules_missing_file() {
        let dir = std::env::temp_dir().join("vibeui_test_no_rules_file");
        let _ = std::fs::create_dir_all(&dir);
        // Ensure file doesn't exist
        let _ = std::fs::remove_file(dir.join(".vibeui.md"));
        let result = load_workspace_rules(&dir);
        assert!(result.is_empty());
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn combined_rules_has_section_headers() {
        let dir = std::env::temp_dir().join("vibeui_test_headers");
        let _ = std::fs::create_dir_all(&dir);
        std::fs::write(dir.join(".vibeui.md"), "project rule").unwrap();
        let out = combined_rules(&dir);
        assert!(out.contains("## Project AI Rules\n"));
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn global_rules_path_is_under_home() {
        let path = global_rules_path();
        assert!(path.to_string_lossy().contains(".vibeui"));
        assert!(path.to_string_lossy().ends_with("rules.md"));
    }
}
