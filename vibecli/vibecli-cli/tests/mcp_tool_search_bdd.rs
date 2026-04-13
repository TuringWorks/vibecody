/*!
 * BDD tests for mcp_tool_search using Cucumber.
 * Run with: cargo test --test mcp_tool_search_bdd
 */
use cucumber::{World, given, then, when};
use serde_json::json;
use vibecli_cli::mcp_tool_search::{ToolRegistry, ToolSchema, ToolStub};

#[derive(Debug, Default, World)]
pub struct MtsWorld {
    registry: ToolRegistry,
    stubs_context: Option<String>,
    savings_pct: f32,
    hit_rate: f64,
}

fn make_schema(name: &str) -> ToolSchema {
    let stub = ToolStub::new(name, format!("Description of {}", name));
    ToolSchema::new(
        stub,
        json!({ "properties": { "path": { "type": "string" } }, "required": ["path"] }),
    )
}

// ---------------------------------------------------------------------------
// Given
// ---------------------------------------------------------------------------

#[given(expr = "a registry with tools {string} {string} {string}")]
fn given_three_tools(world: &mut MtsWorld, t1: String, t2: String, t3: String) {
    for name in [&t1, &t2, &t3] {
        world
            .registry
            .register(ToolStub::new(name.as_str(), format!("Description of {}", name)));
    }
}

#[given("a fresh registry")]
fn given_fresh_registry(world: &mut MtsWorld) {
    world.registry = ToolRegistry::new();
}

// ---------------------------------------------------------------------------
// When
// ---------------------------------------------------------------------------

#[when("I get the stubs context")]
fn when_stubs_context(world: &mut MtsWorld) {
    world.stubs_context = Some(world.registry.stubs_context());
}

#[when(expr = "I load the schema for {string}")]
fn when_load_schema(world: &mut MtsWorld, name: String) {
    let schema = make_schema(&name);
    world.registry.load_schema(&name, schema);
}

#[when("I compute savings for selecting 1 tool")]
fn when_compute_savings(world: &mut MtsWorld) {
    world.savings_pct = world.registry.savings_pct(&["read_file"]);
}

#[when("I record 3 hits and 1 miss")]
fn when_record_hits_misses(world: &mut MtsWorld) {
    world.registry.record_hit();
    world.registry.record_hit();
    world.registry.record_hit();
    world.registry.record_miss();
    world.hit_rate = world.registry.hit_rate();
}

// ---------------------------------------------------------------------------
// Then
// ---------------------------------------------------------------------------

#[then(expr = "the stubs context should contain {string}")]
fn then_stubs_context_contains(world: &mut MtsWorld, expected: String) {
    let ctx = world.stubs_context.as_ref().unwrap();
    assert!(
        ctx.contains(&*expected),
        "Stubs context does not contain '{}'\nContext:\n{}",
        expected,
        ctx
    );
}

#[then("the loaded count should be 1")]
fn then_loaded_count_is_1(world: &mut MtsWorld) {
    assert_eq!(
        world.registry.loaded_count(),
        1,
        "Expected loaded_count 1, got {}",
        world.registry.loaded_count()
    );
}

#[then(expr = "the schema for {string} should be available")]
fn then_schema_available(world: &mut MtsWorld, name: String) {
    assert!(
        world.registry.get_schema(&name).is_some(),
        "Schema for '{}' not available",
        name
    );
}

#[then("the savings percentage should be greater than 0")]
fn then_savings_gt_zero(world: &mut MtsWorld) {
    assert!(
        world.savings_pct > 0.0,
        "Expected savings > 0, got {}",
        world.savings_pct
    );
}

#[then("the hit rate should be 0.75")]
fn then_hit_rate(world: &mut MtsWorld) {
    let delta = (world.hit_rate - 0.75).abs();
    assert!(delta < 0.001, "Expected hit rate 0.75, got {}", world.hit_rate);
}

fn main() {
    futures::executor::block_on(MtsWorld::run("tests/features/mcp_tool_search.feature"));
}
