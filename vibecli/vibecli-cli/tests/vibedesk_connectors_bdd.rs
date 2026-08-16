/*!
 * BDD tests for the VibeDesk Connectors panel.
 * Run with: cargo test --test vibedesk_connectors_bdd
 *
 * The panel shows rows; the agent runs servers. These steps drive the layer
 * between them — `connectors::{add_from_catalog, set_enabled, remove}` and,
 * crucially, `resolve_mcp_configs`, which is what the agent is actually handed.
 * The interesting failure is not "the add errored" but "the add succeeded and
 * the agent never heard about it".
 *
 * What this suite deliberately does not do is launch the servers. That needs
 * npx, uvx and the network, so it lives in `connector_catalog_bdd` behind
 * `#[ignore]`. A suite that cannot run without a package registry is not the
 * one that should run everywhere.
 */
use std::collections::HashMap;
use std::path::{Path, PathBuf};

use cucumber::{given, then, when, World};

use vibecli_cli::connectors::{self, CATALOG, WORKSPACE_PLACEHOLDER};
use vibecli_cli::workspace_store::WorkspaceStore;

// No `Debug` in the derive: `WorkspaceStore` holds the workspace encryption
// key, and the hand-written impl below reports it by presence only.
#[derive(Default, World)]
pub struct ConnectorWorld {
    workspace: Option<tempfile::TempDir>,
    store: Option<WorkspaceStore>,
    /// Outcome per attempted add, in order, so a step can assert on the second.
    adds: Vec<Result<connectors::Connector, String>>,
    removal: Option<(bool, usize)>,
}

impl std::fmt::Debug for ConnectorWorld {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("ConnectorWorld")
            .field("workspace", &self.workspace.as_ref().map(|d| d.path()))
            .field("store", &self.store.as_ref().map(|_| "<open>"))
            .field("adds", &self.adds.len())
            .field("removal", &self.removal)
            .finish()
    }
}

impl ConnectorWorld {
    fn workspace(&self) -> &Path {
        self.workspace
            .as_ref()
            .expect("a `Given a fresh workspace` step must run first")
            .path()
    }

    fn store(&self) -> &WorkspaceStore {
        self.store
            .as_ref()
            .expect("a `Given a fresh workspace` step must run first")
    }

    fn resolved(&self) -> Vec<vibe_ai::mcp::McpServerConfig> {
        connectors::resolve_mcp_configs(self.workspace(), self.store())
            .expect("resolving the agent's servers should succeed")
    }
}

#[given("a fresh workspace")]
fn fresh_workspace(world: &mut ConnectorWorld) {
    let dir = tempfile::tempdir().expect("tempdir");
    // `open_with` rather than `open`: the latter derives a key from the real
    // machine and would touch the developer's own store.
    let store = WorkspaceStore::open_with(&dir.path().join("workspace.db"), [42u8; 32])
        .expect("workspace store should open");
    world.workspace = Some(dir);
    world.store = Some(store);
    world.adds.clear();
    world.removal = None;
}

#[when(regex = r#"^I add the "([^"]+)" connector with no credentials$"#)]
fn add_without_credentials(world: &mut ConnectorWorld, id: String) {
    let outcome = connectors::add_from_catalog(world.store(), &id, &HashMap::new(), 0);
    world.adds.push(outcome);
}

#[when(regex = r#"^I add the "([^"]+)" connector with credential "([^"]+)" set to "([^"]+)"$"#)]
fn add_with_credential(world: &mut ConnectorWorld, id: String, env: String, value: String) {
    let creds = HashMap::from([(env, value)]);
    let outcome = connectors::add_from_catalog(world.store(), &id, &creds, 0);
    world.adds.push(outcome);
}

#[when("I add every credential-free catalog connector")]
fn add_every_credential_free(world: &mut ConnectorWorld) {
    for spec in CATALOG.iter().filter(|s| s.credentials.is_empty()) {
        let outcome = connectors::add_from_catalog(world.store(), spec.id, &HashMap::new(), 0);
        world.adds.push(outcome);
    }
}

