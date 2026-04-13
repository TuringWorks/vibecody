/*!
 * BDD tests for autodream using Cucumber.
 * Run with: cargo test --test autodream_bdd
 */
use cucumber::{World, given, then, when};
use vibecli_cli::autodream::{AutoDream, ConsolidationPolicy, ConsolidationResult, MemoryEntry};

#[derive(Debug, Default, World)]
pub struct AdWorld {
    entries: Vec<MemoryEntry>,
    result: Option<ConsolidationResult>,
    max_age_secs: u64,
    max_entries: usize,
}

impl AdWorld {
    fn dream(&self) -> AutoDream {
        AutoDream::new(ConsolidationPolicy {
            max_age_secs: if self.max_age_secs == 0 {
                999_999_999
            } else {
                self.max_age_secs
            },
            max_entries: if self.max_entries == 0 {
                1000
            } else {
                self.max_entries
            },
            ..Default::default()
        })
    }

    fn unix_now() -> u64 {
        use std::time::{SystemTime, UNIX_EPOCH};
        SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map(|d| d.as_secs())
            .unwrap_or(0)
    }
}

// ---------------------------------------------------------------------------
// Given
// ---------------------------------------------------------------------------

#[given("a memory store with two entries for key \"topic\"")]
fn two_entries_same_key(world: &mut AdWorld) {
    let now = AdWorld::unix_now();
    world.entries.push(MemoryEntry::new("topic", "value one", now));
    world.entries.push(MemoryEntry::new("topic", "value two", now));
}

#[given("a memory store with an entry created 30 days ago")]
fn old_entry(world: &mut AdWorld) {
    use std::time::{SystemTime, UNIX_EPOCH};
    let now = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_secs())
        .unwrap_or(0);
    let thirty_days = 30 * 86_400;
    world.entries.push(MemoryEntry::new("old_key", "stale", now - thirty_days));
}

#[given("the max age policy is 7 days")]
fn set_max_age(world: &mut AdWorld) {
    world.max_age_secs = 7 * 86_400;
}

#[given("a memory store with 3 entries and a max_entries limit of 2")]
fn three_entries_limit_two(world: &mut AdWorld) {
    world.max_entries = 2;
    let now = AdWorld::unix_now();
    let mut rare = MemoryEntry::new("rare", "v", now);
    rare.access_count = 1;
    let mut common = MemoryEntry::new("common", "v", now);
    common.access_count = 10;
    let mut freq = MemoryEntry::new("freq", "v", now);
    freq.access_count = 20;
    world.entries.push(rare);
    world.entries.push(common);
    world.entries.push(freq);
}

#[given(expr = "the entry with key {string} has access_count {int}")]
fn set_access_count(world: &mut AdWorld, key: String, count: u32) {
    if let Some(e) = world.entries.iter_mut().find(|e| e.key == key) {
        e.access_count = count;
    } else {
        let now = AdWorld::unix_now();
        let mut entry = MemoryEntry::new(key, "v", now);
        entry.access_count = count;
        world.entries.push(entry);
    }
}

#[given("a memory store with two entries")]
fn two_entries(world: &mut AdWorld) {
    let now = AdWorld::unix_now();
    world.entries.push(MemoryEntry::new("popular", "v1", now));
    world.entries.push(MemoryEntry::new("rare", "v2", now));
}

// ---------------------------------------------------------------------------
// When
// ---------------------------------------------------------------------------

#[when("I consolidate the memory")]
fn do_consolidate(world: &mut AdWorld) {
    let entries = world.entries.drain(..).collect();
    world.result = Some(world.dream().consolidate(entries));
}

#[when("I rank the entries")]
fn do_rank(world: &mut AdWorld) {
    let entries = world.entries.drain(..).collect();
    let ranked = world.dream().rank_by_relevance(entries);
    world.result = Some(vibecli_cli::autodream::ConsolidationResult {
        merged: 0,
        pruned: 0,
        kept: ranked.len(),
        entries: ranked,
    });
}

// ---------------------------------------------------------------------------
// Then
// ---------------------------------------------------------------------------

#[then(expr = "the result should have {int} kept entry")]
fn check_kept_singular(world: &mut AdWorld, expected: usize) {
    let r = world.result.as_ref().unwrap();
    assert_eq!(r.kept, expected, "kept={}", r.kept);
}

#[then(expr = "the result should have {int} kept entries")]
fn check_kept(world: &mut AdWorld, expected: usize) {
    let r = world.result.as_ref().unwrap();
    assert_eq!(r.kept, expected, "kept={}", r.kept);
}

#[then(expr = "the kept entries should not include key {string}")]
fn check_not_includes(world: &mut AdWorld, key: String) {
    let r = world.result.as_ref().unwrap();
    assert!(
        r.entries.iter().all(|e| e.key != key),
        "entry with key {:?} should not be present",
        key
    );
}

#[then(expr = "the first entry should have key {string}")]
fn check_first_key(world: &mut AdWorld, key: String) {
    let r = world.result.as_ref().unwrap();
    assert!(
        !r.entries.is_empty(),
        "entries are empty"
    );
    assert_eq!(r.entries[0].key, key);
}

fn main() {
    futures::executor::block_on(AdWorld::run("tests/features/autodream.feature"));
}
