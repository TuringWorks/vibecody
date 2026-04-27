//! BDD coverage for the egress policy DSL parser + matcher.

use cucumber::{World, given, then, when};
use vibe_broker::{Decision, Policy, policy::Request};

#[derive(Default, World)]
pub struct PWorld {
    policy: Option<Policy>,
    decision: Option<DecisionDescriptor>,
    parsed_inject_type: Option<String>,
}

impl std::fmt::Debug for PWorld {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("PWorld").finish()
    }
}

#[derive(Debug, Clone)]
struct DecisionDescriptor {
    decision: String,
    inject_type: Option<String>,
}

#[given("an empty egress policy")]
fn empty_policy(world: &mut PWorld) {
    world.policy = Some(Policy {
        default: vibe_broker::policy::DefaultRule::Deny,
        rule: vec![],
    });
}

#[given(expr = "a policy with one rule allowing {string} methods {string} with bearer key {string}")]
fn one_rule_bearer(world: &mut PWorld, host: String, methods: String, key: String) {
    let toml = format!(
        r#"
default = "deny"

[[rule]]
match.host = "{host}"
match.methods = [{}]
match.require_tls = false
inject = {{ type = "bearer", key = "{key}" }}
"#,
        methods
            .split(',')
            .map(|m| format!("\"{}\"", m.trim()))
            .collect::<Vec<_>>()
            .join(", "),
    );
    world.policy = Some(Policy::parse_toml(&toml).unwrap());
}

#[given(expr = "a policy with one rule allowing {string} methods {string} with path prefix {string} and bearer key {string}")]
fn one_rule_with_prefix(
    world: &mut PWorld,
    host: String,
    methods: String,
    prefix: String,
    key: String,
) {
    let toml = format!(
        r#"
default = "deny"

[[rule]]
match.host = "{host}"
match.methods = [{}]
match.path_prefix = "{prefix}"
match.require_tls = false
inject = {{ type = "bearer", key = "{key}" }}
"#,
        methods
            .split(',')
            .map(|m| format!("\"{}\"", m.trim()))
            .collect::<Vec<_>>()
            .join(", "),
    );
    world.policy = Some(Policy::parse_toml(&toml).unwrap());
}

#[given(expr = "a sample policy TOML with one rule for {string}")]
fn sample_policy(world: &mut PWorld, host: String) {
    let toml = format!(
        r#"
default = "deny"

[[rule]]
match.host = "{host}"
match.methods = ["GET"]
inject = {{ type = "none" }}
"#
    );
    world.policy = Some(Policy::parse_toml(&toml).unwrap());
}

#[given(expr = "a TOML rule with inject type {string} profile {string}")]
fn aws_inject_rule(world: &mut PWorld, ty: String, profile: String) {
    let toml = format!(
        r#"
default = "deny"

[[rule]]
match.host = "*.amazonaws.com"
match.methods = ["GET", "POST"]
inject = {{ type = "{ty}", profile = "{profile}" }}
"#
    );
    world.policy = Some(Policy::parse_toml(&toml).unwrap());
}

#[when("I parse the rule")]
fn parse_rule(world: &mut PWorld) {
    let p = world.policy.as_ref().unwrap();
    world.parsed_inject_type = Some(p.rule[0].inject.type_name().to_string());
}

#[when("I parse the TOML into a Policy")]
fn parse_toml_into_policy(_: &mut PWorld) {
    // No-op: parsing happened in the Given; we just preserve world.policy.
}

#[when(expr = "I match a request {string}")]
fn match_request(world: &mut PWorld, req_str: String) {
    let mut parts = req_str.splitn(2, ' ');
    let method = parts.next().unwrap();
    let url = parts.next().unwrap();
    let p = world.policy.as_ref().unwrap();
    let d = p.match_request(&Request { method, url });
    world.decision = Some(match d {
        Decision::Allow { inject, .. } => DecisionDescriptor {
            decision: "Allow".into(),
            inject_type: Some(inject.type_name().to_string()),
        },
        Decision::Deny => DecisionDescriptor {
            decision: "Deny".into(),
            inject_type: None,
        },
    });
}

#[then(expr = "the policy decision is {string}")]
fn decision_is(world: &mut PWorld, expected: String) {
    let d = world.decision.as_ref().unwrap();
    assert_eq!(d.decision, expected);
}

#[then(expr = "the inject type is {string}")]
fn inject_type_is(world: &mut PWorld, expected: String) {
    let d = world.decision.as_ref().unwrap();
    assert_eq!(d.inject_type.as_deref(), Some(expected.as_str()));
}

#[then(expr = "the policy has {int} rules")]
fn policy_count(world: &mut PWorld, n: usize) {
    assert_eq!(world.policy.as_ref().unwrap().rule.len(), n);
}

#[then(expr = "the first rule host glob is {string}")]
fn first_rule_host(world: &mut PWorld, expected: String) {
    assert_eq!(world.policy.as_ref().unwrap().rule[0].match_.host, expected);
}

#[then(expr = "the parsed inject type is {string}")]
fn parsed_inject_is(world: &mut PWorld, expected: String) {
    assert_eq!(world.parsed_inject_type.as_deref(), Some(expected.as_str()));
}

fn main() {
    futures::executor::block_on(PWorld::run(
        "tests/features/broker_policy.feature",
    ));
}
