//! Integration tests for the vibe-ai rules directory system.
//!
//! Tests the full workflow of creating rule files, loading them, matching them
//! against open files, and combining them into an AI system prompt.

use std::sync::Mutex;
use tempfile::TempDir;
use vibe_ai::rules::RulesLoader;

// Serialise tests that mutate HOME to avoid parallel-test interference.
static HOME_LOCK: Mutex<()> = Mutex::new(());

// ── helpers ───────────────────────────────────────────────────────────────────

fn workspace() -> TempDir {
    TempDir::new().unwrap()
}

/// Write a rule file into `<ws>/.vibecli/rules/<filename>`.
fn write_rule(ws: &TempDir, filename: &str, content: &str) {
    let dir = ws.path().join(".vibecli").join("rules");
    std::fs::create_dir_all(&dir).unwrap();
    std::fs::write(dir.join(filename), content).unwrap();
}

/// Load rules from the workspace-local directory only (no global HOME lookup).
fn load_local(ws: &TempDir) -> Vec<vibe_ai::rules::Rule> {
    RulesLoader::load(&ws.path().join(".vibecli").join("rules"))
}

// ── loading ───────────────────────────────────────────────────────────────────

#[test]
fn empty_rules_dir_loads_no_rules() {
    let ws = workspace();
    let rules = load_local(&ws);
    assert!(rules.is_empty());
}

#[test]
fn loads_rule_without_frontmatter() {
    let ws = workspace();
    write_rule(&ws, "plain.md", "Always write safe code.\n");
    let rules = load_local(&ws);
    assert_eq!(rules.len(), 1);
    assert_eq!(rules[0].name, "plain");
    assert!(rules[0].path_pattern.is_none());
    assert!(rules[0].content.contains("safe code"));
}

#[test]
fn loads_rule_with_full_frontmatter() {
    let ws = workspace();
    write_rule(&ws, "rust.md",
        "---\nname: rust-safety\npath_pattern: \"**/*.rs\"\n---\n\nAvoid unwrap().\n");
    let rules = load_local(&ws);
    assert_eq!(rules.len(), 1);
    assert_eq!(rules[0].name, "rust-safety");
    assert_eq!(rules[0].path_pattern.as_deref(), Some("**/*.rs"));
    assert!(rules[0].content.contains("unwrap"));
}

#[test]
fn loads_multiple_rules_from_directory() {
    let ws = workspace();
    write_rule(&ws, "always.md", "Be safe.\n");
    write_rule(&ws, "rust.md",
        "---\nname: rust\npath_pattern: \"**/*.rs\"\n---\n\nNo unwrap.\n");
    write_rule(&ws, "ts.md",
        "---\nname: ts\npath_pattern: \"**/*.ts\"\n---\n\nUse strict.\n");

    let rules = load_local(&ws);
    assert_eq!(rules.len(), 3);
    let names: Vec<_> = rules.iter().map(|r| r.name.as_str()).collect();
    assert!(names.contains(&"always"));
    assert!(names.contains(&"rust"));
    assert!(names.contains(&"ts"));
}

// ── matching ──────────────────────────────────────────────────────────────────

#[test]
fn rule_without_pattern_matches_any_file() {
    let ws = workspace();
    write_rule(&ws, "always.md", "Be safe.");
    let rules = load_local(&ws);
    let rule = rules.iter().find(|r| r.name == "always").unwrap();

    assert!(rule.matches_open_files(&[]));
    assert!(rule.matches_open_files(&["anything.rs".into()]));
    assert!(rule.matches_open_files(&["README.md".into()]));
}

#[test]
fn rule_with_rs_pattern_only_matches_rust_files() {
    let ws = workspace();
    write_rule(&ws, "rust.md",
        "---\nname: rust\npath_pattern: \"**/*.rs\"\n---\n\nNo unwrap.\n");
    let rules = load_local(&ws);
    let rule = rules.iter().find(|r| r.name == "rust").unwrap();

    assert!(rule.matches_open_files(&["src/main.rs".into()]));
    assert!(rule.matches_open_files(&["deep/nested/util.rs".into()]));
    assert!(!rule.matches_open_files(&["src/main.ts".into()]));
    assert!(!rule.matches_open_files(&[]));
}

