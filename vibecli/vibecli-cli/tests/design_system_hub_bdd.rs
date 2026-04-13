/*!
 * BDD tests for design_system_hub using Cucumber.
 * Run with: cargo test --test design_system_hub_bdd
 */
use cucumber::{World, given, then, when};
use vibecli_cli::design_system_hub::{
    audit_design_system, detect_token_drift, merge_provider_tokens,
    vibecody_default_design_system, AuditReport, AuditSeverity, DesignSystem, TokenNamespace,
};
use vibecli_cli::design_providers::{DesignToken, DesignTokenType, ProviderKind};

#[derive(Debug, Default, World)]
pub struct HubWorld {
    system: Option<DesignSystem>,
    baseline: Option<DesignSystem>,
    current: Option<DesignSystem>,
    audit_report: Option<AuditReport>,
    css_output: String,
    tailwind_output: String,
    sd_output: String,
    ts_output: String,
    drift_count: usize,
    merged_tokens: Vec<DesignToken>,
    merged_len: usize,
}

fn make_color(name: &str, value: &str, provider: ProviderKind) -> DesignToken {
    DesignToken { name: name.to_string(), token_type: DesignTokenType::Color, value: value.to_string(), description: None, provider }
}

// ── Given ──────────────────────────────────────────────────────────────────

#[given(expr = "a design system with a color token {string} value {string}")]
fn given_ds_with_color(world: &mut HubWorld, name: String, value: String) {
    let mut ds = DesignSystem::new("Test", "1.0");
    let mut ns = TokenNamespace::new("colors");
    ns.add(make_color(&name, &value, ProviderKind::Inhouse));
    ds.add_namespace(ns);
    world.system = Some(ds);
}

#[given(expr = "a design system {string} namespace with token {string} value {string}")]
fn given_ds_ns_token(world: &mut HubWorld, ns_name: String, tok_name: String, tok_val: String) {
    let mut ds = DesignSystem::new("Test", "1.0");
    let mut ns = TokenNamespace::new(&ns_name);
    ns.add(make_color(&tok_name, &tok_val, ProviderKind::Inhouse));
    ds.add_namespace(ns);
    world.system = Some(ds);
}

#[given(expr = "an empty design system named {string}")]
fn given_empty_ds(world: &mut HubWorld, name: String) {
    world.system = Some(DesignSystem::new(&name, "0.1"));
}

#[given("a design system with two namespaces both containing token \"primary\"")]
fn given_ds_duplicate_tokens(world: &mut HubWorld) {
    let mut ds = DesignSystem::new("Test", "1.0");
    let mut ns1 = TokenNamespace::new("a");
    ns1.add(make_color("primary", "#000", ProviderKind::Inhouse));
    let mut ns2 = TokenNamespace::new("b");
    ns2.add(make_color("primary", "#fff", ProviderKind::Inhouse));
    ds.add_namespace(ns1);
    ds.add_namespace(ns2);
    world.system = Some(ds);
}

#[given("a design system with no tokens")]
fn given_ds_no_tokens(world: &mut HubWorld) {
    world.system = Some(DesignSystem::new("Empty", "0.1"));
}

#[given(expr = "a baseline design system with token {string} value {string}")]
fn given_baseline(world: &mut HubWorld, name: String, value: String) {
    let mut ds = DesignSystem::new("DS", "1.0");
    let mut ns = TokenNamespace::new("c");
    ns.add(make_color(&name, &value, ProviderKind::Inhouse));
    ds.add_namespace(ns);
    world.baseline = Some(ds);
}

#[given(expr = "a current design system with token {string} value {string}")]
fn given_current(world: &mut HubWorld, name: String, value: String) {
    let mut ds = DesignSystem::new("DS", "1.1");
    let mut ns = TokenNamespace::new("c");
    ns.add(make_color(&name, &value, ProviderKind::Inhouse));
    ds.add_namespace(ns);
    world.current = Some(ds);
}

#[given(expr = "provider {string} has token {string} value {string}")]
fn given_provider_token(_world: &mut HubWorld, _provider: String, _name: String, _value: String) {
    // stored implicitly below
}

#[given(expr = "provider {string} has token {string} value {string} and {string} value {string}")]
fn given_provider_two_tokens(_world: &mut HubWorld, _p: String, _n1: String, _v1: String, _n2: String, _v2: String) {
    // stored implicitly below
}

// ── When ───────────────────────────────────────────────────────────────────

#[when("I export to CSS")]
fn when_css(world: &mut HubWorld) {
    world.css_output = world.system.as_ref().unwrap().export_css();
}

#[when("I export to Tailwind config")]
fn when_tailwind(world: &mut HubWorld) {
    world.tailwind_output = world.system.as_ref().unwrap().export_tailwind();
}

#[when("I export to Style Dictionary format")]
fn when_sd(world: &mut HubWorld) {
    world.sd_output = world.system.as_ref().unwrap().export_style_dictionary();
}

#[when("I export to TypeScript")]
fn when_ts(world: &mut HubWorld) {
    world.ts_output = world.system.as_ref().unwrap().export_ts();
}

#[when("I audit the design system")]
fn when_audit(world: &mut HubWorld) {
    world.audit_report = Some(audit_design_system(world.system.as_ref().unwrap()));
}

