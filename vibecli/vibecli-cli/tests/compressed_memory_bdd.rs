/*!
 * BDD tests for CompressedMemoryIndex — TurboQuant-backed memory store.
 *
 * RED phase: vibecli_cli::compressed_hnsw does not exist yet, so this file
 * must fail to compile. Once the module lands the file should compile and
 * the four scenarios below should pass.
 *
 * Run with: cargo test --test compressed_memory_bdd
 */
use cucumber::{World, given, then, when};
use rand::{Rng, SeedableRng};
use rand::rngs::StdRng;
use std::collections::HashMap;
use vibecli_cli::compressed_hnsw::CompressedMemoryIndex;

#[derive(Default, World)]
pub struct CmWorld {
    index: Option<CompressedMemoryIndex>,
    /// Ground-truth vectors mirrored alongside the compressed index so we can
    /// score recall@k against exact cosine search.
    truth: Vec<(String, Vec<f32>)>,
    last_results: Vec<String>,
    last_recall: Option<f32>,
}

impl std::fmt::Debug for CmWorld {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("CmWorld")
            .field("index_len", &self.index.as_ref().map(|i| i.len()))
            .field("truth_len", &self.truth.len())
            .field("last_results", &self.last_results)
            .field("last_recall", &self.last_recall)
            .finish()
    }
}

/* ── Helpers ────────────────────────────────────────────────────────────── */

fn random_unit_vec(rng: &mut StdRng, dim: usize) -> Vec<f32> {
    let mut v: Vec<f32> = (0..dim).map(|_| rng.gen_range(-1.0_f32..1.0)).collect();
    let norm = v.iter().map(|x| x * x).sum::<f32>().sqrt();
    if norm > 0.0 {
        for x in &mut v {
            *x /= norm;
        }
    }
    v
}

/* ── Given ──────────────────────────────────────────────────────────────── */

#[given(regex = r"^a compressed memory index of dimension (\d+)$")]
fn given_index(w: &mut CmWorld, dim: usize) {
    w.index = Some(CompressedMemoryIndex::new(dim));
    w.truth.clear();
}

#[given(regex = r"^a compressed memory index of dimension (\d+) seeded with (\d+) random vectors$")]
fn given_seeded(w: &mut CmWorld, dim: usize, n: usize) {
    let mut idx = CompressedMemoryIndex::new(dim);
    let mut rng = StdRng::seed_from_u64(0xC0DE_F00D);
    let mut truth = Vec::with_capacity(n);
    for i in 0..n {
        let id = format!("seed{i}");
        let v = random_unit_vec(&mut rng, dim);
        idx.insert(id.clone(), &v, HashMap::new());
        truth.push((id, v));
    }
    w.index = Some(idx);
    w.truth = truth;
}

/// Clustered seeding — `k_clusters` random unit centroids, `per_cluster` noisy
/// variants per centroid. This is the realistic shape for text embeddings.
#[given(regex = r"^a compressed memory index of dimension (\d+) seeded with (\d+) clusters of (\d+) vectors$")]
fn given_clustered(w: &mut CmWorld, dim: usize, k_clusters: usize, per_cluster: usize) {
    let mut idx = CompressedMemoryIndex::new(dim);
    let mut rng = StdRng::seed_from_u64(0xC0DE_F00D);
    let centroids: Vec<Vec<f32>> = (0..k_clusters).map(|_| random_unit_vec(&mut rng, dim)).collect();
    let mut truth = Vec::with_capacity(k_clusters * per_cluster);
    // Per-component noise sigma. For dim=384 a unit vector's component
    // magnitude is ~1/sqrt(384) ≈ 0.051; sigma=0.03 yields intra-cluster
    // cosine ≈ 0.86, matching real text-embedding paraphrase clusters.
    let noise_sigma = 0.03_f32;
    for (c, centroid) in centroids.iter().enumerate() {
        for j in 0..per_cluster {
            let mut v: Vec<f32> = centroid
                .iter()
                .map(|x| x + rng.gen_range(-noise_sigma..noise_sigma))
                .collect();
            let norm = v.iter().map(|x| x * x).sum::<f32>().sqrt();
            if norm > 0.0 {
                for x in &mut v {
                    *x /= norm;
                }
            }
            let id = format!("c{c}_v{j}");
            idx.insert(id.clone(), &v, HashMap::new());
            truth.push((id, v));
        }
    }
    w.index = Some(idx);
    w.truth = truth;
}

/* ── When ───────────────────────────────────────────────────────────────── */

