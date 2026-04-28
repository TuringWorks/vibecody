//! BDD: BrokerConfig TOML parsing — verifies the wiring config the
//! daemon will read at startup.

use cucumber::{World, given, then, when};
use vibe_broker::{BrokerConfig, ConfigError};

#[derive(Default, World)]
pub struct CWorld {
    toml_text: String,
    parsed: Option<Result<BrokerConfig, ConfigError>>,
}

impl std::fmt::Debug for CWorld {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("CWorld").finish()
    }
}

#[given(expr = "a config TOML with TCP listener {string} and policy_id {string}")]
fn minimal_tcp(world: &mut CWorld, addr: String, policy_id: String) {
    world.toml_text = format!(
        r#"
[broker]
listen_tcp = "{addr}"
policy_id = "{policy_id}"
"#
    );
}

#[given("a full config TOML with UDS listener, tls dir, jsonl audit, IMDS section")]
fn full_uds(world: &mut CWorld) {
    world.toml_text = r#"
[broker]
listen_uds = "/run/vibe-broker.sock"
tls_ca_dir = "/var/run/vibe-ca"

[broker.audit]
jsonl_path = "/var/log/vibe-audit.jsonl"

[broker.imds]
role_name = "vibe-broker-role"
secret_ref = "@workspace.aws_default"
listen_tcp = "127.0.0.1:8181"
"#
    .into();
}

#[given("a config TOML with one azure profile and one gcp profile")]
fn refresher_profiles(world: &mut CWorld) {
    world.toml_text = r#"
[broker]
listen_uds = "/run/vibe-broker.sock"

[refresher]
interval_secs = 300

[[azure]]
secret_ref = "@workspace.azure_default"
tenant = "tenant42"
client_id = "client42"
client_secret = "secret42"
scope = "default"

[[gcp]]
secret_ref = "@workspace.gcp_default"
client_email = "sa@example.iam.gserviceaccount.com"
private_key_pem_path = "/etc/vibe/gcp-key.pem"
scope = "https://www.googleapis.com/auth/cloud-platform"
"#
    .into();
}

#[given("a malformed config TOML")]
fn malformed(world: &mut CWorld) {
    world.toml_text = "this is not = valid = toml syntax]]]".into();
}

#[when("I parse the config")]
fn parse(world: &mut CWorld) {
    world.parsed = Some(BrokerConfig::from_toml_str(&world.toml_text));
}

fn cfg(world: &CWorld) -> &BrokerConfig {
    world
        .parsed
        .as_ref()
        .expect("parse called")
        .as_ref()
        .expect("parse succeeded")
}

#[then(expr = "the parsed listener kind is {string}")]
fn listener_kind(world: &mut CWorld, expected: String) {
    let actual = match cfg(world).listener_kind() {
        vibe_broker::ListenerKind::Tcp => "tcp",
        vibe_broker::ListenerKind::Uds => "uds",
    };
    assert_eq!(actual, expected);
}

#[then(expr = "the parsed listener address is {string}")]
fn listener_address(world: &mut CWorld, expected: String) {
    assert_eq!(cfg(world).listener_address(), expected);
}

#[then(expr = "the parsed policy_id is {string}")]
fn policy_id(world: &mut CWorld, expected: String) {
    assert_eq!(cfg(world).broker.policy_id, expected);
}

#[then(expr = "the parsed tls_ca_dir is {string}")]
fn tls_ca_dir(world: &mut CWorld, expected: String) {
    let p = cfg(world)
        .broker
        .tls_ca_dir
        .as_ref()
        .map(|p| p.to_string_lossy().into_owned())
        .unwrap_or_default();
    assert_eq!(p, expected);
}

#[then(expr = "the parsed audit jsonl path is {string}")]
fn audit_path(world: &mut CWorld, expected: String) {
    let p = cfg(world)
        .broker
        .audit
        .as_ref()
        .map(|a| a.jsonl_path.to_string_lossy().into_owned())
        .unwrap_or_default();
    assert_eq!(p, expected);
}

#[then(expr = "the parsed IMDS role_name is {string}")]
fn imds_role(world: &mut CWorld, expected: String) {
    assert_eq!(
        cfg(world).broker.imds.as_ref().unwrap().role_name,
        expected
    );
}

#[then(expr = "the parsed IMDS listen_tcp is {string}")]
fn imds_listen(world: &mut CWorld, expected: String) {
    assert_eq!(
        cfg(world).broker.imds.as_ref().unwrap().listen_tcp,
        expected
    );
}

#[then(expr = "the parsed refresher has {int} azure profiles")]
fn azure_count(world: &mut CWorld, expected: usize) {
    assert_eq!(cfg(world).azure.len(), expected);
}

#[then(expr = "the parsed refresher has {int} gcp profiles")]
fn gcp_count(world: &mut CWorld, expected: usize) {
    assert_eq!(cfg(world).gcp.len(), expected);
}

#[then(expr = "the parsed first azure tenant is {string}")]
fn first_azure_tenant(world: &mut CWorld, expected: String) {
    assert_eq!(cfg(world).azure[0].tenant, expected);
}

#[then(expr = "the parsed first gcp client_email is {string}")]
fn first_gcp_email(world: &mut CWorld, expected: String) {
    assert_eq!(cfg(world).gcp[0].client_email, expected);
}

#[then("the parse result is an error")]
fn is_err(world: &mut CWorld) {
    assert!(world.parsed.as_ref().unwrap().is_err());
}

fn main() {
    futures::executor::block_on(CWorld::run(
        "tests/features/broker_config.feature",
    ));
}
