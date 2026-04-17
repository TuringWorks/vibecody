/*!
 * BDD tests for alt_explore using Cucumber.
 * Run with: cargo test --test alt_explore_bdd
 */
use cucumber::{World, given, then, when};
use vibecli_cli::alt_explore::{ExploreCandidate, Tournament, TournamentConfig, TournamentResult};

#[derive(Debug, Default, World)]
#[allow(dead_code)]
pub struct AeWorld {
    candidates: Vec<ExploreCandidate>,
    scored: Option<ExploreCandidate>,
    ranked: Vec<ExploreCandidate>,
    result: Option<TournamentResult>,
    min_compile_required: bool,
}

impl AeWorld {
    fn tournament(&self) -> Tournament {
        Tournament::new(TournamentConfig {
            min_compile_required: self.min_compile_required,
            ..Default::default()
        })
    }
}

// ---------------------------------------------------------------------------
// Given
// ---------------------------------------------------------------------------

#[given(expr = "a candidate with pass_rate {float} diff_lines {int} and compile {word}")]
fn set_candidate(world: &mut AeWorld, pass_rate: f32, diff_lines: usize, compile: String) {
    let compile_success = compile == "true";
    world.scored = Some(ExploreCandidate::new(
        "test",
        "",
        pass_rate,
        diff_lines,
        compile_success,
    ));
}

#[given(expr = "two candidates where {string} has higher score than {string}")]
fn two_candidates_scored(world: &mut AeWorld, high: String, low: String) {
    let mut strong = ExploreCandidate::new(high, "", 1.0, 0, true);
    strong.score = 0.9;
    let mut weak = ExploreCandidate::new(low, "", 0.1, 500, false);
    weak.score = 0.1;
    world.candidates.push(weak);
    world.candidates.push(strong);
}

#[given(expr = "two candidates where {string} compiles and {string} does not")]
fn two_candidates_compile(world: &mut AeWorld, good: String, bad: String) {
    world.candidates.push(ExploreCandidate::new(good, "", 0.8, 20, true));
    world.candidates.push(ExploreCandidate::new(bad, "", 1.0, 0, false));
}

#[given("min_compile_required is true")]
fn set_min_compile(world: &mut AeWorld) {
    world.min_compile_required = true;
}

// ---------------------------------------------------------------------------
// When
// ---------------------------------------------------------------------------

#[when("I score the candidate")]
fn do_score(world: &mut AeWorld) {
    let t = world.tournament();
    if let Some(c) = world.scored.as_mut() {
        t.score(c);
    }
}

#[when("I rank the candidates")]
fn do_rank(world: &mut AeWorld) {
    let t = world.tournament();
    let candidates = world.candidates.drain(..).collect();
    world.ranked = t.rank(candidates);
}

#[when("I disqualify non-compiling candidates")]
fn do_disqualify(world: &mut AeWorld) {
    let t = world.tournament();
    let candidates = world.candidates.drain(..).collect();
    world.ranked = t.disqualify_non_compiling(candidates);
}

// ---------------------------------------------------------------------------
// Then
// ---------------------------------------------------------------------------

#[then(expr = "the score should be {float}")]
fn check_score(world: &mut AeWorld, expected: f32) {
    let c = world.scored.as_ref().unwrap();
    assert!(
        (c.score - expected).abs() < 1e-5,
        "score={} expected={}",
        c.score,
        expected
    );
}

#[then(expr = "the score should be less than or equal to {float}")]
fn check_score_lte(world: &mut AeWorld, limit: f32) {
    let c = world.scored.as_ref().unwrap();
    assert!(
        c.score <= limit + 1e-5,
        "score={} should be <= {}",
        c.score,
        limit
    );
}

#[then(expr = "the first candidate should be {string}")]
fn check_first_candidate(world: &mut AeWorld, expected: String) {
    assert!(
        !world.ranked.is_empty(),
        "ranked list is empty"
    );
    assert_eq!(world.ranked[0].id, expected);
}

#[then(expr = "only {string} should remain")]
fn check_only_remains(world: &mut AeWorld, expected: String) {
    assert_eq!(world.ranked.len(), 1, "expected exactly 1 candidate");
    assert_eq!(world.ranked[0].id, expected);
}

fn main() {
    futures::executor::block_on(AeWorld::run("tests/features/alt_explore.feature"));
}
