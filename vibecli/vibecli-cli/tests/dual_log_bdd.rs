/*!
 * BDD tests for the dual_log module.
 * Run with: cargo test --test dual_log_bdd
 */
use cucumber::{given, then, when, World};
use tempfile::TempDir;
use vibecli_cli::dual_log::{DualLog, LogEntry, LogRole};

// ---------------------------------------------------------------------------
// World
// ---------------------------------------------------------------------------

#[derive(Debug, World)]
pub struct DlWorld {
    dl: DualLog,
    grep_results: Vec<String>, // entry ids from last grep
    serialised_full: String,
    serialised_ctx: String,
    tmp_dir: Option<TempDir>,
}

impl Default for DlWorld {
    fn default() -> Self {
        Self {
            dl: DualLog::new(10),
            grep_results: Vec::new(),
            serialised_full: String::new(),
            serialised_ctx: String::new(),
            tmp_dir: None,
        }
    }
}

// ---------------------------------------------------------------------------
// Given
// ---------------------------------------------------------------------------

#[given(expr = "a dual-log with max context {int}")]
fn given_dual_log(world: &mut DlWorld, max: usize) {
    world.dl = DualLog::new(max);
}

// ---------------------------------------------------------------------------
// When
// ---------------------------------------------------------------------------

#[when(expr = "I append a {string} entry {string} with id {string} at time {int}")]
fn when_append(world: &mut DlWorld, role_str: String, content: String, id: String, ts: u64) {
    let role = LogRole::from_str(&role_str);
    world.dl.append(LogEntry::new(id, role, content, ts));
}

#[when(expr = "I append an {string} entry {string} with id {string} at time {int}")]
fn when_append_an(world: &mut DlWorld, role_str: String, content: String, id: String, ts: u64) {
    when_append(world, role_str, content, id, ts);
}

#[when(expr = "I append {int} {string} entries starting at time {int}")]
fn when_append_n(world: &mut DlWorld, n: usize, role_str: String, start_ts: u64) {
    let role = LogRole::from_str(&role_str);
    for i in 0..n {
        let ts = start_ts + i as u64;
        world.dl.append(LogEntry::new(
            format!("auto-{i}"),
            role.clone(),
            format!("msg {i}"),
            ts,
        ));
    }
}

#[when("I sync the context")]
fn when_sync(world: &mut DlWorld) {
    world.dl.sync_context();
}

#[when(expr = "I compact with summary {string} keeping {int} recent entries")]
fn when_compact(world: &mut DlWorld, summary: String, keep: usize) {
    world.dl.compact(&summary, keep);
}

#[when("I serialize and reload the dual-log")]
fn when_serialize_reload(world: &mut DlWorld) {
    world.serialised_full = world.dl.serialize_full_log();
    world.serialised_ctx = world.dl.serialize_context();
    let max = 10;
    world.dl = DualLog::load(&world.serialised_full, &world.serialised_ctx, max)
        .expect("DualLog::load failed");
}

#[when("I persist the dual-log to a temporary directory")]
fn when_persist(world: &mut DlWorld) {
    let tmp = TempDir::new().expect("TempDir::new");
    let log_path = tmp.path().join("log.jsonl");
    let ctx_path = tmp.path().join("context.jsonl");
    world.dl.persist(&log_path, &ctx_path).expect("persist failed");
    world.tmp_dir = Some(tmp);
}

#[when("I load the dual-log from that temporary directory")]
fn when_load(world: &mut DlWorld) {
    let tmp = world.tmp_dir.as_ref().expect("no tmp dir");
    let log_path = tmp.path().join("log.jsonl");
    let ctx_path = tmp.path().join("context.jsonl");
    let full_src = std::fs::read_to_string(&log_path).expect("read log.jsonl");
    let ctx_src = std::fs::read_to_string(&ctx_path).expect("read context.jsonl");
    world.dl = DualLog::load(&full_src, &ctx_src, 5).expect("load failed");
}

// Grep step — stores matching entry ids
#[when(expr = "I grep for {string}")]
fn when_grep(world: &mut DlWorld, pattern: String) {
    world.grep_results = world
        .dl
        .grep_log(&pattern)
        .iter()
        .map(|e| e.id.clone())
        .collect();
}

// ---------------------------------------------------------------------------
// Then
// ---------------------------------------------------------------------------

#[then(expr = "the full log count should be {int}")]
fn then_full_count(world: &mut DlWorld, expected: usize) {
    assert_eq!(
        world.dl.full_log_count(),
        expected,
        "full_log_count mismatch"
    );
}

#[then(expr = "the context count should be {int}")]
fn then_ctx_count(world: &mut DlWorld, expected: usize) {
    assert_eq!(
        world.dl.context_count(),
        expected,
        "context_count mismatch"
    );
}

#[then(expr = "the unsynced count should be {int}")]
fn then_unsynced(world: &mut DlWorld, expected: usize) {
    assert_eq!(
        world.dl.unsynced_count(),
        expected,
        "unsynced_count mismatch"
    );
}

#[then("the first context entry should be a compaction summary")]
fn then_first_is_summary(world: &mut DlWorld) {
    let first = world
        .dl
        .context_entries()
        .first()
        .expect("context is empty");
    assert!(first.is_compacted, "expected first entry to be a compaction summary");
}

#[then(expr = "the context should not contain entry {string}")]
fn then_ctx_not_contain(world: &mut DlWorld, id: String) {
    let found = world.dl.context_entries().iter().any(|e| e.id == id);
    assert!(!found, "context unexpectedly contains entry '{id}'");
}

#[then(expr = "grepping for {string} should return {int} result")]
fn then_grep_count(world: &mut DlWorld, pattern: String, expected: usize) {
    let results = world.dl.grep_log(&pattern);
    assert_eq!(
        results.len(),
        expected,
        "grep result count mismatch for pattern '{pattern}'"
    );
    // Cache ids for subsequent step.
    world.grep_results = results.iter().map(|e| e.id.clone()).collect();
}

#[then(expr = "the grep result id should be {string}")]
fn then_grep_id(world: &mut DlWorld, expected_id: String) {
    assert!(
        world.grep_results.contains(&expected_id),
        "expected id '{expected_id}' in grep results: {:?}",
        world.grep_results
    );
}

#[then(expr = "the entry at full-log index {int} should have content {string}")]
fn then_entry_content(world: &mut DlWorld, idx: usize, expected: String) {
    let entry = &world.dl.full_log_entries()[idx];
    assert_eq!(entry.content, expected, "content mismatch at index {idx}");
}

// ---------------------------------------------------------------------------
// Entry point
// ---------------------------------------------------------------------------

fn main() {
    futures::executor::block_on(DlWorld::run("tests/features/dual_log.feature"));
}
