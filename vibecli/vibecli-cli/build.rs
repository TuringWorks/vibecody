//! Build script: emits `cfg(mistralrs_enabled)` whenever the in-process
//! mistral.rs backend is active.
//!
//! Cargo features can't be made conditional on `target_os`, so we compute
//! the union here and expose a single canonical cfg flag the source code
//! gates on. The flag is on when either:
//!
//!   - the user opted in with `--features vibe-mistralrs`, or
//!   - we're building for macOS, where Metal acceleration is on by default
//!     and the in-process backend is the expected configuration.
//!
//! The matching Cargo target-specific dep block in `Cargo.toml` adds
//! `vibe-infer/mistralrs` + `vibe-infer/mistralrs-metal` on macOS so the
//! underlying crate features line up with this cfg.

fn main() {
    println!("cargo:rustc-check-cfg=cfg(mistralrs_enabled)");
    println!("cargo:rerun-if-env-changed=CARGO_FEATURE_VIBE_MISTRALRS");
    println!("cargo:rerun-if-env-changed=CARGO_CFG_TARGET_OS");

    let feature_on = std::env::var("CARGO_FEATURE_VIBE_MISTRALRS").is_ok();
    let is_macos = std::env::var("CARGO_CFG_TARGET_OS").as_deref() == Ok("macos");

    if feature_on || is_macos {
        println!("cargo:rustc-cfg=mistralrs_enabled");
    }
}
