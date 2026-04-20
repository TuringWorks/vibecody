/*!
 * BDD tests for LocalEmbeddingEngine — fixed-dimension feature-hashed TF-IDF.
 *
 * RED phase: LocalEmbeddingEngine has no with_dim() / dim() yet, so this file
 * must fail to compile. After the GREEN refactor the engine produces vectors
 * of a constant dimension regardless of how the vocabulary grows.
 *
 * Run with: cargo test --test embedding_engine_bdd
 */
use cucumber::{World, given, then, when};
use std::collections::HashMap;
use vibecli_cli::open_memory::LocalEmbeddingEngine;

#[derive(Default, World)]
pub struct EngWorld {
    engine: Option<LocalEmbeddingEngine>,
    vecs: HashMap<String, Vec<f32>>,
}

impl std::fmt::Debug for EngWorld {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("EngWorld")
            .field("engine_dim", &self.engine.as_ref().map(|e| e.dim()))
            .field("vec_keys", &self.vecs.keys().collect::<Vec<_>>())
            .finish()
    }
}

impl EngWorld {
    fn engine_mut(&mut self) -> &mut LocalEmbeddingEngine {
        self.engine.as_mut().expect("engine not built")
    }
    fn engine(&self) -> &LocalEmbeddingEngine {
        self.engine.as_ref().expect("engine not built")
    }
}

/* ── Given ──────────────────────────────────────────────────────────────── */

#[given(regex = r"^a local embedding engine with dimension (\d+)$")]
fn given_engine(w: &mut EngWorld, dim: usize) {
    w.engine = Some(LocalEmbeddingEngine::with_dim(dim));
    w.vecs.clear();
}

/* ── When ───────────────────────────────────────────────────────────────── */

#[when(regex = r#"^document "([^"]+)" is added$"#)]
fn when_doc_added(w: &mut EngWorld, text: String) {
    w.engine_mut().add_document(&text);
}

#[when(regex = r#"^"([^"]+)" is embedded as "([^"]+)"$"#)]
fn when_embed_as(w: &mut EngWorld, text: String, name: String) {
    let v = w.engine().embed(&text);
    w.vecs.insert(name, v);
}

#[when(regex = r#"^document "([^"]+)" is added and embedded as "([^"]+)"$"#)]
fn when_doc_added_and_embedded(w: &mut EngWorld, text: String, name: String) {
    w.engine_mut().add_document(&text);
    let v = w.engine().embed(&text);
    w.vecs.insert(name, v);
}

#[when(regex = r#"^document "([^"]+)" is embedded as "([^"]+)"$"#)]
fn when_doc_embedded(w: &mut EngWorld, text: String, name: String) {
    let v = w.engine().embed(&text);
    w.vecs.insert(name, v);
}

#[when(regex = r"^(\d+) unrelated documents are added$")]
fn when_unrelated_docs(w: &mut EngWorld, n: usize) {
    let eng = w.engine_mut();
    // Synthetic tokens that won't collide with English test corpora.
    for i in 0..n {
        let doc = format!(
            "filler{i}_alpha filler{i}_beta filler{i}_gamma filler{i}_delta"
        );
        eng.add_document(&doc);
    }
}

/* ── Then ───────────────────────────────────────────────────────────────── */

#[then(regex = r#"^"([^"]+)" has length (\d+)$"#)]
fn then_len(w: &mut EngWorld, name: String, expected: usize) {
    let v = w.vecs.get(&name).expect("vec not embedded");
    assert_eq!(v.len(), expected, "{name} length = {} != {expected}", v.len());
}

#[then(regex = r#"^cosine similarity between "([^"]+)" and "([^"]+)" is at least ([0-9.]+)$"#)]
fn then_cos_floor(w: &mut EngWorld, a: String, b: String, floor: f64) {
    let va = w.vecs.get(&a).expect("a missing");
    let vb = w.vecs.get(&b).expect("b missing");
    let sim = LocalEmbeddingEngine::cosine_similarity(va, vb);
    assert!(sim >= floor, "cos({a},{b}) = {sim:.4} below floor {floor}");
}

#[then(regex = r#"^cosine "([^"]+)" "([^"]+)" is greater than cosine "([^"]+)" "([^"]+)"$"#)]
fn then_cos_gt(w: &mut EngWorld, a: String, b: String, c: String, d: String) {
    let va = w.vecs.get(&a).expect("a missing");
    let vb = w.vecs.get(&b).expect("b missing");
    let vc = w.vecs.get(&c).expect("c missing");
    let vd = w.vecs.get(&d).expect("d missing");
    let sim_ab = LocalEmbeddingEngine::cosine_similarity(va, vb);
    let sim_cd = LocalEmbeddingEngine::cosine_similarity(vc, vd);
    assert!(
        sim_ab > sim_cd,
        "expected cos({a},{b})={sim_ab:.4} > cos({c},{d})={sim_cd:.4}"
    );
}

fn main() {
    futures::executor::block_on(EngWorld::run(
        "tests/features/embedding_engine.feature",
    ));
}
