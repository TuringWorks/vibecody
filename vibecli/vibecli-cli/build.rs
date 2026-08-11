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
    // `skills_embedded.rs` bakes `skills/**` into the binary with
    // `include_dir!`. rustc's dep-info tracks the *contents* of the files
    // the macro expanded to, but not the directory listing — add or delete
    // a skill and nothing re-expands the macro. Emitting any `rerun-if-*`
    // instruction (the two above already do) also turns off cargo's default
    // "rescan the whole package" behaviour, so the directory has to be
    // named explicitly or a stale catalogue ships silently.
    println!("cargo:rerun-if-changed=skills");

    let feature_on = std::env::var("CARGO_FEATURE_VIBE_MISTRALRS").is_ok();
    let is_macos = std::env::var("CARGO_CFG_TARGET_OS").as_deref() == Ok("macos");

    if feature_on || is_macos {
        println!("cargo:rustc-cfg=mistralrs_enabled");
    }
}
