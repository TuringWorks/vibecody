/*!
 * BDD tests for session_tree using Cucumber.
 * Run with: cargo test --test session_tree_bdd
 */
use cucumber::{World, given, then, when};
use vibecli_cli::session_tree::{EntryId, EntryKind, SessionTree};

// ---------------------------------------------------------------------------
// World
// ---------------------------------------------------------------------------

#[derive(Debug, Default, World)]
pub struct StWorld {
    tree: SessionTree,
    /// Maps a human-readable name (from scenario) to an EntryId.
    named: std::collections::HashMap<String, EntryId>,
    /// The id of the most recently appended entry.
    last_id: Option<EntryId>,
    /// JSONL serialization scratch space.
    jsonl: String,
    /// Tree restored from JSONL.
    restored: Option<SessionTree>,
    /// Entries visible after a fold operation.
    folded_count: usize,
}

impl StWorld {
    fn tree_ref(&self) -> &SessionTree {
        &self.tree
    }
    fn resolved_parent(&self, name: &str) -> Option<String> {
        self.named.get(name).map(|id| id.0.clone())
    }
}

// ---------------------------------------------------------------------------
// Given
// ---------------------------------------------------------------------------

#[given("a new session tree")]
fn new_tree(world: &mut StWorld) {
    world.tree = SessionTree::new();
    world.named.clear();
    world.last_id = None;
    world.jsonl.clear();
    world.restored = None;
    world.folded_count = 0;
}

// ---------------------------------------------------------------------------
// When — append / branch
// ---------------------------------------------------------------------------

#[when(expr = "I append a user message {string}")]
fn append_user(world: &mut StWorld, content: String) {
    let id = world.tree.append(None, EntryKind::Message {
        role: "user".to_owned(),
        content: content.clone(),
    });
    world.named.insert(content.clone(), id.clone());
    world.last_id = Some(id);
}

#[when(expr = "I append an assistant message {string} as child of the last entry")]
fn append_assistant_of_last(world: &mut StWorld, content: String) {
    let parent = world.last_id.as_ref().map(|id| id.0.clone());
    let id = world.tree.append(
        parent.as_deref(),
        EntryKind::Message { role: "assistant".to_owned(), content: content.clone() },
    );
    world.named.insert(content.clone(), id.clone());
    world.last_id = Some(id);
}

#[when(expr = "I append an assistant message {string} as child of {string}")]
fn append_assistant_of_named(world: &mut StWorld, content: String, parent_name: String) {
    let parent_id = world.resolved_parent(&parent_name);
    let id = world.tree.append(
        parent_id.as_deref(),
        EntryKind::Message { role: "assistant".to_owned(), content: content.clone() },
    );
    world.named.insert(content.clone(), id.clone());
    world.last_id = Some(id);
}

#[when(expr = "I append a user message {string} as child of {string}")]
fn append_user_of_named(world: &mut StWorld, content: String, parent_name: String) {
    let parent_id = world.resolved_parent(&parent_name);
    let id = world.tree.append(
        parent_id.as_deref(),
        EntryKind::Message { role: "user".to_owned(), content: content.clone() },
    );
    world.named.insert(content.clone(), id.clone());
    world.last_id = Some(id);
}

#[when(expr = "I branch from {string} with an assistant message {string}")]
fn branch_from_named(world: &mut StWorld, parent_name: String, content: String) {
    let parent_id = world.resolved_parent(&parent_name)
        .expect("parent name not found in named map");
    let id = world.tree
        .branch_from(&parent_id, EntryKind::Message {
            role: "assistant".to_owned(),
            content: content.clone(),
        })
        .expect("branch_from failed");
    world.named.insert(content.clone(), id.clone());
    world.last_id = Some(id);
}

#[when(expr = "I append a compaction entry under {string}")]
fn append_compaction(world: &mut StWorld, parent_name: String) {
    let parent_id = world.resolved_parent(&parent_name);
    let id = world.tree.append(
        parent_id.as_deref(),
        EntryKind::Compaction {
            summary: "compacted context".to_owned(),
            files_touched: vec!["src/lib.rs".to_owned()],
        },
    );
    world.named.insert("__compaction__".to_owned(), id.clone());
    world.last_id = Some(id);
}

// ---------------------------------------------------------------------------
// When — serialization
// ---------------------------------------------------------------------------

