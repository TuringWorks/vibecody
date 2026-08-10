//! The Dockerfile duplicates the workspace member list; this keeps them in sync.
//!
//! `Dockerfile` copies each member's `Cargo.toml` individually so the expensive
//! dependency build lands in a cached layer. Cargo refuses to resolve a
//! workspace whose declared member is missing, so **every** member added to
//! `[workspace] members` must also be added in three places in the Dockerfile:
//! the manifest `COPY`, the stub-source `mkdir`, and the real-source `COPY`.
//!
//! That hand-maintained list has now drifted twice. It was fixed for v0.5.6
//! (issue #32, seven members missing) and had drifted again by v0.5.8 — fifteen
//! members, including every `fluxo/*` crate, VibeDesk, and the shared
//! `crates/*` libraries. Both times the symptom was the same: the Docker
//! release job fails and nothing else does, because it is the only job that
//! builds the workspace through the Dockerfile.
//!
//! The drift is invisible locally unless you have a Docker daemon and think to
//! run a full image build, which is why it reached a release twice. A string
//! comparison catches it in the ordinary test run instead.
//!
//! If this test fails, add the member to all three Dockerfile sections. It
//! deliberately does not auto-fix: the stub for a binary crate needs
//! `fn main() {}` while a library needs an empty `lib.rs`, and only the crate
//! layout says which.

use std::path::{Path, PathBuf};

fn repo_root() -> PathBuf {
    // CARGO_MANIFEST_DIR is <root>/vibecli/vibecli-cli
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .ancestors()
        .nth(2)
        .expect("repo root is two levels above the crate manifest")
        .to_path_buf()
}

/// Members declared in the root `[workspace]` table.
fn workspace_members(root: &Path) -> Vec<String> {
    let manifest = std::fs::read_to_string(root.join("Cargo.toml")).expect("read root Cargo.toml");
    let body = manifest
        .split_once("members = [")
        .expect("root manifest declares [workspace] members")
        .1
        .split_once(']')
        .expect("members list is closed")
        .0;
    body.lines()
        .filter_map(|l| {
            let l = l.trim();
            l.starts_with('"')
                .then(|| l.trim_matches(|c| c == '"' || c == ',').to_string())
        })
        .filter(|l| !l.is_empty())
        .collect()
}

fn dockerfile(root: &Path) -> String {
    std::fs::read_to_string(root.join("Dockerfile")).expect("read Dockerfile")
}

#[test]
fn every_workspace_member_has_a_manifest_copy() {
    let root = repo_root();
    let docker = dockerfile(&root);
    let missing: Vec<_> = workspace_members(&root)
        .into_iter()
        .filter(|m| !docker.contains(&format!("COPY {m}/Cargo.toml")))
        .collect();

    assert!(
        missing.is_empty(),
        "these workspace members have no manifest COPY in the Dockerfile, so \
         `cargo` cannot resolve the workspace and the Docker release job fails:\n  {}\n\
         Add `COPY <member>/Cargo.toml <member>/Cargo.toml` alongside the others.",
        missing.join("\n  ")
    );
}

#[test]
fn every_workspace_member_has_a_stub_source() {
    let root = repo_root();
    let docker = dockerfile(&root);
    // The stub layer must create *something* under each member's src/, or the
    // pre-build step fails on a manifest pointing at a non-existent target.
    let missing: Vec<_> = workspace_members(&root)
        .into_iter()
        .filter(|m| !docker.contains(&format!("mkdir -p {m}/src")))
        .collect();

    assert!(
        missing.is_empty(),
        "these workspace members have no stub source in the Dockerfile's \
         dependency-cache layer:\n  {}\n\
         Add `mkdir -p <member>/src && echo '' > <member>/src/lib.rs` (or \
         `echo 'fn main() {{}}' > <member>/src/main.rs` for a binary).",
        missing.join("\n  ")
    );
}

#[test]
fn every_workspace_member_has_its_real_source_copied() {
    let root = repo_root();
    let docker = dockerfile(&root);
    // Without this the build links against the empty stub — which compiles,
    // and then fails at the first `use` of a symbol that only exists in the
    // real crate. That was the v0.5.6 `vibe-memory` failure.
    let missing: Vec<_> = workspace_members(&root)
        .into_iter()
        .filter(|m| {
            let direct = docker.contains(&format!("COPY {m}/src/"));
            // A parent-directory copy covers its children (e.g. `COPY vibecli/`
            // covers every vibecli/crates/* member).
            let via_parent = Path::new(m)
                .ancestors()
                .skip(1)
                .filter_map(|p| p.to_str())
                .filter(|p| !p.is_empty())
                .any(|p| docker.contains(&format!("COPY {p}/ {p}/")));
            !(direct || via_parent)
        })
        .collect();

    assert!(
        missing.is_empty(),
        "these workspace members are stubbed but never have their real source \
         copied over the stub:\n  {}\n\
         The build links against an empty crate and fails at the first use of \
         a symbol it should export.",
        missing.join("\n  ")
    );
}

