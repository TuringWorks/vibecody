/*!
 * BDD tests for paste_guard using Cucumber.
 * Run with: cargo test --test paste_guard_bdd
 */
use cucumber::{given, then, when, World};
use vibecli_cli::paste_guard::{
    extract_paste_content, PasteGuard, PasteGuardConfig, ProcessResult, BRACKETED_PASTE_END,
    BRACKETED_PASTE_START,
};

// ── World ─────────────────────────────────────────────────────────────────────

#[derive(Debug, World)]
pub struct PgWorld {
    guard: PasteGuard,
    /// Content for the *current* paste being built.
    current_content: String,
    /// Most recent ProcessResult.
    result: Option<ProcessResult>,
    /// Raw string used by the extract_paste_content scenario.
    raw_input: String,
    /// Result of calling extract_paste_content.
    extracted: Option<String>,
    /// Running count of pastes processed so far (for id tracking).
    paste_count: u32,
}

impl Default for PgWorld {
    fn default() -> Self {
        Self {
            guard: PasteGuard::with_defaults(),
            current_content: String::new(),
            result: None,
            raw_input: String::new(),
            extracted: None,
            paste_count: 0,
        }
    }
}

// ── Background ────────────────────────────────────────────────────────────────

#[given(expr = "a PasteGuard with line threshold {int} and max stored pastes {int}")]
fn setup_guard(world: &mut PgWorld, threshold: usize, max_pastes: usize) {
    let config = PasteGuardConfig {
        line_threshold: threshold,
        max_stored_pastes: max_pastes,
        show_preview_lines: 2,
        auto_expand_under_threshold: true,
    };
    world.guard = PasteGuard::new(config);
    world.paste_count = 0;
}

// ── Givens ────────────────────────────────────────────────────────────────────

#[given(expr = "a paste containing {int} lines")]
fn set_paste_n_lines(world: &mut PgWorld, n: usize) {
    world.current_content = (0..n)
        .map(|i| format!("line {}", i + 1))
        .collect::<Vec<_>>()
        .join("\n");
}

#[given(expr = "a paste containing {int} lines labeled {string}")]
fn set_paste_labeled(world: &mut PgWorld, n: usize, label: String) {
    world.current_content = (0..n)
        .map(|i| format!("{} line {}", label, i + 1))
        .collect::<Vec<_>>()
        .join("\n");
}

#[given(expr = "a raw string with bracketed paste sequences wrapping {string}")]
fn set_raw_bracketed(world: &mut PgWorld, inner: String) {
    world.raw_input = format!("{}{}{}", BRACKETED_PASTE_START, inner, BRACKETED_PASTE_END);
}

// ── Whens ─────────────────────────────────────────────────────────────────────

#[when("I process the bracketed input")]
fn process_input(world: &mut PgWorld) {
    let bracketed = format!(
        "{}{}{}",
        BRACKETED_PASTE_START, world.current_content, BRACKETED_PASTE_END
    );
    let result = world.guard.process(&bracketed);
    world.paste_count += 1;
    world.result = Some(result);
}

#[when("I expand the marker in the processed output")]
fn expand_marker(world: &mut PgWorld) {
    let result = world.result.as_ref().expect("no result yet");
    // Find the marker line in the processed output.
    let marker = result
        .processed_input
        .lines()
        .find(|l| l.starts_with("[paste #"))
        .expect("no marker found in processed output")
        .to_string();
    // Store the expansion back onto the current_content field for assertion.
    let expanded = world
        .guard
        .expand_marker(&marker)
        .expect("expand_marker returned None")
        .to_string();
    world.current_content = expanded;
}

#[when("I call extract_paste_content on it")]
fn call_extract(world: &mut PgWorld) {
    world.extracted = extract_paste_content(&world.raw_input);
}

// ── Thens ─────────────────────────────────────────────────────────────────────

#[then(expr = "the result was_paste flag is {word}")]
fn check_was_paste(world: &mut PgWorld, expected: String) {
    let result = world.result.as_ref().expect("no result yet");
    let flag = expected.trim().eq_ignore_ascii_case("true");
    assert_eq!(result.was_paste, flag, "was_paste mismatch");
}

#[then(expr = "the result was_collapsed flag is {word}")]
fn check_was_collapsed(world: &mut PgWorld, expected: String) {
    let result = world.result.as_ref().expect("no result yet");
    let flag = expected.trim().eq_ignore_ascii_case("true");
    assert_eq!(result.was_collapsed, flag, "was_collapsed mismatch");
}

#[then("the processed output contains the original lines")]
fn check_contains_original(world: &mut PgWorld) {
    let result = world.result.as_ref().expect("no result yet");
    // For a small paste the original lines should appear verbatim.
    for line in world.current_content.lines() {
        assert!(
            result.processed_input.contains(line),
            "output missing line: {:?}",
            line
        );
    }
}

#[then(expr = "the processed output contains a marker matching {string}")]
fn check_marker_present(world: &mut PgWorld, expected_marker: String) {
    let result = world.result.as_ref().expect("no result yet");
    assert!(
        result.processed_input.contains(&expected_marker),
        "expected marker {:?} not found in:\n{}",
        expected_marker,
        result.processed_input
    );
}

#[then(expr = "the processed output does not contain {string}")]
fn check_not_contains(world: &mut PgWorld, text: String) {
    let result = world.result.as_ref().expect("no result yet");
    // The collapsed output may include preview lines (first 2); ensure lines
    // beyond the preview window do not appear literally in the output.
    // We check that the specific text is absent (line 10 is past preview 2).
    assert!(
        !result.processed_input.contains(&text),
        "output should not contain {:?} but found it in:\n{}",
        text,
        result.processed_input
    );
}

#[then("the expanded content matches the original paste")]
fn check_expanded_matches(world: &mut PgWorld) {
    // world.current_content was replaced by the expanded content in the When step.
    // The original paste was build from 12 lines in the Given step.
    // Verify it has 12 lines.
    let lines: Vec<&str> = world.current_content.lines().collect();
    assert_eq!(
        lines.len(),
        12,
        "expanded content should have 12 lines, got {}",
        lines.len()
    );
    assert_eq!(lines[0], "line 1");
    assert_eq!(lines[11], "line 12");
}

#[then(expr = "the store contains {int} pastes")]
fn check_store_count(world: &mut PgWorld, expected: usize) {
    assert_eq!(
        world.guard.store().count(),
        expected,
        "store count mismatch"
    );
}

#[then(expr = "paste id {int} is no longer in the store")]
fn check_id_absent(world: &mut PgWorld, id: u32) {
    assert!(
        world.guard.store().get(id).is_none(),
        "paste id {} should have been evicted",
        id
    );
}

#[then(expr = "paste id {int} is in the store")]
fn check_id_present(world: &mut PgWorld, id: u32) {
    assert!(
        world.guard.store().get(id).is_some(),
        "paste id {} should be in the store",
        id
    );
}

#[then(expr = "the extracted content is {string}")]
fn check_extracted(world: &mut PgWorld, expected: String) {
    let extracted = world
        .extracted
        .as_ref()
        .expect("extract_paste_content returned None");
    assert_eq!(extracted, &expected, "extracted content mismatch");
}

// ── Runner ────────────────────────────────────────────────────────────────────

fn main() {
    futures::executor::block_on(PgWorld::run("tests/features/paste_guard.feature"));
}
