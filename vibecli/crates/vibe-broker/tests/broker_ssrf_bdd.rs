//! BDD coverage for the broker SSRF guard.

use cucumber::{World, given, then, when};
use vibe_broker::{SsrfGuard, SsrfVerdict};

#[derive(Default, World)]
pub struct SWorld {
    guard: Option<SsrfGuard>,
    verdict: Option<SsrfVerdict>,
}

impl std::fmt::Debug for SWorld {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("SWorld").finish()
    }
}

#[given("a fresh SSRF guard")]
fn fresh_guard(world: &mut SWorld) {
    world.guard = Some(SsrfGuard::new());
}

#[given("an SSRF guard with IMDS faker enabled")]
fn imds_guard(world: &mut SWorld) {
    world.guard = Some(SsrfGuard::new().with_imds_allowed());
}

#[when(expr = "I check {string}")]
fn check(world: &mut SWorld, url: String) {
    let g = world.guard.as_ref().unwrap();
    world.verdict = Some(g.check(&url));
}

#[then(expr = "the guard verdict is {string}")]
fn verdict_is(world: &mut SWorld, expected: String) {
    let want = match expected.as_str() {
        "Allow" => SsrfVerdict::Allow,
        "Block" => SsrfVerdict::Block,
        other => panic!("unknown verdict: {other}"),
    };
    assert_eq!(world.verdict.as_ref(), Some(&want));
}

fn main() {
    futures::executor::block_on(SWorld::run("tests/features/broker_ssrf.feature"));
}
