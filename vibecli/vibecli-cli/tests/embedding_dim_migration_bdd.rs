/*!
 * BDD tests for OpenMemoryStore::load migrating legacy variable-dim
 * embeddings to the current engine dimension.
 *
 * RED phase: load() does not currently regenerate stale embeddings, so the
 * "every loaded memory has the engine dimension" check should fail and the
 * "queryable after migration" check should miss (cosine_similarity returns
 * 0.0 on length mismatch, dragging the right answer below the score floor).
 *
 * Run with: cargo test --test embedding_dim_migration_bdd
 */
use cucumber::{World, given, then, when};
use serde_json::json;
use std::path::PathBuf;
use tempfile::TempDir;
use vibecli_cli::open_memory::{LocalEmbeddingEngine, OpenMemoryStore, QueryResult};

#[derive(Default, World)]
pub struct MigWorld {
    tmp: Option<TempDir>,
    /// The "on-disk" embedding for the byte-equality scenario.
    saved_embedding: Vec<f32>,
    store: Option<OpenMemoryStore>,
    last_query: Vec<QueryResult>,
}

impl std::fmt::Debug for MigWorld {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("MigWorld")
            .field("tmp", &self.tmp.as_ref().map(|t| t.path().to_path_buf()))
            .field("saved_len", &self.saved_embedding.len())
            .field("loaded", &self.store.is_some())
            .field("query_hits", &self.last_query.len())
            .finish()
    }
}

impl MigWorld {
    fn data_dir(&self) -> PathBuf {
        self.tmp.as_ref().expect("tmp dir").path().to_path_buf()
    }
    fn write_memories(&self, memories: serde_json::Value) {
        let dir = self.data_dir();
        std::fs::create_dir_all(&dir).unwrap();
        std::fs::write(dir.join("memories.json"), serde_json::to_vec(&memories).unwrap())
            .unwrap();
    }
}

fn fresh_tmp() -> TempDir {
    tempfile::tempdir().expect("tempdir")
}

fn make_memory(id: &str, content: &str, embedding: Vec<f32>) -> serde_json::Value {
    json!({
        "id": id,
        "content": content,
        "sector": "Semantic",
        "secondary_sectors": [],
        "tags": [],
        "metadata": {},
        "salience": 0.5,
        "decay_lambda": 0.01,
        "embedding": embedding,
        "created_at": 1_700_000_000u64,
        "updated_at": 1_700_000_000u64,
        "last_seen_at": 1_700_000_000u64,
        "version": 1u32,
        "user_id": "alice",
        "project_id": null,
        "session_id": null,
        "pinned": false,
        "encrypted": false,
    })
}

/* ── Given ──────────────────────────────────────────────────────────────── */

#[given(regex = r"^a legacy memories\.json with (\d+) entries whose embeddings are length (\d+)$")]
fn given_legacy_n(w: &mut MigWorld, n: usize, len: usize) {
    w.tmp = Some(fresh_tmp());
    let stale: Vec<f32> = (0..len).map(|i| (i as f32) * 0.1 + 0.01).collect();
    let mems: Vec<serde_json::Value> = (0..n)
        .map(|i| make_memory(&format!("m{i}"), &format!("legacy memory {i}"), stale.clone()))
        .collect();
    w.write_memories(json!(mems));
}

#[given(regex = r#"^a legacy memories\.json with a target "([^"]+)" and (\d+) unrelated decoys all carrying length-(\d+) embeddings$"#)]
fn given_legacy_target_and_decoys(
    w: &mut MigWorld,
    target: String,
    decoy_count: usize,
    len: usize,
) {
    w.tmp = Some(fresh_tmp());
    let stale: Vec<f32> = (0..len).map(|i| (i as f32) * 0.1 + 0.01).collect();
    let mut mems = vec![make_memory("seed", &target, stale.clone())];
    let decoys = [
        "xylophone harmonica saxophone",
        "soufflé pastry custard meringue",
        "stratosphere troposphere mesosphere",
        "calculus algebra trigonometry",
    ];
    for (i, content) in decoys.iter().take(decoy_count).enumerate() {
        mems.push(make_memory(&format!("decoy{i}"), content, stale.clone()));
    }
    w.write_memories(json!(mems));
}