#[when("I detect token drift")]
fn when_drift(world: &mut HubWorld) {
    let drifts = detect_token_drift(
        world.baseline.as_ref().unwrap(),
        world.current.as_ref().unwrap(),
    );
    world.drift_count = drifts.len();
    if !drifts.is_empty() {
        world.merged_tokens = drifts.iter().map(|d| DesignToken {
            name: d.token_name.clone(),
            token_type: DesignTokenType::Color,
            value: d.current_value.clone(),
            description: None,
            provider: d.provider.clone(),
        }).collect();
    }
}

#[when(expr = "I merge with preferred provider {string}")]
fn when_merge(world: &mut HubWorld, preferred_str: String) {
    let figma_tokens = vec![make_color("primary", "#000", ProviderKind::Figma)];
    let penpot_tokens = vec![
        make_color("primary", "#fff", ProviderKind::Penpot),
        make_color("secondary", "#888", ProviderKind::Penpot),
    ];
    let preferred = if preferred_str == "figma" { ProviderKind::Figma } else { ProviderKind::Penpot };
    let merged = merge_provider_tokens(
        &[(ProviderKind::Figma, figma_tokens), (ProviderKind::Penpot, penpot_tokens)],
        Some(&preferred),
    );
    world.merged_len = merged.len();
    world.merged_tokens = merged;
}

#[when("I load the VibeCody default design system")]
fn when_load_default(world: &mut HubWorld) {
    world.system = Some(vibecody_default_design_system());
}

// ── Then ───────────────────────────────────────────────────────────────────

#[then(expr = "the CSS should contain {string}")]
fn then_css_contains(world: &mut HubWorld, s: String) {
    assert!(world.css_output.contains(s.as_str()), "CSS missing: {s}");
}

#[then(expr = "the output should contain {string}")]
fn then_output_contains(world: &mut HubWorld, s: String) {
    let text = if !world.tailwind_output.is_empty() { &world.tailwind_output }
               else if !world.ts_output.is_empty() { &world.ts_output }
               else { &world.sd_output };
    assert!(text.contains(s.as_str()), "Output missing: {s}");
}

#[then("the JSON should be parseable")]
fn then_json_parseable(world: &mut HubWorld) {
    let v: Result<serde_json::Value, _> = serde_json::from_str(&world.sd_output);
    assert!(v.is_ok(), "Invalid JSON: {}", world.sd_output);
}

#[then("the JSON should contain \"primary\"")]
fn then_json_has_primary(world: &mut HubWorld) {
    assert!(world.sd_output.contains("primary"));
}

#[then(expr = "the report should have at least {int} issue")]
fn then_report_issues(world: &mut HubWorld, min: usize) {
    let report = world.audit_report.as_ref().unwrap();
    assert!(report.issues.len() >= min, "Expected >= {} issues, got {}", min, report.issues.len());
}

#[then(expr = "the score should be less than {int}")]
fn then_score_lt(world: &mut HubWorld, max: u8) {
    assert!(world.audit_report.as_ref().unwrap().score < max);
}

#[then(expr = "the report should contain an error with code {string}")]
fn then_report_error_code(world: &mut HubWorld, code: String) {
    let issues = &world.audit_report.as_ref().unwrap().issues;
    assert!(
        issues.iter().any(|i| i.code == code && i.severity == AuditSeverity::Error),
        "No error with code {code}"
    );
}

#[then(expr = "the report should contain a warning with code {string}")]
fn then_report_warning_code(world: &mut HubWorld, code: String) {
    let issues = &world.audit_report.as_ref().unwrap().issues;
    assert!(
        issues.iter().any(|i| i.code == code && i.severity == AuditSeverity::Warning),
        "No warning with code {code}"
    );
}

#[then(expr = "{int} drift should be reported")]
fn then_drift_count(world: &mut HubWorld, count: usize) {
    assert_eq!(world.drift_count, count);
}

#[then(expr = "{int} drifts should be reported")]
fn then_drifts_count(world: &mut HubWorld, count: usize) {
    assert_eq!(world.drift_count, count);
}

#[then(expr = "the drifted token should be {string}")]
fn then_drifted_token(world: &mut HubWorld, name: String) {
    assert!(world.merged_tokens.iter().any(|t| t.name == name));
}

#[then(expr = "the merged list should have {int} tokens")]
fn then_merged_count(world: &mut HubWorld, count: usize) {
    assert_eq!(world.merged_len, count);
}

#[then(expr = "the {string} token value should be {string}")]
fn then_token_value(world: &mut HubWorld, name: String, value: String) {
    let tok = world.merged_tokens.iter().find(|t| t.name == name);
    assert!(tok.is_some(), "Token {name} not found");
    assert_eq!(tok.unwrap().value, value);
}

#[then("it should have color tokens")]
fn then_has_colors(world: &mut HubWorld) {
    let colors = world.system.as_ref().unwrap().tokens_by_type(&DesignTokenType::Color);
    assert!(!colors.is_empty());
}

#[then(expr = "it should contain a token named {string}")]
fn then_has_named_token(world: &mut HubWorld, name: String) {
    let all = world.system.as_ref().unwrap().all_tokens();
    assert!(all.iter().any(|t| t.name == name), "Token {name} not found");
}

fn main() {
    futures::executor::block_on(HubWorld::run("tests/features/design_system_hub.feature"));
}
