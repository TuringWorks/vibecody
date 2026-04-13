/*!
 * BDD tests for penpot_connector using Cucumber.
 * Run with: cargo test --test penpot_integration_bdd
 */
use cucumber::{World, given, then, when};
use vibecli_cli::penpot_connector::{
    PenpotColor, PenpotComponent, PenpotConfig, PenpotRequest,
    parse_penpot_file_response, penpot_colors_to_css, penpot_component_to_react,
    validate_penpot_config,
};
use std::collections::HashMap;

#[derive(Debug, Default, World)]
pub struct PenpotWorld {
    config: Option<PenpotConfig>,
    api_url: String,
    validation_result: Option<Result<(), vibecli_cli::design_providers::DesignError>>,
    curl_command: String,
    css_output: String,
    react_code: String,
    parse_result: Option<Result<vibecli_cli::design_providers::DesignFile, vibecli_cli::design_providers::DesignError>>,
    colors: Vec<PenpotColor>,
    component: Option<PenpotComponent>,
}

// ── Given ──────────────────────────────────────────────────────────────────

#[given(expr = "a Penpot config with host {string} and token {string}")]
fn given_config(world: &mut PenpotWorld, host: String, token: String) {
    world.config = Some(PenpotConfig::new(&host, &token));
}

#[given(expr = "a Penpot color named {string} with hex {string}")]
fn given_color(world: &mut PenpotWorld, name: String, hex: String) {
    world.colors.push(PenpotColor {
        id: "c1".into(), name, color: hex, opacity: Some(1.0), path: None,
    });
}

#[given(expr = "a Penpot component named {string} with id {string}")]
fn given_component(world: &mut PenpotWorld, name: String, id: String) {
    world.component = Some(PenpotComponent {
        id, name, path: String::new(), objects: HashMap::new(), main_instance_id: None,
    });
}

#[given(expr = "a Penpot file response JSON string {string}")]
fn given_json_str(world: &mut PenpotWorld, json: String) {
    world.parse_result = Some(parse_penpot_file_response(&json));
}

#[given(expr = "a Penpot file response JSON with id {string} and name {string}")]
fn given_json_with_id(world: &mut PenpotWorld, id: String, name: String) {
    let json = format!(r#"{{"id": "{}", "name": "{}", "data": {{}}}}"#, id, name);
    world.parse_result = Some(parse_penpot_file_response(&json));
}

// ── When ───────────────────────────────────────────────────────────────────

#[when(expr = "I request the API URL for command {string}")]
fn when_api_url(world: &mut PenpotWorld, command: String) {
    world.api_url = world.config.as_ref().unwrap().api_url(&command);
}

#[when("I validate the config")]
fn when_validate(world: &mut PenpotWorld) {
    world.validation_result = Some(validate_penpot_config(world.config.as_ref().unwrap()));
}

#[when("I build a get-profile request")]
fn when_get_profile_request(world: &mut PenpotWorld) {
    // Just store for curl conversion
    let req = PenpotRequest::get_profile(world.config.as_ref().unwrap());
    world.curl_command = req.to_curl();
}

#[when("I convert to curl")]
fn when_to_curl(_world: &mut PenpotWorld) { /* already done */ }

#[when("I export colors to CSS")]
fn when_colors_to_css(world: &mut PenpotWorld) {
    world.css_output = penpot_colors_to_css(&world.colors);
}

#[when("I export to React")]
fn when_export_react(world: &mut PenpotWorld) {
    world.react_code = penpot_component_to_react(world.component.as_ref().unwrap(), "react");
}

#[when("I export to Vue")]
fn when_export_vue(world: &mut PenpotWorld) {
    world.react_code = penpot_component_to_react(world.component.as_ref().unwrap(), "vue");
}

#[when("I parse the file response")]
fn when_parse(_world: &mut PenpotWorld) { /* result set in given */ }

// ── Then ───────────────────────────────────────────────────────────────────

#[then(expr = "the URL should be {string}")]
fn then_url(world: &mut PenpotWorld, expected: String) {
    assert_eq!(world.api_url, expected);
}

#[then("the host should not end with \"/\"")]
fn then_no_trailing_slash(world: &mut PenpotWorld) {
    assert!(!world.config.as_ref().unwrap().host.ends_with('/'));
}

#[then("validation should fail")]
fn then_validation_fail(world: &mut PenpotWorld) {
    assert!(world.validation_result.as_ref().unwrap().is_err());
}

#[then("validation should pass")]
fn then_validation_pass(world: &mut PenpotWorld) {
    assert!(world.validation_result.as_ref().unwrap().is_ok());
}

#[then(expr = "the curl command should contain {string}")]
fn then_curl_contains(world: &mut PenpotWorld, s: String) {
    assert!(world.curl_command.contains(s.as_str()), "curl missing: {s}");
}

#[then(expr = "the CSS should contain {string}")]
fn then_css_contains(world: &mut PenpotWorld, s: String) {
    assert!(world.css_output.contains(s.as_str()), "CSS missing: {s}\n{}", world.css_output);
}

#[then(expr = "the code should contain {string}")]
fn then_code_contains(world: &mut PenpotWorld, s: String) {
    assert!(world.react_code.contains(s.as_str()), "Code missing: {s}");
}

#[then("a parse error should be returned")]
fn then_parse_error(world: &mut PenpotWorld) {
    assert!(world.parse_result.as_ref().unwrap().is_err());
}

#[then(expr = "the file id should be {string}")]
fn then_file_id(world: &mut PenpotWorld, id: String) {
    let df = world.parse_result.as_ref().unwrap().as_ref().unwrap();
    assert_eq!(df.id, id);
}

#[then(expr = "the file name should be {string}")]
fn then_file_name(world: &mut PenpotWorld, name: String) {
    let df = world.parse_result.as_ref().unwrap().as_ref().unwrap();
    assert_eq!(df.name, name);
}

#[then(expr = "the provider should be {string}")]
fn then_provider(world: &mut PenpotWorld, provider: String) {
    let df = world.parse_result.as_ref().unwrap().as_ref().unwrap();
    let p = format!("{:?}", df.provider).to_lowercase();
    assert_eq!(p, provider, "Provider mismatch: got {p}");
}

fn main() {
    futures::executor::block_on(PenpotWorld::run("tests/features/penpot_integration.feature"));
}
