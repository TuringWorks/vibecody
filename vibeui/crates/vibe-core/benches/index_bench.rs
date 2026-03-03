//! Criterion benchmarks for hot paths in vibe-core index & embedding modules.

use criterion::{black_box, criterion_group, criterion_main, Criterion};
use std::path::PathBuf;
use vibe_core::index::{
    cosine_similarity, CodebaseIndex, Language,
};
use vibe_core::index::symbol::extract_symbols;

// ── Cosine similarity ────────────────────────────────────────────────────────

fn bench_cosine_similarity_small(c: &mut Criterion) {
    let a: Vec<f32> = (0..384).map(|i| (i as f32 * 0.01).sin()).collect();
    let b: Vec<f32> = (0..384).map(|i| (i as f32 * 0.02).cos()).collect();

    c.bench_function("cosine_similarity_384d", |bencher| {
        bencher.iter(|| cosine_similarity(black_box(&a), black_box(&b)));
    });
}

fn bench_cosine_similarity_large(c: &mut Criterion) {
    let a: Vec<f32> = (0..1536).map(|i| (i as f32 * 0.01).sin()).collect();
    let b: Vec<f32> = (0..1536).map(|i| (i as f32 * 0.02).cos()).collect();

    c.bench_function("cosine_similarity_1536d", |bencher| {
        bencher.iter(|| cosine_similarity(black_box(&a), black_box(&b)));
    });
}

fn bench_cosine_similarity_batch(c: &mut Criterion) {
    let query: Vec<f32> = (0..384).map(|i| (i as f32 * 0.01).sin()).collect();
    let corpus: Vec<Vec<f32>> = (0..1000)
        .map(|j| (0..384).map(|i| ((i + j) as f32 * 0.007).cos()).collect())
        .collect();

    c.bench_function("cosine_similarity_1000x384d", |bencher| {
        bencher.iter(|| {
            let mut best = 0.0f32;
            for v in &corpus {
                let sim = cosine_similarity(black_box(&query), black_box(v));
                if sim > best {
                    best = sim;
                }
            }
            best
        });
    });
}

// ── Symbol extraction ────────────────────────────────────────────────────────

fn generate_rust_source(n_functions: usize) -> String {
    let mut buf = String::with_capacity(n_functions * 80);
    for i in 0..n_functions {
        buf.push_str(&format!(
            "pub fn function_{i}(x: i32) -> i32 {{\n    x + {i}\n}}\n\n"
        ));
        if i % 5 == 0 {
            buf.push_str(&format!("pub struct Struct{i} {{\n    field: i32,\n}}\n\n"));
        }
        if i % 10 == 0 {
            buf.push_str(&format!("pub enum Enum{i} {{\n    A,\n    B,\n}}\n\n"));
        }
    }
    buf
}

fn bench_extract_symbols_small(c: &mut Criterion) {
    let content = generate_rust_source(50);
    let path = PathBuf::from("bench.rs");

    c.bench_function("extract_symbols_50fn", |bencher| {
        bencher.iter(|| extract_symbols(black_box(&path), black_box(&content), &Language::Rust));
    });
}

fn bench_extract_symbols_large(c: &mut Criterion) {
    let content = generate_rust_source(500);
    let path = PathBuf::from("bench.rs");

    c.bench_function("extract_symbols_500fn", |bencher| {
        bencher.iter(|| extract_symbols(black_box(&path), black_box(&content), &Language::Rust));
    });
}

// ── CodebaseIndex build ──────────────────────────────────────────────────────

fn bench_index_build(c: &mut Criterion) {
    let dir = tempfile::tempdir().expect("create temp dir");
    let src = dir.path().join("src");
    std::fs::create_dir_all(&src).unwrap();

    // Create 100 small Rust source files
    for i in 0..100 {
        let content = generate_rust_source(10);
        std::fs::write(src.join(format!("mod_{i}.rs")), &content).unwrap();
    }

    c.bench_function("index_build_100_files", |bencher| {
        bencher.iter(|| {
            let mut idx = CodebaseIndex::new(dir.path().to_path_buf());
            idx.build().unwrap();
            idx
        });
    });
}

// ── CodebaseIndex search ─────────────────────────────────────────────────────

fn bench_search_symbols(c: &mut Criterion) {
    let dir = tempfile::tempdir().expect("create temp dir");
    let src = dir.path().join("src");
    std::fs::create_dir_all(&src).unwrap();

    for i in 0..50 {
        let content = generate_rust_source(20);
        std::fs::write(src.join(format!("mod_{i}.rs")), &content).unwrap();
    }

    let mut idx = CodebaseIndex::new(dir.path().to_path_buf());
    idx.build().unwrap();

    c.bench_function("search_symbols_1000syms", |bencher| {
        bencher.iter(|| idx.search_symbols(black_box("function_5")));
    });
}

fn bench_relevant_symbols(c: &mut Criterion) {
    let dir = tempfile::tempdir().expect("create temp dir");
    let src = dir.path().join("src");
    std::fs::create_dir_all(&src).unwrap();

    for i in 0..50 {
        let content = generate_rust_source(20);
        std::fs::write(src.join(format!("mod_{i}.rs")), &content).unwrap();
    }

    let mut idx = CodebaseIndex::new(dir.path().to_path_buf());
    idx.build().unwrap();

    c.bench_function("relevant_symbols_1000syms", |bencher| {
        bencher.iter(|| idx.relevant_symbols(black_box("authenticate user login"), 20));
    });
}

// ── Groups ───────────────────────────────────────────────────────────────────

criterion_group!(
    cosine_benches,
    bench_cosine_similarity_small,
    bench_cosine_similarity_large,
    bench_cosine_similarity_batch,
);

criterion_group!(
    symbol_benches,
    bench_extract_symbols_small,
    bench_extract_symbols_large,
);

criterion_group!(
    index_benches,
    bench_index_build,
    bench_search_symbols,
    bench_relevant_symbols,
);

criterion_main!(cosine_benches, symbol_benches, index_benches);
