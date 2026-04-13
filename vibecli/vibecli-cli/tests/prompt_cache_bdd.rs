/*!
 * BDD tests for prompt_cache using Cucumber.
 * Run with: cargo test --test prompt_cache_bdd
 */
use cucumber::{World, given, then, when};
use vibecli_cli::prompt_cache::{CacheKey, PromptCache};

#[derive(Debug, Default, World)]
pub struct PcWorld {
    system: String,
    tools: String,
    config: String,
    key1: Option<CacheKey>,
    key2: Option<CacheKey>,
    cache: Option<PromptCache>,
    inserted_key: Option<CacheKey>,
}

// ---------------------------------------------------------------------------
// Given
// ---------------------------------------------------------------------------

#[given(expr = "the system prompt {string}")]
fn set_system(world: &mut PcWorld, s: String) {
    world.system = s;
}

#[given(expr = "the tools json {string}")]
fn set_tools(world: &mut PcWorld, t: String) {
    world.tools = t;
}

#[given(expr = "the config json {string}")]
fn set_config(world: &mut PcWorld, c: String) {
    world.config = c;
}

#[given("a fresh prompt cache")]
fn fresh_cache(world: &mut PcWorld) {
    world.cache = Some(PromptCache::new());
    world.system = "sys".to_string();
    world.tools = "tools".to_string();
    world.config = "cfg".to_string();
}

#[given("I have inserted a prefix entry")]
fn insert_prefix(world: &mut PcWorld) {
    let cache = world.cache.as_mut().expect("cache not initialised");
    let prefix = cache.get_or_insert(&world.system, &world.tools, &world.config);
    world.inserted_key = Some(prefix.key);
}

// ---------------------------------------------------------------------------
// When
// ---------------------------------------------------------------------------

#[when("I compute the cache key twice")]
fn compute_twice(world: &mut PcWorld) {
    world.key1 = Some(CacheKey::from_parts(&world.system, &world.tools, &world.config));
    world.key2 = Some(CacheKey::from_parts(&world.system, &world.tools, &world.config));
}

#[when("I call get_or_insert with the same inputs twice")]
fn call_twice(world: &mut PcWorld) {
    let cache = world.cache.as_mut().expect("cache not initialised");
    cache.get_or_insert(&world.system, &world.tools, &world.config);
    cache.get_or_insert(&world.system, &world.tools, &world.config);
}

#[when("I invalidate that entry")]
fn do_invalidate(world: &mut PcWorld) {
    let key = world.inserted_key.expect("no key to invalidate");
    let cache = world.cache.as_mut().expect("cache not initialised");
    cache.invalidate(key);
}

// ---------------------------------------------------------------------------
// Then
// ---------------------------------------------------------------------------

#[then("both keys should be equal")]
fn keys_equal(world: &mut PcWorld) {
    assert_eq!(world.key1, world.key2);
}

#[then(expr = "the miss count should be {int}")]
fn check_misses(world: &mut PcWorld, expected: u64) {
    let cache = world.cache.as_ref().expect("cache not initialised");
    assert_eq!(cache.stats().misses, expected);
}

#[then(expr = "the hit count should be {int}")]
fn check_hits(world: &mut PcWorld, expected: u64) {
    let cache = world.cache.as_ref().expect("cache not initialised");
    assert_eq!(cache.stats().hits, expected);
}

#[then(expr = "the hit rate should be {float}")]
fn check_hit_rate(world: &mut PcWorld, expected: f64) {
    let cache = world.cache.as_ref().expect("cache not initialised");
    let rate = cache.hit_rate();
    assert!(
        (rate - expected).abs() < 1e-9,
        "hit_rate={} expected={}",
        rate,
        expected
    );
}

#[then(expr = "the cache should have {int} entries")]
fn check_entries(world: &mut PcWorld, expected: usize) {
    let cache = world.cache.as_ref().expect("cache not initialised");
    assert_eq!(cache.stats().entries, expected);
}

fn main() {
    futures::executor::block_on(PcWorld::run("tests/features/prompt_cache.feature"));
}
