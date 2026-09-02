//! Build script.
//!
//! `tauri_build::build()` is the whole job; the check above it exists because
//! the one way to get this crate wrong produces no error at all.

fn main() {
    warn_when_the_frontend_will_not_be_embedded();
    reserve_a_main_thread_stack_windows_can_actually_start_on();
    tauri_build::build()
}

/// Raise the main thread's stack reserve on MSVC targets.
///
/// Windows takes the main thread's stack size from the PE header, and the
/// default reserve is 1 MB. `main` calls `vibe_coder_lib::run()` directly —
/// it has to, because the Tauri event loop must own the main thread, so the
/// usual escape of re-entering on a `thread::Builder::new().stack_size(..)`
/// thread is not available here. `run()` builds a ~100-field `AppState`
/// literal and a `generate_handler!` closure covering well over a thousand
/// commands, all as temporaries in one frame; at `opt-level = 0` nothing is
/// inlined away and that frame does not fit in 1 MB. The binary then dies at
/// startup with `STATUS_STACK_OVERFLOW` (0xc00000fd) before a window appears.
///
/// 8 MB matches the Unix default. It is reserved address space, not committed
/// memory, so the cost on 64-bit is nil and it applies to release too — a
/// future command list is no likelier to shrink than grow.
///
/// `/STACK:` is a link.exe flag, hence the MSVC gate; the GNU toolchain wants
/// `-Wl,--stack` and every other platform already starts with 8 MB.
fn reserve_a_main_thread_stack_windows_can_actually_start_on() {
    if std::env::var("CARGO_CFG_TARGET_ENV").as_deref() == Ok("msvc") {
        println!("cargo:rustc-link-arg-bins=/STACK:8388608");
    }
}

/// Say so when a release binary is being built that will look for a dev server.
///
/// The frontend is embedded only when the `tauri/custom-protocol` feature is on,
/// which `tauri build` passes and a bare `cargo build --release` does not.
/// Without it the window loads `http://localhost:1420`, nothing is listening there, and the
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
             will load http://localhost:1420 and open on a blank window. Build it with \
             `npm run tauri:build` (or `make build-vibecoder`), or add `--features custom-protocol`."
        );
    }
}
