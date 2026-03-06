//! Criterion benchmarks for hot paths in vibe-core.
//!
//! Run with: `cargo bench -p vibe-core`
//! Compile-check only: `cargo bench -p vibe-core --no-run`

use criterion::{black_box, criterion_group, criterion_main, Criterion};
use std::path::PathBuf;
use vibe_core::index::{CodebaseIndex, Language};
use vibe_core::index::symbol::extract_symbols;
use vibe_core::diff::DiffEngine;

// ── Helpers ──────────────────────────────────────────────────────────────────

/// Generate a realistic Rust source file with ~`n_functions` functions,
/// structs every 5th entry, and enums every 10th.  A call with
/// `n_functions = 200` produces roughly 1 000 lines.
fn generate_rust_source(n_functions: usize) -> String {
    let mut buf = String::with_capacity(n_functions * 80);
    for i in 0..n_functions {
        buf.push_str(&format!(
            "pub fn function_{i}(x: i32) -> i32 {{\n    x + {i}\n}}\n\n"
        ));
        if i % 5 == 0 {
            buf.push_str(&format!(
                "pub struct Struct{i} {{\n    field: i32,\n}}\n\n"
            ));
        }
        if i % 10 == 0 {
            buf.push_str(&format!(
                "pub enum Enum{i} {{\n    A,\n    B,\n}}\n\n"
            ));
        }
    }
    buf
}

// ── 1. bench_extract_symbols ─────────────────────────────────────────────────
// Extract symbols from a generated ~1 000-line Rust file.

fn bench_extract_symbols(c: &mut Criterion) {
    let content = generate_rust_source(200); // ~1 000 lines
    let path = PathBuf::from("large_module.rs");

    c.bench_function("extract_symbols_1000_lines", |b| {
        b.iter(|| {
            extract_symbols(black_box(&path), black_box(&content), &Language::Rust)
        });
    });
}

// ── 2. bench_search_symbols ──────────────────────────────────────────────────
// Build an index with ~1 000 symbols, then search with several queries.

fn bench_search_symbols(c: &mut Criterion) {
    let dir = tempfile::tempdir().expect("create temp dir");
    let src = dir.path().join("src");
    std::fs::create_dir_all(&src).unwrap();

    // 50 files x 20 functions ≈ 1 000 function symbols + structs + enums
    for i in 0..50 {
        let content = generate_rust_source(20);
        std::fs::write(src.join(format!("mod_{i}.rs")), &content).unwrap();
    }

    let mut idx = CodebaseIndex::new(dir.path().to_path_buf());
    idx.build().unwrap();

    let queries = ["function_5", "Struct10", "nonexistent_symbol"];

    c.bench_function("search_symbols_varied_queries", |b| {
        b.iter(|| {
            for q in &queries {
                let _ = idx.search_symbols(black_box(q));
            }
        });
    });
}

// ── 3. bench_diff_generate ───────────────────────────────────────────────────
// Diff two ~500-line files that differ by scattered edits.

fn bench_diff_generate(c: &mut Criterion) {
    let original = generate_rust_source(100); // ~500 lines
    // Produce a modified version: change every 10th function body
    let modified: String = original
        .lines()
        .enumerate()
        .map(|(i, line)| {
            if i % 40 == 0 && line.contains("x +") {
                format!("    x * 2 + {i}")
            } else {
                line.to_string()
            }
        })
        .collect::<Vec<_>>()
        .join("\n");

    c.bench_function("diff_generate_500_lines", |b| {
        b.iter(|| {
            DiffEngine::generate_diff(black_box(&original), black_box(&modified))
        });
    });
}

// ── 4. bench_buffer_insert_delete ────────────────────────────────────────────
// Repeated insert + delete cycle on the rope-backed TextBuffer.

fn bench_buffer_insert_delete(c: &mut Criterion) {
    use vibe_core::buffer::{Position, Range, TextBuffer};

    // Seed the buffer with ~1 000 lines
    let seed = generate_rust_source(200);

    c.bench_function("buffer_insert_delete_cycle", |b| {
        b.iter(|| {
            let mut buf = TextBuffer::from_str(black_box(&seed));
            // Insert 50 lines at scattered positions
            for i in 0..50 {
                let line = i * 10;
                buf.insert(Position::new(line, 0), "// inserted\n").unwrap();
            }
            // Delete 50 short ranges
            for i in (0..50).rev() {
                let line = i * 10;
                buf.delete(Range {
                    start: Position::new(line, 0),
                    end: Position::new(line + 1, 0),
                })
                .unwrap();
            }
            buf
        });
    });
}

// ── Groups & main ────────────────────────────────────────────────────────────

criterion_group!(
    benches,
    bench_extract_symbols,
    bench_search_symbols,
    bench_diff_generate,
    bench_buffer_insert_delete,
);

criterion_main!(benches);
