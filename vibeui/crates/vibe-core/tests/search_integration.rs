//! Integration tests for vibe-core file search across a realistic directory tree.

use tempfile::TempDir;
use vibe_core::search::search_files;

fn setup_project() -> TempDir {
    let dir = TempDir::new().unwrap();
    let root = dir.path();

    std::fs::create_dir_all(root.join("src")).unwrap();
    std::fs::create_dir_all(root.join("tests")).unwrap();
    std::fs::create_dir_all(root.join("docs")).unwrap();

    std::fs::write(root.join("src/main.rs"),
        "fn main() {\n    println!(\"Hello, world!\");\n    let x = unwrap_value();\n}\n").unwrap();
    std::fs::write(root.join("src/lib.rs"),
        "pub fn add(a: i32, b: i32) -> i32 { a + b }\npub fn unwrap_value() -> i32 { 42 }\n").unwrap();
    std::fs::write(root.join("tests/integration.rs"),
        "use super::*;\n#[test]\nfn test_add() { assert_eq!(add(1,2), 3); }\n").unwrap();
    std::fs::write(root.join("docs/README.md"),
        "# Project\nThis is a sample README.\nUse `cargo test` to run tests.\n").unwrap();
    dir
}

// ── Basic search ──────────────────────────────────────────────────────────────

#[test]
fn search_finds_matches_across_multiple_files() {
    let dir = setup_project();
    let results = search_files(&dir.path().to_path_buf(), "unwrap_value", false).unwrap();
    assert!(results.len() >= 2,
        "unwrap_value appears in main.rs and lib.rs; got {} results", results.len());
    assert!(results.iter().any(|r| r.path.contains("main.rs")));
    assert!(results.iter().any(|r| r.path.contains("lib.rs")));
}

#[test]
fn search_returns_empty_for_no_match() {
    let dir = setup_project();
    let results = search_files(&dir.path().to_path_buf(), "xyzzy_nonexistent_token", false).unwrap();
    assert!(results.is_empty());
}

// ── Case sensitivity ──────────────────────────────────────────────────────────

#[test]
fn search_case_insensitive_finds_mixed_case() {
    let dir = setup_project();
    let results = search_files(&dir.path().to_path_buf(), "readme", false).unwrap();
    assert!(results.iter().any(|r| r.path.contains("README")),
        "case-insensitive search should find README.md");
}

#[test]
fn search_case_sensitive_misses_wrong_case() {
    let dir = setup_project();
    let results = search_files(&dir.path().to_path_buf(), "readme", true).unwrap();
    assert!(results.iter().all(|r| !r.path.contains("README.md")),
        "case-sensitive 'readme' should not match 'README.md'");
}

// ── Regex patterns ────────────────────────────────────────────────────────────

#[test]
fn search_regex_pattern_matches_function_signatures() {
    let dir = setup_project();
    let results = search_files(&dir.path().to_path_buf(), r"pub fn \w+", false).unwrap();
    assert!(!results.is_empty(), "should match pub fn signatures");
    assert!(results.iter().any(|r| r.path.contains("lib.rs")));
}

#[test]
fn search_regex_matches_test_attribute() {
    let dir = setup_project();
    let results = search_files(&dir.path().to_path_buf(), r"#\[test\]", false).unwrap();
    assert!(!results.is_empty(), "#[test] should be found");
    assert!(results.iter().any(|r| r.path.contains("integration.rs")));
}

// ── Result fields ─────────────────────────────────────────────────────────────

#[test]
fn search_results_include_line_number_and_content() {
    let dir = setup_project();
    let results = search_files(&dir.path().to_path_buf(), "fn main", false).unwrap();
    let main_result = results.iter()
        .find(|r| r.path.contains("main.rs"))
        .expect("should find match in main.rs");

    assert!(main_result.line_number > 0, "line number should be > 0");
    assert!(main_result.line_content.contains("fn main"),
        "line_content should contain the matched text");
}

// ── Empty and edge cases ──────────────────────────────────────────────────────

#[test]
fn search_in_empty_directory_returns_empty() {
    let dir = TempDir::new().unwrap();
    let results = search_files(&dir.path().to_path_buf(), "anything", false).unwrap();
    assert!(results.is_empty());
}

#[test]
fn search_single_file_project() {
    let dir = TempDir::new().unwrap();
    std::fs::write(dir.path().join("only.txt"), "unique_token_here").unwrap();
    let results = search_files(&dir.path().to_path_buf(), "unique_token_here", true).unwrap();
    assert_eq!(results.len(), 1);
    assert!(results[0].line_content.contains("unique_token_here"));
}
