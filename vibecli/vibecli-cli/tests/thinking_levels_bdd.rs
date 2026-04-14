/*!
 * BDD tests for the thinking_levels module.
 * Run with: cargo test --test thinking_levels_bdd
 */
use cucumber::{given, then, when, World};
use vibecli_cli::thinking_levels::{
    TaskHint, ThinkingBudgetOverride, ThinkingConfig, ThinkingLevel, ModelWithLevel,
};

// ── World ─────────────────────────────────────────────────────────────────────

#[derive(Debug, Default, World)]
pub struct TlWorld {
    /// Raw shorthand string supplied by a Given step.
    shorthand: String,
    /// Parsed model-with-level.
    parsed_model: Option<ModelWithLevel>,
    /// Single level under test.
    level: ThinkingLevel,
    /// Provider config built by a When step.
    config: Option<ThinkingConfig>,
    /// Task hint for auto-selection tests.
    task_hint: Option<TaskHint>,
    /// Budget override helper.
    override_store: ThinkingBudgetOverride,
    /// Resolved budget value from an override.
    resolved_budget: u32,
}

// ── Given ─────────────────────────────────────────────────────────────────────

#[given(expr = "the model shorthand {string}")]
fn set_shorthand(world: &mut TlWorld, s: String) {
    world.shorthand = s;
}

#[given(expr = "the thinking level {string}")]
fn set_level(world: &mut TlWorld, s: String) {
    world.level = ThinkingLevel::from_str(&s).unwrap_or(ThinkingLevel::Off);
}

#[given(expr = "the task hint {string}")]
fn set_task_hint(world: &mut TlWorld, s: String) {
    world.task_hint = Some(match s.as_str() {
        "SimpleEdit" => TaskHint::SimpleEdit,
        "CodeGeneration" => TaskHint::CodeGeneration,
        "Debugging" => TaskHint::Debugging,
        "Architecture" => TaskHint::Architecture,
        "ComplexReasoning" => TaskHint::ComplexReasoning,
        _ => TaskHint::Unknown,
    });
}

#[given(expr = "a budget override of {int} tokens for level {string}")]
fn set_budget_override(world: &mut TlWorld, tokens: u32, level_str: String) {
    let level = ThinkingLevel::from_str(&level_str).unwrap_or(ThinkingLevel::Off);
    world.override_store.set(level, tokens);
}

// ── When ──────────────────────────────────────────────────────────────────────

#[when("I parse the model shorthand")]
fn do_parse_shorthand(world: &mut TlWorld) {
    world.parsed_model = Some(ModelWithLevel::parse(&world.shorthand));
}

#[when("I build the Anthropic provider config")]
fn do_anthropic_config(world: &mut TlWorld) {
    world.config = Some(ThinkingConfig::for_anthropic(&world.level));
}

#[when("I build the OpenAI provider config")]
fn do_openai_config(world: &mut TlWorld) {
    world.config = Some(ThinkingConfig::for_openai(&world.level));
}

#[when("I build the Gemini provider config")]
fn do_gemini_config(world: &mut TlWorld) {
    world.config = Some(ThinkingConfig::for_gemini(&world.level));
}

#[when(expr = "I resolve the budget for level {string}")]
fn do_resolve_budget(world: &mut TlWorld, level_str: String) {
    let level = ThinkingLevel::from_str(&level_str).unwrap_or(ThinkingLevel::Off);
    world.resolved_budget = world.override_store.resolve(&level);
}

// ── Then ──────────────────────────────────────────────────────────────────────

#[then(expr = "the model name should be {string}")]
fn check_model_name(world: &mut TlWorld, expected: String) {
    let mwl = world.parsed_model.as_ref().expect("model not parsed");
    assert_eq!(mwl.model_name, expected);
}

#[then(expr = "the thinking level should be {string}")]
fn check_model_level(world: &mut TlWorld, expected: String) {
    let mwl = world.parsed_model.as_ref().expect("model not parsed");
    assert_eq!(
        mwl.level.as_str(),
        expected.as_str(),
        "expected level {:?} but got {:?}",
        expected,
        mwl.level.as_str()
    );
}

#[then(expr = "the token budget for level {string} should be {int}")]
fn check_token_budget(world: &mut TlWorld, level_str: String, expected: u32) {
    let level = ThinkingLevel::from_str(&level_str)
        .unwrap_or_else(|| panic!("unknown level: {level_str}"));
    assert_eq!(
        level.token_budget(),
        expected,
        "token budget mismatch for level {:?}",
        level_str
    );
    // Suppress unused warning; world is read-only in this step.
    let _ = &world.level;
}

#[then("the config should be enabled")]
fn check_config_enabled(world: &mut TlWorld) {
    let cfg = world.config.as_ref().expect("config not built");
    assert!(cfg.enabled, "expected config to be enabled, but it was disabled");
}

#[then(expr = "the provider param should be {string}")]
fn check_provider_param_exact(world: &mut TlWorld, expected: String) {
    let cfg = world.config.as_ref().expect("config not built");
    assert_eq!(
        cfg.provider_param.as_deref(),
        Some(expected.as_str()),
        "provider_param mismatch"
    );
}

#[then(expr = "the provider param should contain {string}")]
fn check_provider_param_contains(world: &mut TlWorld, expected: String) {
    let cfg = world.config.as_ref().expect("config not built");
    let param = cfg.provider_param.as_deref().unwrap_or("");
    assert!(
        param.contains(expected.as_str()),
        "provider_param {:?} does not contain {:?}",
        param,
        expected
    );
}

#[then(expr = "the config token budget should be {int}")]
fn check_config_token_budget(world: &mut TlWorld, expected: u32) {
    let cfg = world.config.as_ref().expect("config not built");
    assert_eq!(cfg.token_budget, expected);
}

#[then(expr = "the auto-selected level should be {string}")]
fn check_auto_level(world: &mut TlWorld, expected: String) {
    let hint = world.task_hint.as_ref().expect("task hint not set");
    let level = ThinkingLevel::default_for_task(hint);
    assert_eq!(
        level.as_str(),
        expected.as_str(),
        "auto-selected level {:?} does not match expected {:?}",
        level.as_str(),
        expected
    );
}

#[then(expr = "the resolved budget should be {int}")]
fn check_resolved_budget(world: &mut TlWorld, expected: u32) {
    assert_eq!(
        world.resolved_budget, expected,
        "resolved budget {} does not match expected {}",
        world.resolved_budget, expected
    );
}

// ── Runner ────────────────────────────────────────────────────────────────────

fn main() {
    futures::executor::block_on(TlWorld::run("tests/features/thinking_levels.feature"));
}