#[when("I serialize the tree to JSONL")]
fn serialize(world: &mut StWorld) {
    world.jsonl = world.tree.serialize_jsonl();
}

#[when("I deserialize the JSONL into a new tree")]
fn deserialize(world: &mut StWorld) {
    world.restored = Some(
        SessionTree::deserialize_jsonl(&world.jsonl).expect("deserialize failed"),
    );
}

// ---------------------------------------------------------------------------
// When — label
// ---------------------------------------------------------------------------

#[when(expr = "I label that entry {string}")]
fn label_last(world: &mut StWorld, label: String) {
    let id = world.last_id.as_ref().expect("no last entry").0.clone();
    world.tree.label_entry(&id, &label);
}

// ---------------------------------------------------------------------------
// When — fold
// ---------------------------------------------------------------------------

#[when(expr = "I fold the subtree at {string}")]
fn fold_at(world: &mut StWorld, entry_name: String) {
    let id = world.resolved_parent(&entry_name)
        .expect("entry name not found for fold");
    let visible = world.tree.fold_subtree(&id);
    world.folded_count = visible.len();
}

// ---------------------------------------------------------------------------
// Then
// ---------------------------------------------------------------------------

#[then(expr = "the tree has {int} entries")]
fn check_entry_count(world: &mut StWorld, expected: usize) {
    assert_eq!(
        world.tree.entry_count(), expected,
        "entry count mismatch: got {}, want {}",
        world.tree.entry_count(), expected
    );
}

#[then("the last entry is a leaf")]
fn check_last_is_leaf(world: &mut StWorld) {
    let last_id = world.last_id.as_ref().expect("no last id");
    let leaves: Vec<_> = world.tree.leaf_entries();
    let is_leaf = leaves.iter().any(|e| e.id == *last_id);
    assert!(is_leaf, "last entry {:?} is not a leaf", last_id.0);
}

#[then(expr = "the branch count is {int}")]
fn check_branch_count(world: &mut StWorld, expected: usize) {
    assert_eq!(
        world.tree.branch_count(), expected,
        "branch count mismatch"
    );
}

#[then(expr = "the path to {string} has length {int}")]
fn check_path_length(world: &mut StWorld, entry_name: String, expected: usize) {
    let id = world.resolved_parent(&entry_name)
        .expect("entry name not found for path check");
    let path = world.tree.path_to(&id);
    assert_eq!(path.len(), expected, "path length mismatch: {:?}", path.len());
}

#[then(expr = "the first entry in the path is {string}")]
fn check_path_root(world: &mut StWorld, root_name: String) {
    // The "leaf" in the scenario is the deepest entry; we navigate to it.
    // Find the entry named by root_name in the tree.
    let leaf_id = world.last_id.as_ref().expect("no last id").0.clone();
    let path = world.tree.path_to(&leaf_id);
    let first = path.first().expect("path is empty");
    let expected_id = world.resolved_parent(&root_name)
        .expect("root name not in named map");
    assert_eq!(first.id.0, expected_id, "first path entry mismatch");
}

#[then(expr = "the restored tree has {int} entries")]
fn check_restored_count(world: &mut StWorld, expected: usize) {
    let count = world.restored.as_ref().expect("no restored tree").entry_count();
    assert_eq!(count, expected, "restored entry count mismatch");
}

#[then(expr = "only {int} entries are visible after folding")]
fn check_folded_count(world: &mut StWorld, expected: usize) {
    assert_eq!(
        world.folded_count, expected,
        "folded visible count mismatch: got {}, want {}",
        world.folded_count, expected
    );
}

#[then(expr = "the restored entry has label {string}")]
fn check_restored_label(world: &mut StWorld, expected_label: String) {
    let restored = world.restored.as_ref().expect("no restored tree");
    let has_label = restored
        .leaf_entries()
        .iter()
        .chain(restored.active_branch().iter())
        .any(|e| e.label.as_deref() == Some(&expected_label));
    // Walk all entries in restored tree via active_branch which covers the full path.
    let full_active = restored.active_branch();
    let found = full_active.iter().any(|e| e.label.as_deref() == Some(&expected_label));
    assert!(found || has_label, "label '{}' not found in restored tree", expected_label);
}

// ---------------------------------------------------------------------------
// Main
// ---------------------------------------------------------------------------

fn main() {
    futures::executor::block_on(StWorld::run("tests/features/session_tree.feature"));
}
