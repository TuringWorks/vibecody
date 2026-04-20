/*!
 * BDD tests for swapping HnswIndex → CompressedMemoryIndex inside
 * OpenMemoryStore. Drives RED via:
 *   - missing accessor `embedding_compression_ratio`
 *   - the f32-backed HnswIndex would never report ≥ 8× compression
 * Once the swap lands, basic add → query → delete must still work.
 *
 * Run with: cargo test --test compressed_memory_store_bdd
 */
use cucumber::{World, given, then, when};
use std::collections::HashMap;
use tempfile::TempDir;
use vibecli_cli::open_memory::{OpenMemoryStore, QueryResult};

#[derive(Default, World)]
pub struct StoreWorld {
    tmp: Option<TempDir>,
    store: Option<OpenMemoryStore>,
    captured_ids: HashMap<String, String>,
    last_query: Vec<QueryResult>,
}

impl std::fmt::Debug for StoreWorld {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("StoreWorld")
            .field("tmp", &self.tmp.as_ref().map(|t| t.path().to_path_buf()))
            .field("store", &self.store.is_some())
            .field("captured", &self.captured_ids)
            .field("hits", &self.last_query.len())
            .finish()
    }
}

impl StoreWorld {
    fn store_mut(&mut self) -> &mut OpenMemoryStore {
        self.store.as_mut().expect("store not built")
    }
    fn store(&self) -> &OpenMemoryStore {
        self.store.as_ref().expect("store not built")
    }
}

/* ── Given ──────────────────────────────────────────────────────────────── */

#[given(regex = r"^a fresh OpenMemoryStore$")]
fn given_fresh(w: &mut StoreWorld) {
    let tmp = tempfile::tempdir().expect("tempdir");
    w.store = Some(OpenMemoryStore::new(tmp.path(), "alice"));
    w.tmp = Some(tmp);
    w.captured_ids.clear();
    w.last_query.clear();
}

/* ── When ───────────────────────────────────────────────────────────────── */

#[when(regex = r"^(\d+) distinct memories are added$")]
fn when_add_many(w: &mut StoreWorld, n: usize) {
    let store = w.store_mut();
    // Synthetic content with no overlap so each one is its own cluster.
    for i in 0..n {
        let content = format!(
            "memory_{i}_alpha_{i} memory_{i}_beta_{i} memory_{i}_gamma_{i}"
        );
        store.add(content);
    }
}

#[when(regex = r#"^the memory "([^"]+)" is added$"#)]
fn when_add_one(w: &mut StoreWorld, content: String) {
    w.store_mut().add(content);
}

#[when(regex = r#"^the memory "([^"]+)" is added and its id captured as "([^"]+)"$"#)]
fn when_add_capture(w: &mut StoreWorld, content: String, name: String) {
    let id = w.store_mut().add(content);
    w.captured_ids.insert(name, id);
}

#[when(regex = r#"^the memory at id "([^"]+)" is deleted$"#)]
fn when_delete_captured(w: &mut StoreWorld, name: String) {
    let id = w.captured_ids.get(&name).cloned().expect("captured id");
    let removed = w.store_mut().delete(&id);
    assert!(removed, "delete returned false for captured id {id}");
}

#[when(regex = r#"^the store is queried for "([^"]+)" with limit (\d+)$"#)]
fn when_query(w: &mut StoreWorld, q: String, k: usize) {
    w.last_query = w.store().query(&q, k);
}

/* ── Then ───────────────────────────────────────────────────────────────── */

#[then(regex = r"^the embedding compression ratio is at least ([0-9.]+)$")]
fn then_ratio(w: &mut StoreWorld, floor: f64) {
    let r = w.store().embedding_compression_ratio();
    assert!(r >= floor, "compression ratio = {r:.3} below floor {floor}");
}

#[then(regex = r#"^the top result content is "([^"]+)"$"#)]
fn then_top_content(w: &mut StoreWorld, expected: String) {
    let top = w.last_query.first().expect("at least one result");
    assert_eq!(
        top.memory.content, expected,
        "top content = {:?}, expected {:?}",
        top.memory.content, expected
    );
}

#[then(regex = r#"^no result has id equal to "([^"]+)"$"#)]
fn then_no_id(w: &mut StoreWorld, name: String) {
    let id = w.captured_ids.get(&name).cloned().expect("captured id");
    let bad: Vec<String> = w
        .last_query
        .iter()
        .filter(|r| r.memory.id == id)
        .map(|r| r.memory.id.clone())
        .collect();
    assert!(bad.is_empty(), "deleted id {id} still appears in results: {bad:?}");
}

fn main() {
    futures::executor::block_on(StoreWorld::run(
        "tests/features/compressed_memory_store.feature",
    ));
}
