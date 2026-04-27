//! Cross-platform BDD tests for the native() constructor.

use cucumber::{World, then, when};
use vibe_sandbox::Sandbox;

#[derive(Default, World)]
pub struct NWorld {
    sandbox: Option<Box<dyn Sandbox>>,
}

impl std::fmt::Debug for NWorld {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("NWorld")
            .field("has_sandbox", &self.sandbox.is_some())
            .finish()
    }
}

#[when("I call vibe_sandbox_native::native")]
fn call_native(world: &mut NWorld) {
    world.sandbox = Some(vibe_sandbox_native::native().expect("native() succeeds"));
}

#[then(expr = "I get a sandbox whose tier is {string}")]
fn tier_is(world: &mut NWorld, expected: String) {
    let actual = world.sandbox.as_ref().unwrap().tier();
    assert_eq!(actual.to_string(), expected);
}

fn main() {
    futures::executor::block_on(NWorld::run("tests/features/native_tier.feature"));
}
