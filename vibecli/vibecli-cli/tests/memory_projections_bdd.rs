/*!
 * BDD tests for memory projections (Phase 6). Exercises the USER.md /
 * MEMORY.md write path end-to-end against a tempdir so no real user
 * config is touched.
 *
 * Run with: cargo test --test memory_projections_bdd
 */
use cucumber::{World, given, then, when};
use std::path::PathBuf;
use tempfile::TempDir;
use vibecli_cli::memory_projections::{ProjectionPaths, write_projections};
use vibecli_cli::open_memory::OpenMemoryStore;

#[derive(Default, World)]
pub struct ProjWorld {
    workspace: Option<TempDir>,
    home: Option<TempDir>,
    data_dir: Option<TempDir>,
    store: Option<OpenMemoryStore>,
    last_run_bytes: Vec<Vec<u8>>,
    last: Option<ProjectionPaths>,
}

impl std::fmt::Debug for ProjWorld {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("ProjWorld")
            .field(
                "workspace",
                &self.workspace.as_ref().map(|t| t.path().to_owned()),
            )
            .field(
                "home",
                &self.home.as_ref().map(|t| t.path().to_owned()),
            )
            .field(
                "data_dir",
                &self.data_dir.as_ref().map(|t| t.path().to_owned()),
            )
            .field("has_store", &self.store.is_some())
            .field("last_run_bytes_count", &self.last_run_bytes.len())
            .finish()
    }
}

impl ProjWorld {
    fn workspace_path(&self) -> PathBuf {
        self.workspace
            .as_ref()
            .expect("workspace not created")
            .path()
            .to_owned()
    }
    fn home_path(&self) -> PathBuf {
        self.home
            .as_ref()
            .expect("home not created")
            .path()
            .to_owned()
    }
}

// ── Given ───────────────────────────────────────────────────────────────────

#[given(regex = r"^a fresh workspace$")]
fn given_workspace(w: &mut ProjWorld) {
    w.workspace = Some(TempDir::new().expect("tempdir"));
}

#[given(regex = r"^a fresh home directory$")]
fn given_home(w: &mut ProjWorld) {
    w.home = Some(TempDir::new().expect("home tempdir"));
}

// ── When ────────────────────────────────────────────────────────────────────

#[when(regex = r"^projections are written with no home directory$")]
fn when_write_no_home(w: &mut ProjWorld) {
    let ws = w.workspace_path();
    let paths = write_projections(None, &ws).expect("write_projections");
    let bytes = std::fs::read(&paths.memory_md).expect("read memory.md");
    w.last_run_bytes.push(bytes);
    w.last = Some(paths);
}

#[when(regex = r"^projections are written with the home directory$")]
fn when_write_with_home(w: &mut ProjWorld) {
    let ws = w.workspace_path();
    let home = w.home_path();
    let paths =
        write_projections(Some(&home), &ws).expect("write_projections");
    let bytes = std::fs::read(&paths.memory_md).expect("read memory.md");
    w.last_run_bytes.push(bytes);
    w.last = Some(paths);
}

// ── Then ────────────────────────────────────────────────────────────────────

#[then(expr = "the file {string} exists in the workspace")]
fn then_file_exists_workspace(w: &mut ProjWorld, rel: String) {
    let path = w.workspace_path().join(&rel);
    assert!(path.exists(), "expected {path:?} to exist");
}

#[then(expr = "the file {string} exists in the home directory")]
fn then_file_exists_home(w: &mut ProjWorld, rel: String) {
    let path = w.home_path().join(&rel);
    assert!(path.exists(), "expected {path:?} to exist");
}

#[then(expr = "the file {string} starts with {string}")]
fn then_file_starts_with(w: &mut ProjWorld, rel: String, prefix: String) {
    // Prefer workspace; fall back to home if the file doesn't land there.
    let ws_path = w.workspace_path().join(&rel);
    let home_path = w.home.as_ref().map(|h| h.path().join(&rel));
    let path = if ws_path.exists() {
        ws_path
    } else if let Some(p) = home_path.filter(|p| p.exists()) {
        p
    } else {
        panic!("file {rel} not found in workspace or home");
    };
    let body = std::fs::read_to_string(&path).expect("read");
    assert!(
        body.starts_with(&prefix),
        "expected {path:?} to start with {prefix:?}; got: {:?}",
        body.lines().next().unwrap_or("")
    );
}

#[then(expr = "the file {string} contains {string}")]
fn then_file_contains(w: &mut ProjWorld, rel: String, needle: String) {
    let path = w.workspace_path().join(&rel);
    let body = std::fs::read_to_string(&path).expect("read");
    assert!(
        body.contains(&needle),
        "expected {path:?} to contain {needle:?}"
    );
}

#[then(regex = r"^the MEMORY\.md bytes match between the two runs$")]
fn then_bytes_match(w: &mut ProjWorld) {
    assert!(
        w.last_run_bytes.len() >= 2,
        "need at least two runs captured, got {}",
        w.last_run_bytes.len()
    );
    assert_eq!(
        w.last_run_bytes[0], w.last_run_bytes[1],
        "projection output diverged between runs"
    );
}

// ── Phase 7: auto-refresh on save() ─────────────────────────────────────────

#[given(regex = r"^a project-scoped open memory store$")]
fn given_store(w: &mut ProjWorld) {
    let dir = TempDir::new().expect("data_dir tempdir");
    let mut store = OpenMemoryStore::new(dir.path(), "bdd-user");
    store.set_project("bdd-project");
    w.data_dir = Some(dir);
    w.store = Some(store);
}

#[given(regex = r"^projection refresh is enabled with no home$")]
fn given_refresh_no_home(w: &mut ProjWorld) {
    let workspace = w.workspace_path();
    let store = w
        .store
        .as_mut()
        .expect("store not initialized — add a Given for store first");
    store.enable_projection_refresh(workspace, None);
}

#[given(regex = r"^projection refresh is enabled with the home directory$")]
fn given_refresh_with_home(w: &mut ProjWorld) {
    let workspace = w.workspace_path();
    let home = w.home_path();
    let store = w
        .store
        .as_mut()
        .expect("store not initialized — add a Given for store first");
    store.enable_projection_refresh(workspace, Some(home));
}

#[when(expr = "a memory {string} is added")]
fn when_memory_added(w: &mut ProjWorld, content: String) {
    let store = w.store.as_mut().expect("store not initialized");
    store.add(content);
}

#[when(regex = r"^the store is saved$")]
fn when_store_saved(w: &mut ProjWorld) {
    let store = w.store.as_ref().expect("store not initialized");
    store.save().expect("save");
}

#[then(expr = "the file {string} does not exist in the workspace")]
fn then_file_absent_workspace(w: &mut ProjWorld, rel: String) {
    let path = w.workspace_path().join(&rel);
    assert!(
        !path.exists(),
        "expected {path:?} to NOT exist, but it does"
    );
}

fn main() {
    futures::executor::block_on(ProjWorld::run(
        "tests/features/memory_projections.feature",
    ));
}