#[when(regex = r#"^I disable the "([^"]+)" connector$"#)]
fn disable(world: &mut ConnectorWorld, id: String) {
    connectors::set_enabled(world.store(), &id, false).expect("disable should succeed");
}

#[when(regex = r#"^I enable the "([^"]+)" connector$"#)]
fn enable(world: &mut ConnectorWorld, id: String) {
    connectors::set_enabled(world.store(), &id, true).expect("enable should succeed");
}

#[when(regex = r#"^I remove the "([^"]+)" connector$"#)]
fn remove(world: &mut ConnectorWorld, id: String) {
    world.removal = Some(connectors::remove(world.store(), &id).expect("remove should succeed"));
}

#[then("every add succeeds")]
fn every_add_succeeds(world: &mut ConnectorWorld) {
    let failures: Vec<_> = world
        .adds
        .iter()
        .filter_map(|r| r.as_ref().err())
        .cloned()
        .collect();
    assert!(failures.is_empty(), "adds failed: {failures:?}");
    assert!(!world.adds.is_empty(), "no connector was attempted");
}

#[then("the add succeeds")]
fn add_succeeds(world: &mut ConnectorWorld) {
    let last = world.adds.last().expect("an add step must run first");
    assert!(last.is_ok(), "add failed: {:?}", last.as_ref().err());
}

#[then("each added connector appears in the workspace listing")]
fn each_added_is_listed(world: &mut ConnectorWorld) {
    let listed: Vec<String> = connectors::list(world.store())
        .expect("listing should succeed")
        .into_iter()
        .map(|c| c.id)
        .collect();
    for added in world.adds.iter().filter_map(|r| r.as_ref().ok()) {
        assert!(
            listed.contains(&added.id),
            "`{}` was added but is not in the listing: {listed:?}",
            added.id
        );
    }
}

#[then("the add is refused because a credential is missing")]
fn refused_missing_credential(world: &mut ConnectorWorld) {
    let err = world
        .adds
        .last()
        .expect("an add step must run first")
        .as_ref()
        .expect_err("adding without a required credential must fail");
    // The message names the field the way the user sees it — "`Personal access
    // token` is required for GitHub" — not the env var. That is the better
    // message; the assertion was what needed widening.
    assert!(
        err.to_lowercase().contains("required"),
        "the error should say what is required, got: {err}"
    );
    let spec = connectors::spec("github").expect("github is in the catalog");
    let label = spec.credentials[0].label;
    assert!(
        err.contains(label) || err.contains(spec.credentials[0].env),
        "the error should name the credential ({label} or {}), got: {err}",
        spec.credentials[0].env
    );
}

#[then("the add is refused as unknown")]
fn refused_unknown(world: &mut ConnectorWorld) {
    let err = world
        .adds
        .last()
        .expect("an add step must run first")
        .as_ref()
        .expect_err("an unknown id must fail");
    assert!(err.contains("unknown"), "got: {err}");
}

#[then("the second add is refused as already configured")]
fn refused_duplicate(world: &mut ConnectorWorld) {
    assert_eq!(world.adds.len(), 2, "two add steps should have run");
    assert!(world.adds[0].is_ok(), "the first add should have succeeded");
    let err = world.adds[1]
        .as_ref()
        .expect_err("adding the same connector twice must fail");
    assert!(
        err.contains("already configured"),
        "the error should say it is already configured, got: {err}"
    );
}

#[then("the connector reports no missing credentials")]
fn no_missing_credentials(world: &mut ConnectorWorld) {
    let added = world
        .adds
        .last()
        .expect("an add step must run first")
        .as_ref()
        .expect("the add should have succeeded");
    let missing = connectors::missing_credentials(world.store(), added)
        .expect("checking credentials should succeed");
    assert!(missing.is_empty(), "unexpectedly missing: {missing:?}");
}

