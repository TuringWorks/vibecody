//! BDD coverage for ProactiveScanner real filesystem I/O (US-006).

use cucumber::{World, gherkin::Step, given, then, when};
use std::path::PathBuf;
use std::time::Duration;
use tempfile::TempDir;
use tokio::time::timeout;
use vibecli_cli::proactive_agent::ScanCategory;
use vibecli_cli::proactive_scanner::{
    WatcherHandle, discover_files, scan_project, start_watcher,
};

#[derive(Default, World)]
pub struct ScannerWorld {
    project: Option<TempDir>,
    discovered: Vec<PathBuf>,
    suggestions_count: usize,
    watcher: Option<WatcherHandle>,
    matched_event: Option<PathBuf>,
}

impl std::fmt::Debug for ScannerWorld {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("ScannerWorld")
            .field("project", &self.project.as_ref().map(|d| d.path().to_owned()))
            .field("discovered", &self.discovered.len())
            .field("suggestions", &self.suggestions_count)
            .field("matched_event", &self.matched_event)
            .finish()
    }
}

fn write_file(root: &std::path::Path, rel: &str) {
    let p = root.join(rel);
    if let Some(parent) = p.parent() {
        std::fs::create_dir_all(parent).unwrap();
    }
    std::fs::write(p, b"// seed\n").unwrap();
}

// ── Given ───────────────────────────────────────────────────────────────────

#[given(regex = r#"^a temp project with files:$"#)]
fn given_project_files(w: &mut ScannerWorld, step: &Step) {
    let dir = tempfile::tempdir().unwrap();
    if let Some(table) = step.table.as_ref() {
        for row in &table.rows {
            // Each row has a single cell with the relative path.
            let rel = row[0].trim();
            write_file(dir.path(), rel);
        }
    }
    w.project = Some(dir);
}

// ── When ────────────────────────────────────────────────────────────────────

#[when(regex = r#"^the scanner discovers files under the project root$"#)]
fn when_discover(w: &mut ScannerWorld) {
    let p = w.project.as_ref().unwrap().path().to_path_buf();
    w.discovered = discover_files(&p);
}

#[when(regex = r#"^the scanner scans the project for categories "([^"]+)"$"#)]
fn when_scan(w: &mut ScannerWorld, cats: String) {
    let enabled: Vec<ScanCategory> = cats
        .split(',')
        .filter_map(|s| match s.trim() {
            "Performance" => Some(ScanCategory::Performance),
            "Security" => Some(ScanCategory::Security),
            "TechDebt" => Some(ScanCategory::TechDebt),
            "Correctness" => Some(ScanCategory::Correctness),
            "Accessibility" => Some(ScanCategory::Accessibility),
            "Documentation" => Some(ScanCategory::Documentation),
            _ => None,
        })
        .collect();
    let p = w.project.as_ref().unwrap().path();
    let sugs = scan_project(p, &enabled);
    w.suggestions_count = sugs.len();
}

#[when(regex = r#"^a watcher is started on the project root$"#)]
fn when_start_watcher(w: &mut ScannerWorld) {
    let p = w.project.as_ref().unwrap().path();
    let handle = start_watcher(p).expect("start watcher");
    w.watcher = Some(handle);
    // Give the watcher a beat to install before the next step writes.
    std::thread::sleep(Duration::from_millis(50));
}

#[when(regex = r#"^a new file "([^"]+)" is written$"#)]
fn when_write_new_file(w: &mut ScannerWorld, rel: String) {
    let p = w.project.as_ref().unwrap().path();
    write_file(p, &rel);
}

// ── Then ────────────────────────────────────────────────────────────────────

#[then(regex = r#"^the discovered count is (\d+)$"#)]
fn then_discovered_count(w: &mut ScannerWorld, n: usize) {
    assert_eq!(w.discovered.len(), n, "discovered: {:?}", w.discovered);
}

#[then(regex = r#"^the discovered set contains path "([^"]+)"$"#)]
fn then_discovered_has(w: &mut ScannerWorld, rel: String) {
    assert!(
        w.discovered.iter().any(|p| p.ends_with(&rel)),
        "discovered {:?} missing {rel}",
        w.discovered
    );
}

#[then(regex = r#"^the scan produces at least (\d+) suggestions$"#)]
fn then_suggestions_at_least(w: &mut ScannerWorld, n: usize) {
    assert!(
        w.suggestions_count >= n,
        "got {} suggestions, expected at least {n}",
        w.suggestions_count
    );
}

#[then(regex = r#"^the watcher reports an event for "([^"]+)" within (\d+) seconds$"#)]
async fn then_watcher_fires(w: &mut ScannerWorld, needle: String, secs: u64) {
    let handle = w.watcher.as_mut().expect("watcher");
    let deadline = Duration::from_secs(secs);
    let result = timeout(deadline, async {
        loop {
            match handle.events.recv().await {
                Some(path) => {
                    if path.to_string_lossy().contains(&needle) {
                        return Some(path);
                    }
                }
                None => return None,
            }
        }
    })
    .await;
    match result {
        Ok(Some(p)) => w.matched_event = Some(p),
        Ok(None) => panic!("watcher channel closed before event"),
        Err(_) => panic!("no watcher event matching {needle} within {secs}s"),
    }
}

#[tokio::main(flavor = "multi_thread", worker_threads = 2)]
async fn main() {
    ScannerWorld::run("tests/features/proactive_scanner.feature").await;
}
