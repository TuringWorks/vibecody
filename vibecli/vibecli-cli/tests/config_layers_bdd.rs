/*!
 * BDD tests for config_layers (Subsystem B — three-level JSON deep-merge).
 * Run with: cargo test --test config_layers_bdd
 */
use cucumber::{World, given, then, when};
use serde_json::{json, Value};
use vibecli_cli::config_layers::{ConfigLayer, ConfigError, LayeredConfig};

#[derive(Debug, Default, World)]
pub struct ClWorld {
    config: LayeredConfig,
    merged: Option<Value>,
    validation_errors: Vec<ConfigError>,
    validate_layer_value: Option<Value>,
}

#[given(expr = "user config with model {string}")]
fn user_model(world: &mut ClWorld, model: String) {
    world.config.user = json!({"model": model});
}

#[given(expr = "project config with model {string}")]
fn project_model(world: &mut ClWorld, model: String) {
    world.config.project = json!({"model": model});
}

#[given(expr = "local config with model {string}")]
fn local_model(world: &mut ClWorld, model: String) {
    world.config.local = json!({"model": model});
}

#[given(expr = "user config with keys x={int} and y={int} in object {string}")]
fn user_nested(world: &mut ClWorld, x: i64, y: i64, key: String) {
    let mut inner = serde_json::Map::new();
    inner.insert("x".into(), json!(x));
    inner.insert("y".into(), json!(y));
    let mut outer = serde_json::Map::new();
    outer.insert(key, Value::Object(inner));
    world.config.user = Value::Object(outer);
}

#[given(expr = "project config overriding y={int} in object {string}")]
fn project_nested(world: &mut ClWorld, y: i64, key: String) {
    let mut inner = serde_json::Map::new();
    inner.insert("y".into(), json!(y));
    let mut outer = serde_json::Map::new();
    outer.insert(key, Value::Object(inner));
    world.config.project = Value::Object(outer);
}

#[given("a non-object value in the project layer")]
fn bad_project(world: &mut ClWorld) {
    world.validate_layer_value = Some(Value::String("not-an-object".into()));
}

#[when("I merge all layers")]
fn merge(world: &mut ClWorld) {
    world.merged = Some(world.config.merge());
}

#[when("I validate the project layer")]
fn validate_project(world: &mut ClWorld) {
    if let Some(v) = &world.validate_layer_value {
        world.validation_errors =
            LayeredConfig::validate_schema(v, &ConfigLayer::Project);
    }
}

#[then(expr = "the merged model should be {string}")]
fn check_model(world: &mut ClWorld, expected: String) {
    let model = world.merged.as_ref().unwrap()["model"]
        .as_str()
        .unwrap();
    assert_eq!(model, expected.as_str());
}

#[then(expr = "{string} should be {int}")]
fn check_nested_int(world: &mut ClWorld, path: String, expected: i64) {
    let merged = world.merged.as_ref().unwrap();
    let mut current = merged;
    for key in path.split('.') {
        current = &current[key];
    }
    // JSON integers may be stored as u64 when positive; accept both.
    let actual = current
        .as_i64()
        .or_else(|| current.as_u64().map(|n| n as i64))
        .unwrap_or_else(|| panic!("expected integer at path '{}', got {:?}", path, current));
    assert_eq!(actual, expected);
}

#[then(expr = "the validation error should reference {string}")]
fn check_error_layer(world: &mut ClWorld, layer_name: String) {
    assert!(
        !world.validation_errors.is_empty(),
        "expected validation errors but got none"
    );
    let err_str = world.validation_errors[0].to_string();
    assert!(
        err_str.contains(&layer_name),
        "error '{}' should reference '{}'",
        err_str,
        layer_name
    );
}

fn main() {
    futures::executor::block_on(
        ClWorld::run("tests/features/config_layers.feature"),
    );
}
