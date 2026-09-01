//! Build script.
//!
//! `tauri_build::build()` is the whole job; the check above it exists because
//! the one way to get this crate wrong produces no error at all.

fn main() {
    warn_when_the_frontend_will_not_be_embedded();
    tauri_build::build()
}

/// Say so when a release binary is being built that will look for a dev server.
///
/// The frontend is embedded only when the `tauri/custom-protocol` feature is on,
/// which `tauri build` passes and a bare `cargo build --release` does not.
/// Without it the window loads `http://localhost:1421`, nothing is listening there, and the
/// app opens blank and silent — no panic, no log line, no failed request that
/// anyone sees. This warning is the only notice before that happens.
fn warn_when_the_frontend_will_not_be_embedded() {
    let release = std::env::var("PROFILE").as_deref() == Ok("release");
    // Set by tauri's own build script, so it follows the feature however it was
    // turned on: through this crate's `custom-protocol` or through
    // `--features tauri/custom-protocol` directly.
    let no_embed = std::env::var("DEP_TAURI_DEV").as_deref() == Ok("true");
    if release && no_embed {
        println!(
            "cargo:warning=this release build will not embed the frontend — it \
             will load http://localhost:1421 and open on a blank window. Build it with \
             `npm run tauri:build` (or `make build-vibeaichat`), or add `--features custom-protocol`."
        );
    }
}
