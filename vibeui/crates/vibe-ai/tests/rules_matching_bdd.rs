//! BDD tests for the rules directory loading and path-aware matching system.
//!
//! Run with: `cargo test --test rules_matching_bdd`

use cucumber::{given, then, when, World};
use std::path::PathBuf;
use tempfile::TempDir;
use vibe_ai::rules::{Rule, RulesLoader};

// ── World ─────────────────────────────────────────────────────────────────────

#[derive(Debug, World)]
#[world(init = Self::new)]
pub struct RulesWorld {
    workspace: Option<TempDir>,
    loaded_rules: Vec<Rule>,
}

impl RulesWorld {
    fn new() -> Self {
        Self { workspace: None, loaded_rules: vec![] }
    }

    fn rules_dir(&self) -> PathBuf {
        self.workspace
            .as_ref()
            .expect("workspace not set")
            .path()
            .join(".vibecli")
            .join("rules")
    }

    fn write_rule(&self, filename: &str, content: &str) {
        let dir = self.rules_dir();
        std::fs::create_dir_all(&dir).unwrap();
        std::fs::write(dir.join(filename), content).unwrap();
    }

    fn find_rule(&self, name: &str) -> Option<&Rule> {
        self.loaded_rules.iter().find(|r| r.name == name)
    }
}

// ── Background ────────────────────────────────────────────────────────────────

#[given("a workspace directory exists")]
fn setup_workspace(world: &mut RulesWorld) {
    world.workspace = Some(TempDir::new().expect("tempdir"));
}

// ── Given steps ───────────────────────────────────────────────────────────────

#[given(regex = r#"a rule file "([^"]+)" with content "([^"]+)""#)]
fn rule_no_pattern(world: &mut RulesWorld, filename: String, content: String) {
    world.write_rule(&filename, &content);
}

#[given(regex = r#"a rule file "([^"]+)" with path_pattern "([^"]+)" and content "([^"]+)""#)]
fn rule_with_pattern(world: &mut RulesWorld, filename: String, pattern: String, content: String) {
    let fm = format!("---\nname: {}\npath_pattern: \"{}\"\n---\n\n{}",
        filename.trim_end_matches(".md"), pattern, content);
    world.write_rule(&filename, &fm);
}

#[given(regex = r#"a rule file "([^"]+)" with frontmatter name "([^"]+)" and content "([^"]+)""#)]
fn rule_with_fm_name(world: &mut RulesWorld, filename: String, name: String, content: String) {
    let fm = format!("---\nname: {name}\n---\n\n{content}");
    world.write_rule(&filename, &fm);
}

// ── When steps ────────────────────────────────────────────────────────────────

#[when("I load rules from the workspace")]
fn load_rules(world: &mut RulesWorld) {
    let ws = world.workspace.as_ref().expect("workspace not set").path().to_path_buf();
    world.loaded_rules = RulesLoader::load_for_workspace(&ws);
}

// ── Then steps ────────────────────────────────────────────────────────────────

#[then(regex = r"(\d+) rules? (?:are|is) loaded")]
fn rule_count(world: &mut RulesWorld, expected: usize) {
    assert_eq!(
        world.loaded_rules.len(), expected,
        "expected {expected} rule(s), got {}: {:?}",
        world.loaded_rules.len(),
        world.loaded_rules.iter().map(|r| &r.name).collect::<Vec<_>>()
    );
}

#[then(regex = r#"the rule "([^"]+)" matches an empty file list"#)]
fn matches_empty(world: &mut RulesWorld, name: String) {
    let rule = world.find_rule(&name)
        .unwrap_or_else(|| panic!("rule '{name}' not found"));
    assert!(rule.matches_open_files(&[]), "rule '{name}' should match empty file list");
}

#[then(regex = r#"the rule "([^"]+)" matches the file "([^"]+)""#)]
fn matches_file(world: &mut RulesWorld, name: String, file: String) {
    let rule = world.find_rule(&name)
        .unwrap_or_else(|| panic!("rule '{name}' not found"));
    assert!(
        rule.matches_open_files(&[file.clone()]),
        "rule '{name}' should match '{file}'"
    );
}

#[then(regex = r#"the rule "([^"]+)" does not match the file "([^"]+)""#)]
fn not_matches_file(world: &mut RulesWorld, name: String, file: String) {
    let rule = world.find_rule(&name)
        .unwrap_or_else(|| panic!("rule '{name}' not found"));
    assert!(
        !rule.matches_open_files(&[file.clone()]),
        "rule '{name}' should NOT match '{file}'"
    );
}

#[then(regex = r#"the rule "([^"]+)" does not match an empty file list"#)]
fn not_matches_empty(world: &mut RulesWorld, name: String) {
    let rule = world.find_rule(&name)
        .unwrap_or_else(|| panic!("rule '{name}' not found"));
    assert!(
        !rule.matches_open_files(&[]),
        "rule '{name}' should NOT match empty file list"
    );
}

// ── Entry point ───────────────────────────────────────────────────────────────

#[tokio::main]
async fn main() {
    RulesWorld::run("tests/features/rules_matching.feature").await;
}
