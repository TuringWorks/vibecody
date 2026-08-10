//! Every Watch App Swift file must be in the Xcode target's Sources phase.
//!
//! Adding a `.swift` file to `vibewatch/VibeCodyWatch Watch App/` does not make
//! Xcode compile it. The file needs a `PBXFileReference`, a `PBXBuildFile`, a
//! group entry, and — the one that actually matters — membership in the
//! `PBXSourcesBuildPhase`. An editor that writes the file straight to disk, or
//! a merge that drops the project-file hunk, leaves a source file that exists,
//! is referenced from `ContentView`, and never compiles.
//!
//! This has now shipped twice:
//!
//! - **v0.5.7 (issue #30)** — `GoalsView`, `JobPickerView`, `RecapView` and
//!   `TaintedConfirmationView`, four files, four `cannot find … in scope`
//!   errors, watchOS release job exit 65.
//! - **v0.5.8** — `SkillforgeView`, same error, same job.
//!
//! Both reached a release tag because the watchOS build runs only in the
//! release workflow: `ci.yml` builds neither vibewatch nor the JetBrains
//! plugin, so nothing exercises the project file until a tag is pushed. A
//! string check costs nothing and runs in the ordinary test suite.
//!
//! Scoped to the Watch App target deliberately. `VibeCodyWatchComplication`'s
//! sources are not members of this project and would produce false failures.

use std::path::{Path, PathBuf};

fn repo_root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .ancestors()
        .nth(2)
        .expect("repo root is two levels above the crate manifest")
        .to_path_buf()
}

#[test]
fn every_watch_app_swift_file_is_compiled() {
    let root = repo_root();
    let sources = root.join("vibewatch/VibeCodyWatch Watch App");
    let project = root.join("vibewatch/VibeCodyWatch.xcodeproj/project.pbxproj");

    // A checkout without the watch tree (or a future reorganisation) should not
    // fail this test — it should stop applying. A silent skip is only
    // acceptable because the paths are asserted to exist together.
    if !sources.is_dir() || !project.is_file() {
        return;
    }

    let pbxproj = std::fs::read_to_string(&project).expect("read project.pbxproj");

    let mut swift: Vec<String> = std::fs::read_dir(&sources)
        .expect("read Watch App sources")
        .filter_map(|e| e.ok())
        .map(|e| e.file_name().to_string_lossy().into_owned())
        .filter(|n| n.ends_with(".swift"))
        .collect();
    swift.sort();

    assert!(
        swift.len() > 5,
        "found only {} Swift files — did the Watch App move?",
        swift.len()
    );

    // `<name> in Sources` is the PBXBuildFile comment for the compile phase.
    // Checking for the bare filename would pass on a file that is merely
    // *referenced* by the project but never built — which is exactly the state
    // that produced both failures.
    let uncompiled: Vec<&String> = swift
        .iter()
        .filter(|n| !pbxproj.contains(&format!("{n} in Sources")))
        .collect();

    assert!(
        uncompiled.is_empty(),
        "these Watch App sources are not in the Xcode Sources build phase, so \
         they will not compile and any reference to them fails with \
         `cannot find … in scope` — in the release workflow, which is the only \
         place watchOS is built:\n  {}\n\
         Add a PBXFileReference, a PBXBuildFile, a group entry, and a \
         PBXSourcesBuildPhase entry for each (see GoalsView.swift for the shape).",
        uncompiled
            .iter()
            .map(|s| s.as_str())
            .collect::<Vec<_>>()
            .join("\n  ")
    );
}