#[given(regex = r"^a legacy memories\.json with 1 entry whose embedding is already at the engine dimension$")]
fn given_legacy_correct(w: &mut MigWorld) {
    w.tmp = Some(fresh_tmp());
    // Build an embedding via the current engine so we know it's "correct dim".
    let mut eng = LocalEmbeddingEngine::new();
    eng.add_document("already correct memory");
    let correct = eng.embed("already correct memory");
    assert_eq!(correct.len(), eng.dim());
    w.saved_embedding = correct.clone();
    let mems = vec![make_memory("ok", "already correct memory", correct)];
    w.write_memories(json!(mems));
}

/* ── When ───────────────────────────────────────────────────────────────── */

#[when(regex = r"^the store is loaded from that directory$")]
fn when_load(w: &mut MigWorld) {
    let dir = w.data_dir();
    w.store = Some(OpenMemoryStore::load(&dir, "alice").expect("load"));
}

#[when(regex = r#"^the store is queried for "([^"]+)" with limit (\d+)$"#)]
fn when_query(w: &mut MigWorld, q: String, k: usize) {
    let store = w.store.as_ref().expect("store loaded");
    w.last_query = store.query(&q, k);
}

/* ── Then ───────────────────────────────────────────────────────────────── */

#[then(regex = r"^every loaded memory has an embedding of the engine's dimension$")]
fn then_dim_uniform(w: &mut MigWorld) {
    let store = w.store.as_ref().expect("store");
    let expected = LocalEmbeddingEngine::DEFAULT_DIM;
    let mems = store.list_memories(0, 1_000_000);
    assert!(!mems.is_empty(), "no memories loaded");
    for m in mems {
        assert_eq!(
            m.embedding.len(),
            expected,
            "memory {} embedding len = {} != engine dim {}",
            m.id, m.embedding.len(), expected
        );
    }
}

#[then(regex = r"^no loaded memory carries the legacy length-(\d+) embedding$")]
fn then_no_legacy_len(w: &mut MigWorld, legacy_len: usize) {
    let store = w.store.as_ref().expect("store");
    for m in store.list_memories(0, 1_000_000) {
        assert_ne!(
            m.embedding.len(), legacy_len,
            "memory {} still carries legacy length-{legacy_len} embedding",
            m.id
        );
    }
}

#[then(regex = r#"^the top result content is "([^"]+)"$"#)]
fn then_top_content(w: &mut MigWorld, expected: String) {
    let top = w.last_query.first().expect("at least one query result");
    assert_eq!(
        top.memory.content, expected,
        "top result content = {:?}, expected {:?}",
        top.memory.content, expected
    );
}

#[then(regex = r"^the top result similarity is greater than (\d+)$")]
fn then_top_sim_gt(w: &mut MigWorld, floor: f64) {
    let top = w.last_query.first().expect("at least one query result");
    assert!(
        top.similarity > floor,
        "top similarity = {:.4}, expected > {floor}",
        top.similarity
    );
}

#[then(regex = r"^the loaded embedding equals the on-disk embedding$")]
fn then_byte_equal(w: &mut MigWorld) {
    let store = w.store.as_ref().expect("store");
    let mems = store.list_memories(0, 1_000_000);
    assert_eq!(mems.len(), 1);
    let loaded = &mems[0].embedding;
    assert_eq!(
        loaded, &w.saved_embedding,
        "loaded embedding diverged from on-disk embedding"
    );
}

fn main() {
    futures::executor::block_on(MigWorld::run(
        "tests/features/embedding_dim_migration.feature",
    ));
}