#[test]
fn applicable_rules_filter_by_open_files() {
    let ws = workspace();
    write_rule(&ws, "always.md", "Be safe.");
    write_rule(&ws, "rust.md",
        "---\nname: rust\npath_pattern: \"**/*.rs\"\n---\n\nNo unwrap.\n");
    write_rule(&ws, "ts.md",
        "---\nname: ts\npath_pattern: \"**/*.ts\"\n---\n\nUse strict.\n");

    let rules = load_local(&ws);
    let open = vec!["src/main.rs".to_string()];
    let applicable: Vec<_> = rules.iter().filter(|r| r.matches_open_files(&open)).collect();
    let names: Vec<_> = applicable.iter().map(|r| r.name.as_str()).collect();

    assert!(names.contains(&"always"), "always-apply rule should be included");
    assert!(names.contains(&"rust"),   "rust rule should match .rs file");
    assert!(!names.contains(&"ts"),    "ts rule should not match .rs file");
}

// ── global rules deduplication (serialised — touches HOME) ───────────────────

#[test]
fn global_rule_not_loaded_when_workspace_has_same_name() {
    let _guard = HOME_LOCK.lock().unwrap();

    let global_home = TempDir::new().unwrap();
    let ws = workspace();

    write_rule(&ws, "shared.md", "---\nname: shared\n---\n\nWorkspace version.\n");

    let global_rules_dir = global_home.path().join(".vibecli").join("rules");
    std::fs::create_dir_all(&global_rules_dir).unwrap();
    std::fs::write(global_rules_dir.join("shared.md"),
        "---\nname: shared\n---\n\nGlobal version.\n").unwrap();

    let original = std::env::var("HOME").unwrap_or_default();
    std::env::set_var("HOME", global_home.path());
    let rules = RulesLoader::load_for_workspace(ws.path());
    std::env::set_var("HOME", &original);

    let shared: Vec<_> = rules.iter().filter(|r| r.name == "shared").collect();
    assert_eq!(shared.len(), 1);
    assert!(shared[0].content.contains("Workspace version"),
        "workspace rule should take precedence");
}

#[test]
fn global_rule_loaded_when_no_workspace_conflict() {
    let _guard = HOME_LOCK.lock().unwrap();

    let global_home = TempDir::new().unwrap();
    let ws = workspace();

    let global_rules_dir = global_home.path().join(".vibecli").join("rules");
    std::fs::create_dir_all(&global_rules_dir).unwrap();
    std::fs::write(global_rules_dir.join("global-only.md"),
        "---\nname: global-only\n---\n\nGlobal guidance.\n").unwrap();

    let original = std::env::var("HOME").unwrap_or_default();
    std::env::set_var("HOME", global_home.path());
    let rules = RulesLoader::load_for_workspace(ws.path());
    std::env::set_var("HOME", &original);

    assert!(rules.iter().any(|r| r.name == "global-only"),
        "global rule should be loaded when workspace has no conflict");
}

// ── rule content integrity ────────────────────────────────────────────────────

#[test]
fn rule_body_excludes_frontmatter_block() {
    let ws = workspace();
    write_rule(&ws, "rule.md",
        "---\nname: clean\npath_pattern: \"*\"\n---\n\nThis is the body.\nNo YAML here.\n");
    let rules = load_local(&ws);
    let rule = rules.iter().find(|r| r.name == "clean").unwrap();

    assert!(!rule.content.contains("---"), "frontmatter delimiters should not appear in body");
    assert!(!rule.content.contains("path_pattern"), "frontmatter keys should not appear in body");
    assert!(rule.content.contains("This is the body"), "body content should be present");
}
