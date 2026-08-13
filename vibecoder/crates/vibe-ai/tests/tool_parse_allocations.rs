//! Allocation budget for the streaming tool-call parser.
//!
//! `parse_tool_calls` and `strip_thinking` run on **every streamed chunk** from
//! every model response, so their per-call cost is multiplied by the length of
//! every conversation in the product. This target installs a counting allocator
//! and asserts a budget, so "this path is cheap now" is a number CI checks
//! rather than a claim someone made once.
//!
//! The budgets below are deliberately loose — several times the measured value.
//! They are a ratchet against regressions of the *kind* that motivated them (a
//! regex recompiled per call), not a fight over single allocations. If a change
//! legitimately needs more, move the number and say why in the commit.
//!
//! Measured on 2026-08-13, before and after hoisting the regexes into
//! `LazyLock` and replacing `extract_tag`'s per-parameter regex with a scan:
//!
//! | call                            | before              | after         |
//! |---------------------------------|---------------------|---------------|
//! | `parse_tool_calls` (tool call)  | 91,504 / 43.7 MB    | 180 / 10 KB   |
//! | `parse_tool_calls` (prose)      | 100,503 / 41.6 MB   | 226 / 9 KB    |
//! | `strip_thinking`                | 46,559 / 26.9 MB    | 61 / 4.4 KB   |
//!
//! The prose case is the one worth remembering: 41 MB and 100k allocations to
//! determine that a fragment of plain text contained no tool call.

use vibe_alloc_count::{measure_steady_state, CountingAllocator};

#[global_allocator]
static ALLOC: CountingAllocator = CountingAllocator::new();

/// A chunk shaped like what a model actually streams: some prose, a thinking
/// block, and one element-style tool call.
const CHUNK: &str = r#"Let me look at that file.
<thinking>The user wants the config; read it first.</thinking>
<read_file path="src/main.rs" />
That should tell us what we need."#;

/// A chunk with no tool call at all — the overwhelmingly common case, since
/// most streamed fragments are plain prose.
const PROSE: &str = "That change looks right to me, and the tests should cover \
                     the regression you were worried about.";

#[test]
fn parsing_a_chunk_with_a_tool_call_stays_within_budget() {
    let stats = measure_steady_state(|| vibe_ai::tools::parse_tool_calls(CHUNK));

    // Was 91,504. The budget is ~5x the measured 180, so ordinary churn is
    // fine and a reintroduced per-call regex compile is not.
    assert!(
        stats.allocations < 1_000,
        "parse_tool_calls allocated {} times per chunk: {stats:?}",
        stats.allocations
    );
}

#[test]
fn parsing_plain_prose_stays_within_budget() {
    let stats = measure_steady_state(|| vibe_ai::tools::parse_tool_calls(PROSE));

    // Was 100,503 allocations and 41 MB — to learn that a fragment of prose
    // contains no tool call. Now 226.
    assert!(
        stats.allocations < 1_000,
        "parse_tool_calls allocated {} times on prose: {stats:?}",
        stats.allocations
    );
}

#[test]
fn stripping_thinking_stays_within_budget() {
    let stats = measure_steady_state(|| vibe_ai::tools::strip_thinking(CHUNK));

    // Was 46,559. This is the genuinely per-streamed-chunk path, so it is the
    // one whose budget matters most.
    assert!(
        stats.allocations < 300,
        "strip_thinking allocated {} times per chunk: {stats:?}",
        stats.allocations
    );
}

/// The property that actually matters: cost must not scale with how many tools
/// exist. Compiling a regex per tool name per call is exactly that failure, and
/// it gets worse every time a tool is added.
#[test]
fn cost_does_not_scale_with_the_number_of_tools() {
    let one_call = measure_steady_state(|| vibe_ai::tools::parse_tool_calls(CHUNK));
    let n_tools = vibe_ai::tools::AVAILABLE_TOOL_NAMES.len() as u64;

    assert!(
        one_call.allocations < n_tools * 100,
        "{} allocations for {n_tools} tools looks like per-tool work: {one_call:?}",
        one_call.allocations
    );
}
