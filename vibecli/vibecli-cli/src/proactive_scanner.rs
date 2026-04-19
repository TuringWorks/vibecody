//! Real filesystem scanner and watcher for the proactive-agent pipeline
//! (US-006).
//!
//! The existing [`proactive_agent`] module holds the in-memory state
//! (pending suggestions, learning store, digest logic, metrics) plus the
//! [`SuggestionGenerator`] decision table. This module supplies the real I/O
//! layer that module was missing:
//!
//! - [`discover_files`] walks a project tree with `walkdir` and applies a
//!   sensible default ignore list (`.git`, `target`, `node_modules`, `dist`,
//!   `build`).
//! - [`categorize_by_ext`] maps a filename to the set of [`ScanCategory`]s
//!   that are worth running against it.
//! - [`scan_project`] ties the two together: discover, categorize, feed each
//!   file into `SuggestionGenerator`, return the suggestions.
//! - [`start_watcher`] opens a [`notify`] recommended watcher and forwards
//!   create/write events onto a `tokio::sync::mpsc::Receiver` so async code
//!   can await filesystem changes without blocking the reactor.
//!
//! The stub in `proactive_agent.rs` keeps its pure business logic
//! (suggestion state machine, digests, learning). Callers that want real
//! "background scan" behaviour construct a scanner from this module and
//! pass its results into [`ProactiveAgent::scan`].

use crate::proactive_agent::{ScanCategory, SuggestionGenerator, Suggestion};
use notify::{Event, EventKind, RecommendedWatcher, RecursiveMode, Watcher};
use std::path::{Path, PathBuf};
use tokio::sync::mpsc;
use walkdir::WalkDir;

// ── Defaults ────────────────────────────────────────────────────────────────

const DEFAULT_IGNORES: &[&str] = &[
    ".git",
    "target",
    "node_modules",
    "dist",
    "build",
    ".next",
    ".venv",
    "__pycache__",
];

// ── Discovery ───────────────────────────────────────────────────────────────

/// Recursively walk `root`, skipping any directory whose name appears in
/// [`DEFAULT_IGNORES`], and return absolute paths to every regular file.
pub fn discover_files(root: &Path) -> Vec<PathBuf> {
    discover_files_with_ignores(root, DEFAULT_IGNORES)
}

/// Like [`discover_files`] but with a caller-supplied ignore list.
pub fn discover_files_with_ignores(root: &Path, ignores: &[&str]) -> Vec<PathBuf> {
    let mut out = Vec::new();
    let walker = WalkDir::new(root)
        .follow_links(false)
        .into_iter()
        .filter_entry(|e| {
            if e.depth() == 0 {
                return true;
            }
            let name = e.file_name().to_string_lossy();
            !ignores.iter().any(|ig| name == *ig)
        });
    for entry in walker.flatten() {
        if entry.file_type().is_file() {
            out.push(entry.into_path());
        }
    }
    out
}

// ── Categorization ──────────────────────────────────────────────────────────

/// Map a filename to the scan categories that make sense for it, based on
/// the file extension. Files with no useful extension return an empty slice.
pub fn categorize_by_ext(path: &Path) -> Vec<ScanCategory> {
    let Some(ext) = path.extension().and_then(|e| e.to_str()) else {
        return Vec::new();
    };
    match ext {
        "rs" | "py" | "go" | "java" | "kt" | "swift" | "c" | "cpp" | "h" | "hpp" => {
            vec![
                ScanCategory::Performance,
                ScanCategory::Security,
                ScanCategory::TechDebt,
                ScanCategory::Correctness,
            ]
        }
        "js" | "ts" | "jsx" | "tsx" => vec![
            ScanCategory::Performance,
            ScanCategory::Security,
            ScanCategory::Accessibility,
            ScanCategory::TechDebt,
        ],
        "html" | "htm" => vec![ScanCategory::Accessibility, ScanCategory::Security],
        "css" | "scss" | "sass" => vec![ScanCategory::Accessibility],
        "yml" | "yaml" | "toml" | "json" => vec![ScanCategory::Security],
        _ => Vec::new(),
    }
}

