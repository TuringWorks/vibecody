/*!
 * BDD tests for the tui_ime module — CURSOR_MARKER, CJK width, and IME state.
 * Run with: cargo test --test tui_ime_bdd
 */
use cucumber::{given, then, when, World};
use vibecli_cli::tui_ime::{
    find_cursor_marker, insert_cursor_marker, strip_cursor_marker, truncate_to_width,
    visible_width, ImeHandler, ImeState, CURSOR_MARKER,
};

// ── World ─────────────────────────────────────────────────────────────────────

#[derive(Debug, Default, World)]
pub struct ImeWorld {
    /// Input string for cursor-marker / width scenarios.
    input: String,
    /// String produced by the step under test.
    output: String,
    /// Numeric result (column offset, width, …).
    number: usize,
    /// Maximum columns for truncation scenarios.
    max_cols: usize,
    /// IME handler for state-machine scenarios.
    handler: ImeHandler,
}

// ── Given ─────────────────────────────────────────────────────────────────────

#[given(expr = "an ASCII line {string}")]
fn given_ascii_line(world: &mut ImeWorld, line: String) {
    world.input = line;
}

#[given("a rendered string with cursor markers at columns 0 and 6")]
fn given_string_with_markers(world: &mut ImeWorld) {
    // "foo" at col 0, "bar" after the second marker.
    world.input = format!("{}foo{}bar", CURSOR_MARKER, CURSOR_MARKER);
}

#[given(expr = "the string {string}")]
fn given_ansi_string(world: &mut ImeWorld, s: String) {
    // Cucumber delivers the literal text; we must interpret \x1b ourselves.
    world.input = s
        .replace("\\x1b", "\x1b")
        .replace("\\x5c", "\\")
        .replace("\\n", "\n");
}

#[given(expr = "the string {string} with max columns {int}")]
fn given_string_with_max_cols(world: &mut ImeWorld, s: String, max: usize) {
    world.input = s;
    world.max_cols = max;
}

#[given("a fresh ImeHandler")]
fn given_fresh_handler(world: &mut ImeWorld) {
    world.handler = ImeHandler::new();
}

// ── When ──────────────────────────────────────────────────────────────────────

#[when(expr = "I insert the cursor marker at column {int}")]
fn when_insert_marker(world: &mut ImeWorld, col: usize) {
    world.output = insert_cursor_marker(&world.input, col);
}

#[when("I strip all cursor markers")]
fn when_strip_markers(world: &mut ImeWorld) {
    world.output = strip_cursor_marker(&world.input);
}

#[when("I compute the visible width")]
fn when_compute_width(world: &mut ImeWorld) {
    world.number = visible_width(&world.input);
}

#[when("I truncate to max columns")]
fn when_truncate(world: &mut ImeWorld) {
    world.output = truncate_to_width(&world.input, world.max_cols);
}

#[when("composition starts")]
fn when_composition_starts(world: &mut ImeWorld) {
    world.handler.on_composition_start();
}

#[when(expr = "composition updates to {string}")]
fn when_composition_updates(world: &mut ImeWorld, text: String) {
    world.handler.on_composition_update(&text);
}

#[when(expr = "composition ends with {string}")]
fn when_composition_ends(world: &mut ImeWorld, final_text: String) {
    world.handler.on_composition_end(&final_text);
}

#[when("I reset the IME handler")]
fn when_reset_handler(world: &mut ImeWorld) {
    world.handler.reset();
}

// ── Then ──────────────────────────────────────────────────────────────────────

#[then(expr = "the visible text is still {string}")]
fn then_visible_text(world: &mut ImeWorld, expected: String) {
    let stripped = strip_cursor_marker(&world.output);
    assert_eq!(
        stripped, expected,
        "stripped visible text mismatch: got {:?}",
        stripped
    );
}

#[then(expr = "find_cursor_marker returns column {int}")]
fn then_find_marker_col(world: &mut ImeWorld, expected_col: usize) {
    let col = find_cursor_marker(&world.output)
        .expect("CURSOR_MARKER should be present in the output");
    assert_eq!(
        col, expected_col,
        "cursor column mismatch: expected {}, got {}",
        expected_col, col
    );
}

#[then(expr = "the result equals {string}")]
fn then_result_equals(world: &mut ImeWorld, expected: String) {
    assert_eq!(
        world.output, expected,
        "output mismatch: expected {:?}, got {:?}",
        expected, world.output
    );
}

#[then(expr = "the visible width is {int}")]
fn then_visible_width(world: &mut ImeWorld, expected: usize) {
    assert_eq!(
        world.number, expected,
        "visible width mismatch: expected {}, got {}",
        expected, world.number
    );
}

#[then(expr = "the truncated visible width is at most {int}")]
fn then_truncated_width_le(world: &mut ImeWorld, max: usize) {
    let w = visible_width(&world.output);
    assert!(
        w <= max,
        "truncated width {} exceeds max {}",
        w,
        max
    );
}

#[then("no wide character is split across the boundary")]
fn then_no_split_wide_char(world: &mut ImeWorld) {
    // All wide chars in the output must have their full 2-column display width
    // intact.  We verify by checking that visible_width == sum of char widths.
    use vibecli_cli::tui_ime::EawCategory;
    let stripped = strip_cursor_marker(&world.output);
    let char_sum: usize = stripped
        .chars()
        .map(|c| EawCategory::for_char(c).display_width())
        .sum();
    let vw = visible_width(&world.output);
    assert_eq!(
        vw, char_sum,
        "visible_width {} != char-sum {} — a wide char may have been split",
        vw, char_sum
    );
}

#[then("the IME state is Composing")]
fn then_state_composing(world: &mut ImeWorld) {
    assert_eq!(
        world.handler.state(),
        &ImeState::Composing,
        "expected Composing state"
    );
}

#[then(expr = "the preedit text is {string}")]
fn then_preedit_text(world: &mut ImeWorld, expected: String) {
    assert_eq!(
        world.handler.composition(),
        expected,
        "preedit mismatch"
    );
}

#[then("the IME state is Committed")]
fn then_state_committed(world: &mut ImeWorld) {
    assert_eq!(
        world.handler.state(),
        &ImeState::Committed,
        "expected Committed state"
    );
}

#[then(expr = "the committed text is {string}")]
fn then_committed_text(world: &mut ImeWorld, expected: String) {
    assert_eq!(
        world.handler.committed(),
        expected,
        "committed text mismatch"
    );
}

#[then("the IME state is Idle")]
fn then_state_idle(world: &mut ImeWorld) {
    assert_eq!(
        world.handler.state(),
        &ImeState::Idle,
        "expected Idle state"
    );
}

// ── Main ──────────────────────────────────────────────────────────────────────

fn main() {
    futures::executor::block_on(ImeWorld::run("tests/features/tui_ime.feature"));
}
