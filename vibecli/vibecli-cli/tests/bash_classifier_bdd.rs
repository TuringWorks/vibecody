/*!
 * BDD tests for bash_classifier using Cucumber.
 * Run with: cargo test --test bash_classifier_bdd
 */
use cucumber::{World, given, then, when};
use vibecli_cli::bash_classifier::{BashClassifier, ClassificationResult};

#[derive(Debug, Default, World)]
pub struct BcWorld {
    command: String,
    result: Option<ClassificationResult>,
}

#[given(expr = "the command {string}")]
fn set_command(world: &mut BcWorld, cmd: String) {
    world.command = cmd;
}

#[when("I classify it")]
fn classify(world: &mut BcWorld) {
    world.result = Some(BashClassifier::classify_semantic(&world.command));
}

#[then(expr = "the category should be {string}")]
fn check_category(world: &mut BcWorld, expected: String) {
    let r = world.result.as_ref().unwrap();
    assert_eq!(r.category.to_string(), expected, "command: {}", world.command);
}

#[then(expr = "the flags should include {string}")]
fn check_flag(world: &mut BcWorld, flag: String) {
    let r = world.result.as_ref().unwrap();
    assert!(r.flags.contains(&flag), "flags {:?} do not contain {:?}", r.flags, flag);
}

fn main() {
    futures::executor::block_on(BcWorld::run("tests/features/bash_classifier.feature"));
}