#[then(regex = r#"^the agent's resolved servers include "([^"]+)"$"#)]
fn resolved_includes(world: &mut ConnectorWorld, id: String) {
    let names: Vec<String> = world.resolved().into_iter().map(|c| c.name).collect();
    assert!(names.contains(&id), "`{id}` not handed to the agent: {names:?}");
}

#[then(regex = r#"^the agent's resolved servers do not include "([^"]+)"$"#)]
fn resolved_excludes(world: &mut ConnectorWorld, id: String) {
    let names: Vec<String> = world.resolved().into_iter().map(|c| c.name).collect();
    assert!(
        !names.contains(&id),
        "`{id}` is still handed to the agent: {names:?}"
    );
}

#[then(regex = r#"^the workspace listing still shows "([^"]+)"$"#)]
fn listing_still_shows(world: &mut ConnectorWorld, id: String) {
    let listed: Vec<String> = connectors::list(world.store())
        .expect("listing should succeed")
        .into_iter()
        .map(|c| c.id)
        .collect();
    assert!(
        listed.contains(&id),
        "disabling should not remove `{id}` from the panel: {listed:?}"
    );
}

#[then("no resolved server argument still contains the workspace placeholder")]
fn placeholder_substituted(world: &mut ConnectorWorld) {
    for cfg in world.resolved() {
        for arg in &cfg.args {
            assert!(
                !arg.contains(WORKSPACE_PLACEHOLDER),
                "`{}` was handed the literal placeholder: {arg}",
                cfg.name
            );
        }
    }
}

#[then(regex = r#"^the resolved arguments for "([^"]+)" name the workspace directory$"#)]
fn args_name_workspace(world: &mut ConnectorWorld, id: String) {
    let workspace = world.workspace().display().to_string();
    let cfg = world
        .resolved()
        .into_iter()
        .find(|c| c.name == id)
        .unwrap_or_else(|| panic!("`{id}` should be resolved"));
    assert!(
        cfg.args.iter().any(|a| a.contains(&workspace)),
        "expected the workspace path in {:?}",
        cfg.args
    );
}

#[then(regex = r#"^the resolved environment for "([^"]+)" carries "([^"]+)"$"#)]
fn env_carries(world: &mut ConnectorWorld, id: String, env: String) {
    let cfg = world
        .resolved()
        .into_iter()
        .find(|c| c.name == id)
        .unwrap_or_else(|| panic!("`{id}` should be resolved"));
    assert!(
        cfg.env.contains_key(&env),
        "`{env}` missing from the server's environment: {:?}",
        cfg.env.keys().collect::<Vec<_>>()
    );
}

#[then(regex = r#"^no file in the workspace contains "([^"]+)"$"#)]
fn no_plaintext_secret(world: &mut ConnectorWorld, needle: String) {
    fn walk(dir: &Path, out: &mut Vec<PathBuf>) {
        let Ok(entries) = std::fs::read_dir(dir) else { return };
        for entry in entries.flatten() {
            let path = entry.path();
            if path.is_dir() {
                walk(&path, out);
            } else {
                out.push(path);
            }
        }
    }
    let mut files = Vec::new();
    walk(world.workspace(), &mut files);
    assert!(!files.is_empty(), "the workspace should contain the store");

    for file in files {
        let bytes = std::fs::read(&file).unwrap_or_default();
        // Bytes, not text: a secret sitting in a binary DB page is just as
        // readable as one in a config file.
        let leaked = bytes
            .windows(needle.len())
            .any(|w| w == needle.as_bytes());
        assert!(!leaked, "`{needle}` found in plaintext in {}", file.display());
    }
}

#[then(regex = r"^the removal reports (\d+) deleted secret$")]
fn removal_deleted_secrets(world: &mut ConnectorWorld, expected: usize) {
    let (removed, deleted) = world.removal.expect("a remove step must run first");
    assert!(removed, "the connector should have been removed");
    assert_eq!(deleted, expected, "unexpected number of deleted secrets");
}

fn main() {
    futures::executor::block_on(ConnectorWorld::run(
        "tests/features/vibedesk_connectors.feature",
    ));
}