// ── Scanning ────────────────────────────────────────────────────────────────

/// Walk the project tree under `root`, categorize each file, and feed the
/// (category, path) pairs into [`SuggestionGenerator::generate_for_category`].
/// Returns every suggestion whose category is in the caller-supplied set.
pub fn scan_project(root: &Path, enabled: &[ScanCategory]) -> Vec<Suggestion> {
    let files = discover_files(root);
    let mut out = Vec::new();
    for file in files {
        let path_str = file.to_string_lossy().into_owned();
        let cats = categorize_by_ext(&file);
        for cat in cats {
            if !enabled.contains(&cat) {
                continue;
            }
            if let Some(sug) = SuggestionGenerator::generate_for_category(&cat, &path_str) {
                out.push(sug);
            }
        }
    }
    out
}

// ── Watcher ─────────────────────────────────────────────────────────────────

/// Handle returned from [`start_watcher`]. Dropping it stops the watcher;
/// keep it alive for as long as you want change events.
pub struct WatcherHandle {
    _watcher: RecommendedWatcher,
    pub events: mpsc::Receiver<PathBuf>,
}

/// Start a recursive [`notify`] watcher on `root` and return a receiver that
/// yields the path of every file that was created, modified, or renamed
/// under the tree. Synthetic events from the watcher backend are coalesced
/// to "a file at this path changed" so callers don't need to inspect
/// [`EventKind`] themselves.
pub fn start_watcher(root: &Path) -> Result<WatcherHandle, String> {
    let (tx, rx) = mpsc::channel::<PathBuf>(256);
    let handler = move |res: notify::Result<Event>| {
        if let Ok(event) = res {
            let should_forward = matches!(
                event.kind,
                EventKind::Create(_) | EventKind::Modify(_) | EventKind::Remove(_)
            );
            if !should_forward {
                return;
            }
            for path in event.paths {
                let _ = tx.blocking_send(path);
            }
        }
    };
    let mut watcher =
        notify::recommended_watcher(handler).map_err(|e| format!("watcher init: {e}"))?;
    watcher
        .watch(root, RecursiveMode::Recursive)
        .map_err(|e| format!("watch {root:?}: {e}"))?;
    Ok(WatcherHandle {
        _watcher: watcher,
        events: rx,
    })
}

// ── Tests ───────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    fn write_in(root: &Path, rel: &str) {
        let p = root.join(rel);
        if let Some(parent) = p.parent() {
            std::fs::create_dir_all(parent).unwrap();
        }
        std::fs::write(p, b"// test\n").unwrap();
    }

    #[test]
    fn discover_walks_tree_and_ignores_target() {
        let dir = TempDir::new().unwrap();
        write_in(dir.path(), "src/main.rs");
        write_in(dir.path(), "src/lib.rs");
        write_in(dir.path(), "target/debug/x.rs");
        write_in(dir.path(), ".git/config");
        let files = discover_files(dir.path());
        assert_eq!(files.len(), 2, "got {files:?}");
        assert!(files.iter().any(|p| p.ends_with("main.rs")));
        assert!(files.iter().any(|p| p.ends_with("lib.rs")));
    }

    #[test]
    fn categorize_handles_ts_and_rs() {
        let rs = categorize_by_ext(Path::new("src/main.rs"));
        assert!(rs.contains(&ScanCategory::Performance));
        assert!(rs.contains(&ScanCategory::Security));
        let tsx = categorize_by_ext(Path::new("web/app.tsx"));
        assert!(tsx.contains(&ScanCategory::Accessibility));
        let md = categorize_by_ext(Path::new("README.md"));
        assert!(md.is_empty());
    }

    #[test]
    fn scan_project_emits_suggestions() {
        let dir = TempDir::new().unwrap();
        write_in(dir.path(), "src/main.rs");
        write_in(dir.path(), "web/app.tsx");
        let out = scan_project(
            dir.path(),
            &[ScanCategory::Performance, ScanCategory::Security],
        );
        assert!(
            out.len() >= 2,
            "expected suggestions for both files, got {out:?}"
        );
    }
}