#[when(regex = r#"^(\d+) random unit vectors are inserted with ids "v0"\.\."v(\d+)"$"#)]
fn when_insert_named(w: &mut CmWorld, count: usize, _last: usize) {
    let idx = w.index.as_mut().expect("index");
    let dim = idx_dim_or_panic(idx);
    let mut rng = StdRng::seed_from_u64(0xA11CE);
    for i in 0..count {
        let id = format!("v{i}");
        let v = random_unit_vec(&mut rng, dim);
        idx.insert(id.clone(), &v, HashMap::new());
        w.truth.push((id, v));
    }
}

#[when(regex = r"^(\d+) random unit vectors are inserted$")]
fn when_insert_anon(w: &mut CmWorld, count: usize) {
    let idx = w.index.as_mut().expect("index");
    let dim = idx_dim_or_panic(idx);
    let mut rng = StdRng::seed_from_u64(0xB0B);
    for i in 0..count {
        let v = random_unit_vec(&mut rng, dim);
        idx.insert(format!("anon{i}"), &v, HashMap::new());
    }
}

#[when(regex = r#"^the vector at id "([^"]+)" is queried with top_k (\d+)$"#)]
fn when_query_id(w: &mut CmWorld, id: String, k: usize) {
    let v = w
        .truth
        .iter()
        .find(|(x, _)| x == &id)
        .map(|(_, v)| v.clone())
        .expect("id present in truth");
    let idx = w.index.as_ref().expect("index");
    w.last_results = idx.search(&v, k).into_iter().map(|h| h.id).collect();
}

#[when(regex = r"^the zero vector is queried with top_k (\d+)$")]
fn when_query_zero(w: &mut CmWorld, k: usize) {
    let idx = w.index.as_ref().expect("index");
    let dim = idx_dim_or_panic(idx);
    let q = vec![0.0_f32; dim];
    w.last_results = idx.search(&q, k).into_iter().map(|h| h.id).collect();
}

#[when(regex = r"^(\d+) noisy-seed queries are run with top_k (\d+)$")]
fn when_noisy_seed_queries(w: &mut CmWorld, n_queries: usize, k: usize) {
    let idx = w.index.as_ref().expect("index");
    let mut rng = StdRng::seed_from_u64(0xD1CE);
    let query_noise = 0.02_f32;
    let mut hits = 0_usize;
    assert!(!w.truth.is_empty(), "no truth seeded");
    for _ in 0..n_queries {
        let pick = rng.gen_range(0..w.truth.len());
        let target_id = w.truth[pick].0.clone();
        let base = &w.truth[pick].1;
        let mut q: Vec<f32> = base
            .iter()
            .map(|x| x + rng.gen_range(-query_noise..query_noise))
            .collect();
        let norm = q.iter().map(|x| x * x).sum::<f32>().sqrt();
        if norm > 0.0 {
            for x in &mut q {
                *x /= norm;
            }
        }
        let returned: Vec<String> = idx.search(&q, k).into_iter().map(|h| h.id).collect();
        if returned.iter().any(|id| id == &target_id) {
            hits += 1;
        }
    }
    let rate = hits as f32 / n_queries as f32;
    w.last_recall = Some(rate);
}

/* ── Then ───────────────────────────────────────────────────────────────── */

#[then(regex = r#"^the top result id is "([^"]+)"$"#)]
fn then_top_id(w: &mut CmWorld, id: String) {
    let top = w.last_results.first().cloned().unwrap_or_default();
    assert_eq!(top, id, "top result was {top:?}, expected {id:?}");
}

#[then(regex = r"^the reported compression ratio is at least ([0-9.]+)$")]
fn then_ratio(w: &mut CmWorld, floor: f64) {
    let r = w.index.as_ref().expect("index").compression_ratio();
    assert!(r >= floor, "compression ratio {r} below floor {floor}");
}

#[then(regex = r"^target-hit rate at top-(\d+) is at least ([0-9.]+)$")]
fn then_target_hit(w: &mut CmWorld, _k: usize, floor: f32) {
    let r = w.last_recall.expect("hit-rate computed");
    assert!(r >= floor, "target-hit rate = {r} below floor {floor}");
}

#[then(regex = r"^the result list is empty$")]
fn then_empty(w: &mut CmWorld) {
    assert!(w.last_results.is_empty(), "got {:?}", w.last_results);
}

/* ── Internals ──────────────────────────────────────────────────────────── */

fn idx_dim_or_panic(idx: &CompressedMemoryIndex) -> usize {
    idx.dimension()
}

fn main() {
    futures::executor::block_on(CmWorld::run(
        "tests/features/compressed_memory.feature",
    ));
}
