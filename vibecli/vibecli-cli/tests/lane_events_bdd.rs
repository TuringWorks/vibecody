/*!
 * BDD tests for the workflow lane event system using Cucumber.
 * Run with: cargo test --test lane_events_bdd
 */
use cucumber::{given, then, when, World};
use vibecli_cli::lane_events::{
    deduplicate_superseded, LaneEventBuilder, LaneEventType, WorkflowLaneEvent,
};

#[derive(Debug, Default, World)]
pub struct LeWorld {
    events: Vec<WorkflowLaneEvent>,
    built_event: Option<WorkflowLaneEvent>,
    build_error: Option<String>,
    deduped_has_sha: Vec<String>,
}

#[given(expr = "a LaneEventBuilder with type {string} and lane {string}")]
fn builder_with_type(world: &mut LeWorld, type_str: String, lane: String) {
    let et = type_from(&type_str);
    match LaneEventBuilder::new().event_type(et).lane_id(&lane).build() {
        Ok(e) => world.built_event = Some(e),
        Err(e) => world.build_error = Some(e),
    }
}

#[given(expr = "a LaneEventBuilder with lane {string} but no event type")]
fn builder_no_type(world: &mut LeWorld, lane: String) {
    match LaneEventBuilder::new().lane_id(&lane).build() {
        Ok(e) => world.built_event = Some(e),
        Err(e) => world.build_error = Some(e),
    }
}

#[given(expr = "a CommitCreated event with sha {string}")]
fn commit_created(world: &mut LeWorld, sha: String) {
    let e = LaneEventBuilder::new()
        .event_type(LaneEventType::CommitCreated)
        .lane_id("l")
        .meta("sha", &sha)
        .build()
        .unwrap();
    world.events.push(e);
}

#[given(expr = "a CommitSuperseded event with sha {string}")]
fn commit_superseded(world: &mut LeWorld, sha: String) {
    let e = LaneEventBuilder::new()
        .event_type(LaneEventType::CommitSuperseded)
        .lane_id("l")
        .meta("sha", &sha)
        .build()
        .unwrap();
    world.events.push(e);
}

#[when("I build the event")]
fn build_event(_world: &mut LeWorld) {} // already built in Given

#[when("I try to build the event")]
fn try_build(_world: &mut LeWorld) {}

#[when("I deduplicate the events")]
fn dedup(world: &mut LeWorld) {
    let deduped = deduplicate_superseded(&world.events);
    world.deduped_has_sha = deduped
        .iter()
        .filter(|e| e.event_type == LaneEventType::CommitCreated)
        .filter_map(|e| e.metadata.get("sha").cloned())
        .collect();
}

#[then("it should have a non-empty id")]
fn check_id(world: &mut LeWorld) {
    assert!(!world.built_event.as_ref().unwrap().id.is_empty());
}

#[then(expr = "it should have event type {string}")]
fn check_type(world: &mut LeWorld, expected: String) {
    assert_eq!(
        world.built_event.as_ref().unwrap().event_type.to_string(),
        expected
    );
}

#[then("the build should fail")]
fn check_fail(world: &mut LeWorld) {
    assert!(world.build_error.is_some(), "expected build to fail");
}

#[then(expr = "CommitCreated {string} should be removed")]
fn check_removed(world: &mut LeWorld, sha: String) {
    assert!(
        !world.deduped_has_sha.contains(&sha),
        "sha {sha} should have been removed"
    );
}

#[then(expr = "CommitCreated {string} should remain")]
fn check_remains(world: &mut LeWorld, sha: String) {
    assert!(
        world.deduped_has_sha.contains(&sha),
        "sha {sha} should remain"
    );
}

#[then("each LaneEventType variant should have a unique display string")]
fn check_unique_displays(_world: &mut LeWorld) {
    use std::collections::HashSet;
    let displays: HashSet<String> = LaneEventType::all_variants()
        .iter()
        .map(|v| v.to_string())
        .collect();
    assert_eq!(displays.len(), LaneEventType::all_variants().len());
}

fn type_from(s: &str) -> LaneEventType {
    match s {
        "lane_started" => LaneEventType::LaneStarted,
        "lane_stopped" => LaneEventType::LaneStopped,
        "commit_created" => LaneEventType::CommitCreated,
        _ => LaneEventType::LaneStarted,
    }
}

fn main() {
    futures::executor::block_on(LeWorld::run("tests/features/lane_events.feature"));
}