/// Replay the Dockerfile's cache layer and confirm cargo can resolve the
/// workspace from it — the exact operation that failed the v0.5.8 release.
///
/// Needs no Docker daemon, which is the point: both previous occurrences
/// reached a release because verifying them appeared to require one.
///
/// **What this catches:** a member whose manifest is never copied (the v0.5.8
/// failure), and a manifest whose *explicitly declared* `[lib] path` or
/// `[[bin]] path` has no stub behind it — `vibecoder/src-tauri` declares
/// `vibe_coder_lib`, so a missing `src/lib.rs` there is a hard parse error.
///
/// **What it does not catch:** a wrongly-shaped stub for an *auto-discovered*
/// target. Writing `src/main.rs` where a crate wants `src/lib.rs` leaves cargo
/// resolving happily — it simply concludes the crate has no library — and the
/// failure only appears later, when a dependent tries to link it. Catching
/// that needs `cargo check`, which is far too slow for a unit test. Verified
/// by experiment, not assumed: swapping vibe-embed's stub to `main.rs` keeps
/// all four tests green.
#[test]
fn dockerfile_stub_layer_resolves_the_workspace() {
    let root = repo_root();
    let docker = dockerfile(&root);
    let tmp = tempfile::tempdir().expect("tempdir");
    let dst = tmp.path();

    std::fs::copy(root.join("Cargo.toml"), dst.join("Cargo.toml")).expect("copy root manifest");

    // `COPY <member>/Cargo.toml <member>/Cargo.toml`
    let mut copied = 0usize;
    for line in docker.lines().filter(|l| l.starts_with("COPY ")) {
        let Some(src) = line.split_whitespace().nth(1) else {
            continue;
        };
        if !src.ends_with("/Cargo.toml") {
            continue;
        }
        let out = dst.join(src);
        std::fs::create_dir_all(out.parent().expect("has parent")).expect("mkdir");
        std::fs::copy(root.join(src), &out).expect("copy member manifest");
        copied += 1;
    }
    assert!(
        copied > 10,
        "parsed too few manifest COPYs ({copied}) — did the Dockerfile format change?"
    );

    // `mkdir -p <member>/src && echo '…' > <member>/src/<file>`
    //
    // Split on "/src && ", not "/src": `vibecoder/src-tauri` contains "/src",
    // so the loose match truncated the member to `vibecoder` and silently
    // stubbed the wrong directory. This test caught that in its own parser,
    // which is a fair demonstration of why the string-only assertions above
    // are not enough on their own.
    let mut stubbed = 0usize;
    for chunk in docker.split("mkdir -p ").skip(1) {
        let Some((member, rest)) = chunk.split_once("/src && ") else {
            continue;
        };
        let Some(path) = rest
            .split_once("> ")
            .and_then(|(_, p)| p.split_whitespace().next())
        else {
            continue;
        };
        let out = dst.join(path);
        std::fs::create_dir_all(out.parent().expect("has parent")).expect("mkdir");
        let body = if path.ends_with("main.rs") {
            "fn main() {}\n"
        } else {
            "\n"
        };
        std::fs::write(&out, body).expect("write stub");
        debug_assert!(
            path.starts_with(member),
            "stub path must sit under its member"
        );
        stubbed += 1;
    }
    assert!(
        stubbed > 10,
        "parsed too few stub sources ({stubbed}) — did the Dockerfile format change?"
    );

    let output =
        std::process::Command::new(std::env::var("CARGO").unwrap_or_else(|_| "cargo".into()))
            .args([
                "metadata",
                "--no-deps",
                "--format-version",
                "1",
                "--offline",
            ])
            .current_dir(dst)
            .output()
            .expect("run cargo metadata");

    assert!(
        output.status.success(),
        "the Dockerfile's dependency-cache layer does not resolve — the Docker \
         release job will fail before it compiles anything:\n{}",
        String::from_utf8_lossy(&output.stderr)
    );
}
