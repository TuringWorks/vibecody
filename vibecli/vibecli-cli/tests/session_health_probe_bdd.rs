/*!
 * BDD tests for the post-compaction session health probe.
 * Run with: cargo test --test session_health_probe_bdd
 */
use cucumber::{World, given, then, when};
use vibecli_cli::session_health_probe::{
    PostCompactionProbe, ProbeConfig, ProbeResult,
    ResponsiveMockChecker, UnresponsiveMockChecker,
};

#[derive(Debug, Default, World)]
pub struct ShpWorld {
    probe: Option<PostCompactionProbe>,
    result: Option<ProbeResult>,
    compacted_count: usize,
    should_probe: bool,
}

#[given("a responsive tool executor")]
fn responsive_executor(world: &mut ShpWorld) {
    world.probe = Some(PostCompactionProbe::new(ProbeConfig::default()));
    let checker = ResponsiveMockChecker;
    world.result = Some(world.probe.as_ref().unwrap().run(&checker));
}

#[given("an unresponsive tool executor")]
fn unresponsive_executor(world: &mut ShpWorld) {
    world.probe = Some(PostCompactionProbe::new(ProbeConfig::default()));
    let checker = UnresponsiveMockChecker { reason: "timeout".into() };
    world.result = Some(world.probe.as_ref().unwrap().run(&checker));
}

#[given(expr = "a probe with compaction threshold {int}")]
fn probe_with_threshold(world: &mut ShpWorld, threshold: usize) {
    world.probe = Some(PostCompactionProbe::new(ProbeConfig {
        compaction_threshold: threshold,
        ..Default::default()
    }));
}

#[when("I run the health probe")]
fn run_probe(_world: &mut ShpWorld) {
    // Result already set in the Given steps for this feature.
}

#[when(expr = "{int} messages were compacted")]
fn set_compacted(world: &mut ShpWorld, n: usize) {
    world.compacted_count = n;
    world.should_probe = world
        .probe
        .as_ref()
        .map(|p| p.should_probe_after_compaction(n))
        .unwrap_or(false);
}

#[then("the result should be Healthy")]
fn check_healthy(world: &mut ShpWorld) {
    assert_eq!(world.result.as_ref().unwrap(), &ProbeResult::Healthy);
}

#[then("the result should be Failed")]
fn check_failed(world: &mut ShpWorld) {
    assert!(matches!(world.result.as_ref().unwrap(), ProbeResult::Failed(_)));
}

#[then(expr = "the mapped health state should be {string}")]
fn check_health_state(world: &mut ShpWorld, expected: String) {
    let state = world.result.as_ref().unwrap().to_health_string();
    assert_eq!(state, expected.as_str());
}

#[then("should_probe_after_compaction should return true")]
fn check_probe_true(world: &mut ShpWorld) {
    assert!(world.should_probe);
}

#[then("should_probe_after_compaction should return false")]
fn check_probe_false(world: &mut ShpWorld) {
    assert!(!world.should_probe);
}

fn main() {
    futures::executor::block_on(
        ShpWorld::run("tests/features/session_health_probe.feature"),
    );
}
