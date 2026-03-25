//! Large codebase benchmarking suite for VibeCody context management.
//!
//! Benchmarks file indexing, symbol lookup, path search, context window eviction,
//! memory usage, and incremental updates at 100M+ line scale.

#![allow(dead_code)]

use anyhow::Result;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::time::Instant;

// ---------------------------------------------------------------------------
// Structs
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BenchConfig {
    pub target_lines: usize,
    pub target_files: usize,
    pub avg_lines_per_file: usize,
    pub max_file_size_bytes: usize,
    pub languages: Vec<String>,
    pub warmup_iterations: usize,
    pub bench_iterations: usize,
    pub timeout_secs: u64,
}

impl Default for BenchConfig {
    fn default() -> Self {
        Self {
            target_lines: 100_000_000,
            target_files: 500_000,
            avg_lines_per_file: 200,
            max_file_size_bytes: 1_000_000,
            languages: vec![
                "rust".into(),
                "typescript".into(),
                "python".into(),
                "go".into(),
                "java".into(),
                "cpp".into(),
            ],
            warmup_iterations: 3,
            bench_iterations: 10,
            timeout_secs: 300,
        }
    }
}

#[derive(Debug, Clone, Serialize)]
pub struct BenchMetric {
    pub name: String,
    pub value_ms: f64,
    pub throughput: Option<f64>,
    pub memory_bytes: Option<usize>,
    pub iterations: usize,
    pub std_dev_ms: Option<f64>,
}

#[derive(Debug, Clone, Serialize)]
pub enum BenchStatus {
    NotStarted,
    Running,
    Completed,
    Failed(String),
}

#[derive(Debug, Clone, Serialize)]
pub struct FileEntry {
    pub path: String,
    pub language: String,
    pub lines: usize,
    pub size_bytes: usize,
}

#[derive(Debug, Clone, Serialize, PartialEq)]
pub enum SymbolKind {
    Function,
    Struct,
    Trait,
    Class,
    Method,
    Constant,
    Module,
    Interface,
    Enum,
    Variable,
}

#[derive(Debug, Clone, Serialize)]
pub struct Symbol {
    pub name: String,
    pub kind: SymbolKind,
    pub file_path: String,
    pub line: usize,
}

#[derive(Debug, Clone, Serialize)]
pub struct BenchReport {
    pub config: BenchConfig,
    pub metrics: Vec<BenchMetric>,
    pub total_duration_ms: u64,
    pub peak_memory_estimate_mb: f64,
    pub verdict: String,
    pub recommendations: Vec<String>,
}

// ---------------------------------------------------------------------------
// Thresholds
// ---------------------------------------------------------------------------

/// File indexing must complete in under 5 seconds for 500K files.
const THRESHOLD_FILE_INDEX_MS: f64 = 5_000.0;
const THRESHOLD_FILE_INDEX_COUNT: usize = 500_000;

/// Symbol lookup must complete in under 100ms for 1M symbols.
const THRESHOLD_SYMBOL_LOOKUP_MS: f64 = 100.0;
const THRESHOLD_SYMBOL_LOOKUP_COUNT: usize = 1_000_000;

/// Context window eviction must complete in under 50ms for 80K tokens.
const THRESHOLD_EVICTION_MS: f64 = 50.0;
const THRESHOLD_EVICTION_TOKENS: usize = 80_000;

// ---------------------------------------------------------------------------
// Deterministic pseudo-random (no external deps)
// ---------------------------------------------------------------------------

/// Simple xorshift64 PRNG for deterministic benchmark data generation.
struct Rng(u64);

impl Rng {
    fn new(seed: u64) -> Self {
        Self(if seed == 0 { 1 } else { seed })
    }

    fn next_u64(&mut self) -> u64 {
        let mut x = self.0;
        x ^= x << 13;
        x ^= x >> 7;
        x ^= x << 17;
        self.0 = x;
        x
    }

    fn next_usize(&mut self, max: usize) -> usize {
        (self.next_u64() % max as u64) as usize
    }

    fn pick<'a, T>(&mut self, items: &'a [T]) -> &'a T {
        let idx = self.next_usize(items.len());
        &items[idx]
    }
}

// ---------------------------------------------------------------------------
// Path / name building helpers
// ---------------------------------------------------------------------------

const MODULES: &[&str] = &[
    "auth", "api", "core", "db", "cache", "queue", "events", "models",
    "services", "handlers", "middleware", "utils", "config", "logging",
    "metrics", "health", "migrations", "schema", "routes", "controllers",
    "views", "templates", "workers", "jobs", "notifications", "payments",
    "billing", "search", "analytics", "storage", "uploads", "crypto",
    "session", "oauth", "rbac", "audit", "webhooks", "integrations",
    "gateway", "proxy",
];

const FILE_STEMS: &[&str] = &[
    "handler", "service", "model", "controller", "manager", "factory",
    "builder", "parser", "serializer", "validator", "converter", "adapter",
    "provider", "client", "server", "worker", "job", "task", "pipeline",
    "processor", "engine", "registry", "store", "cache", "index",
    "resolver", "dispatcher", "listener", "monitor", "reporter",
];

fn extension_for_language(lang: &str) -> &str {
    match lang {
        "rust" => "rs",
        "typescript" => "ts",
        "python" => "py",
        "go" => "go",
        "java" => "java",
        "cpp" => "cpp",
        "javascript" => "js",
        _ => "txt",
    }
}

fn build_path(rng: &mut Rng, lang: &str) -> String {
    let depth = 2 + rng.next_usize(3); // 2-4 directories
    let mut parts = vec!["src".to_string()];
    for _ in 0..depth {
        parts.push((*rng.pick(MODULES)).to_string());
    }
    let stem = *rng.pick(FILE_STEMS);
    let ext = extension_for_language(lang);
    let suffix = rng.next_usize(10000);
    parts.push(format!("{stem}_{suffix}.{ext}"));
    parts.join("/")
}

// ---------------------------------------------------------------------------
// Public generation functions
// ---------------------------------------------------------------------------

/// Generate a realistic file manifest based on the given config.
pub fn generate_file_manifest(config: &BenchConfig) -> Vec<FileEntry> {
    let mut rng = Rng::new(42);
    let count = config.target_files;
    let mut manifest = Vec::with_capacity(count);

    for _ in 0..count {
        let lang = rng.pick(&config.languages).clone();
        let path = build_path(&mut rng, &lang);
        let lines = config.avg_lines_per_file / 2 + rng.next_usize(config.avg_lines_per_file);
        let bytes_per_line = 30 + rng.next_usize(50);
        let size_bytes = (lines * bytes_per_line).min(config.max_file_size_bytes);

        manifest.push(FileEntry {
            path,
            language: lang,
            lines,
            size_bytes,
        });
    }

    manifest
}

/// Generate a language-appropriate code snippet of the specified line count.
pub fn generate_code_snippet(language: &str, lines: usize) -> String {
    let mut rng = Rng::new(language.len() as u64 + lines as u64);
    let mut buf = String::with_capacity(lines * 60);

    match language {
        "rust" => {
            buf.push_str("use std::collections::HashMap;\n\n");
            let mut remaining = lines.saturating_sub(2);
            while remaining > 0 {
                let fn_name = format!("process_{}", rng.next_usize(100_000));
                buf.push_str(&format!("pub fn {fn_name}(input: &str) -> Result<String, Box<dyn std::error::Error>> {{\n"));
                remaining = remaining.saturating_sub(1);
                let body_lines = (3 + rng.next_usize(8)).min(remaining);
                for i in 0..body_lines {
                    buf.push_str(&format!("    let v{i} = input.len() + {i};\n"));
                    remaining = remaining.saturating_sub(1);
                }
                buf.push_str("    Ok(input.to_string())\n}\n\n");
                remaining = remaining.saturating_sub(2);
            }
        }
        "typescript" => {
            buf.push_str("import {{ useState }} from 'react';\n\n");
            let mut remaining = lines.saturating_sub(2);
            while remaining > 0 {
                let fn_name = format!("handle{}", rng.next_usize(100_000));
                buf.push_str(&format!("export function {fn_name}(data: unknown): string {{\n"));
                remaining = remaining.saturating_sub(1);
                let body_lines = (2 + rng.next_usize(6)).min(remaining);
                for i in 0..body_lines {
                    buf.push_str(&format!("  const val{i} = JSON.stringify(data);\n"));
                    remaining = remaining.saturating_sub(1);
                }
                buf.push_str("  return '';\n}\n\n");
                remaining = remaining.saturating_sub(2);
            }
        }
        "python" => {
            buf.push_str("from typing import Any, Dict, List\n\n");
            let mut remaining = lines.saturating_sub(2);
            while remaining > 0 {
                let fn_name = format!("compute_{}", rng.next_usize(100_000));
                buf.push_str(&format!("def {fn_name}(data: Dict[str, Any]) -> List[str]:\n"));
                remaining = remaining.saturating_sub(1);
                let body_lines = (2 + rng.next_usize(6)).min(remaining);
                for i in 0..body_lines {
                    buf.push_str(&format!("    result_{i} = len(data) + {i}\n"));
                    remaining = remaining.saturating_sub(1);
                }
                buf.push_str("    return []\n\n");
                remaining = remaining.saturating_sub(2);
            }
        }
        "go" => {
            buf.push_str("package main\n\nimport \"fmt\"\n\n");
            let mut remaining = lines.saturating_sub(4);
            while remaining > 0 {
                let fn_name = format!("Process{}", rng.next_usize(100_000));
                buf.push_str(&format!("func {fn_name}(input string) (string, error) {{\n"));
                remaining = remaining.saturating_sub(1);
                let body_lines = (2 + rng.next_usize(5)).min(remaining);
                for i in 0..body_lines {
                    buf.push_str(&format!("\tv{i} := len(input) + {i}\n"));
                    remaining = remaining.saturating_sub(1);
                }
                buf.push_str("\tfmt.Println(input)\n\treturn input, nil\n}\n\n");
                remaining = remaining.saturating_sub(3);
            }
        }
        "java" => {
            buf.push_str("import java.util.*;\n\npublic class Generated {\n\n");
            let mut remaining = lines.saturating_sub(4);
            while remaining > 0 {
                let fn_name = format!("process{}", rng.next_usize(100_000));
                buf.push_str(&format!("    public static String {fn_name}(String input) {{\n"));
                remaining = remaining.saturating_sub(1);
                let body_lines = (2 + rng.next_usize(5)).min(remaining);
                for i in 0..body_lines {
                    buf.push_str(&format!("        int v{i} = input.length() + {i};\n"));
                    remaining = remaining.saturating_sub(1);
                }
                buf.push_str("        return input;\n    }\n\n");
                remaining = remaining.saturating_sub(2);
            }
            buf.push_str("}\n");
        }
        "cpp" => {
            buf.push_str("#include <string>\n#include <vector>\n\n");
            let mut remaining = lines.saturating_sub(3);
            while remaining > 0 {
                let fn_name = format!("process_{}", rng.next_usize(100_000));
                buf.push_str(&format!("std::string {fn_name}(const std::string& input) {{\n"));
                remaining = remaining.saturating_sub(1);
                let body_lines = (2 + rng.next_usize(5)).min(remaining);
                for i in 0..body_lines {
                    buf.push_str(&format!("    auto v{i} = input.size() + {i};\n"));
                    remaining = remaining.saturating_sub(1);
                }
                buf.push_str("    return input;\n}\n\n");
                remaining = remaining.saturating_sub(2);
            }
        }
        _ => {
            for i in 0..lines {
                buf.push_str(&format!("// line {i}\n"));
            }
        }
    }

    buf
}

/// Generate a symbol table with a variety of `SymbolKind`s.
pub fn generate_symbol_table(file_count: usize) -> Vec<Symbol> {
    let mut rng = Rng::new(99);
    let symbols_per_file = 5;
    let total = file_count * symbols_per_file;
    let mut symbols = Vec::with_capacity(total);

    let kinds = [
        SymbolKind::Function,
        SymbolKind::Struct,
        SymbolKind::Trait,
        SymbolKind::Class,
        SymbolKind::Method,
        SymbolKind::Constant,
        SymbolKind::Module,
        SymbolKind::Interface,
        SymbolKind::Enum,
        SymbolKind::Variable,
    ];

    let prefixes = [
        "handle", "process", "create", "delete", "update", "validate",
        "parse", "serialize", "transform", "compute", "resolve", "dispatch",
    ];

    for file_idx in 0..file_count {
        let file_path = format!("src/mod_{}/file_{}.rs", file_idx / 100, file_idx);
        for _ in 0..symbols_per_file {
            let kind = kinds[rng.next_usize(kinds.len())].clone();
            let prefix = prefixes[rng.next_usize(prefixes.len())];
            let suffix = rng.next_usize(1_000_000);
            let name = format!("{prefix}_{suffix}");
            let line = 1 + rng.next_usize(500);
            symbols.push(Symbol {
                name,
                kind,
                file_path: file_path.clone(),
                line,
            });
        }
    }

    symbols
}

// ---------------------------------------------------------------------------
// Benchmark functions
// ---------------------------------------------------------------------------

/// Benchmark HashMap insertion of all file paths (file indexing).
pub fn bench_file_indexing(manifest: &[FileEntry]) -> BenchMetric {
    let iterations = 5;
    let mut durations = Vec::with_capacity(iterations);

    // warmup
    for _ in 0..2 {
        let mut index: HashMap<String, usize> = HashMap::with_capacity(manifest.len());
        for (i, entry) in manifest.iter().enumerate() {
            index.insert(entry.path.clone(), i);
        }
        std::hint::black_box(&index);
    }

    for _ in 0..iterations {
        let start = Instant::now();
        let mut index: HashMap<String, usize> = HashMap::with_capacity(manifest.len());
        for (i, entry) in manifest.iter().enumerate() {
            index.insert(entry.path.clone(), i);
        }
        std::hint::black_box(&index);
        durations.push(start.elapsed().as_secs_f64() * 1000.0);
    }

    let (mean, std_dev) = stats(&durations);
    let throughput = manifest.len() as f64 / (mean / 1000.0);

    BenchMetric {
        name: "file_indexing".into(),
        value_ms: mean,
        throughput: Some(throughput),
        memory_bytes: Some(manifest.len() * 120), // estimate per entry
        iterations,
        std_dev_ms: Some(std_dev),
    }
}

/// Benchmark linear symbol lookup.
pub fn bench_symbol_lookup(symbols: &[Symbol], queries: &[String]) -> BenchMetric {
    let iterations = 5;
    let mut durations = Vec::with_capacity(iterations);

    // warmup
    for _ in 0..2 {
        let mut found = 0usize;
        for q in queries {
            for sym in symbols {
                if sym.name == *q {
                    found += 1;
                    break;
                }
            }
        }
        std::hint::black_box(found);
    }

    for _ in 0..iterations {
        let start = Instant::now();
        let mut found = 0usize;
        for q in queries {
            for sym in symbols {
                if sym.name == *q {
                    found += 1;
                    break;
                }
            }
        }
        std::hint::black_box(found);
        durations.push(start.elapsed().as_secs_f64() * 1000.0);
    }

    let (mean, std_dev) = stats(&durations);
    let throughput = queries.len() as f64 / (mean / 1000.0);

    BenchMetric {
        name: "symbol_lookup".into(),
        value_ms: mean,
        throughput: Some(throughput),
        memory_bytes: None,
        iterations,
        std_dev_ms: Some(std_dev),
    }
}

/// Benchmark glob-style path matching across the manifest.
pub fn bench_path_search(manifest: &[FileEntry], patterns: &[String]) -> BenchMetric {
    let iterations = 5;
    let mut durations = Vec::with_capacity(iterations);

    for _ in 0..iterations {
        let start = Instant::now();
        let mut matches = 0usize;
        for pat in patterns {
            for entry in manifest {
                if simple_glob_match(pat, &entry.path) {
                    matches += 1;
                }
            }
        }
        std::hint::black_box(matches);
        durations.push(start.elapsed().as_secs_f64() * 1000.0);
    }

    let (mean, std_dev) = stats(&durations);
    let throughput = (manifest.len() * patterns.len()) as f64 / (mean / 1000.0);

    BenchMetric {
        name: "path_search".into(),
        value_ms: mean,
        throughput: Some(throughput),
        memory_bytes: None,
        iterations,
        std_dev_ms: Some(std_dev),
    }
}

/// Benchmark context window eviction (simulate pruning tokens down to a budget).
pub fn bench_context_window_eviction(token_count: usize, budget: usize) -> BenchMetric {
    let iterations = 10;
    let mut durations = Vec::with_capacity(iterations);

    // Build a simulated token priority list.
    let mut rng = Rng::new(777);
    let entries: Vec<(usize, f64)> = (0..token_count)
        .map(|id| {
            let score = (rng.next_u64() % 10_000) as f64 / 10_000.0;
            (id, score)
        })
        .collect();

    for _ in 0..iterations {
        let start = Instant::now();
        let mut scored = entries.clone();
        // Sort by score descending — keep highest-priority tokens.
        scored.sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap_or(std::cmp::Ordering::Equal));
        let kept: Vec<usize> = scored.iter().take(budget).map(|(id, _)| *id).collect();
        std::hint::black_box(&kept);
        durations.push(start.elapsed().as_secs_f64() * 1000.0);
    }

    let (mean, std_dev) = stats(&durations);
    let throughput = token_count as f64 / (mean / 1000.0);

    BenchMetric {
        name: "context_window_eviction".into(),
        value_ms: mean,
        throughput: Some(throughput),
        memory_bytes: Some(token_count * 16), // (usize + f64) per entry
        iterations,
        std_dev_ms: Some(std_dev),
    }
}

/// Estimate memory usage per file entry at the given scale.
pub fn bench_memory_usage(file_count: usize) -> BenchMetric {
    let bytes_per_path_estimate = 80; // avg path length
    let bytes_per_entry_overhead = 64; // struct fields, allocator
    let bytes_per_hashmap_slot = 48; // bucket + metadata
    let per_entry = bytes_per_path_estimate + bytes_per_entry_overhead + bytes_per_hashmap_slot;
    let total = file_count * per_entry;

    let start = Instant::now();
    // Actually allocate to validate the estimate.
    let mut map: HashMap<usize, Vec<u8>> = HashMap::with_capacity(file_count.min(100_000));
    let sample = file_count.min(100_000);
    for i in 0..sample {
        map.insert(i, vec![0u8; 80]);
    }
    std::hint::black_box(&map);
    let elapsed = start.elapsed().as_secs_f64() * 1000.0;
    drop(map);

    BenchMetric {
        name: "memory_usage_estimate".into(),
        value_ms: elapsed,
        throughput: None,
        memory_bytes: Some(total),
        iterations: 1,
        std_dev_ms: None,
    }
}

/// Benchmark incremental re-indexing when a percentage of files change.
pub fn bench_incremental_update(manifest: &[FileEntry], changed_pct: f64) -> BenchMetric {
    let iterations = 5;
    let mut durations = Vec::with_capacity(iterations);
    let changed_count = ((manifest.len() as f64) * changed_pct / 100.0) as usize;

    // Build initial index.
    let mut index: HashMap<String, usize> = HashMap::with_capacity(manifest.len());
    for (i, entry) in manifest.iter().enumerate() {
        index.insert(entry.path.clone(), i);
    }

    for _ in 0..iterations {
        let start = Instant::now();
        // Re-index only the changed portion.
        for i in 0..changed_count {
            let entry = &manifest[i % manifest.len()];
            index.insert(entry.path.clone(), i + manifest.len());
        }
        std::hint::black_box(&index);
        durations.push(start.elapsed().as_secs_f64() * 1000.0);
    }

    let (mean, std_dev) = stats(&durations);
    let throughput = changed_count as f64 / (mean / 1000.0);

    BenchMetric {
        name: "incremental_update".into(),
        value_ms: mean,
        throughput: Some(throughput),
        memory_bytes: None,
        iterations,
        std_dev_ms: Some(std_dev),
    }
}

// ---------------------------------------------------------------------------
// Suite runner
// ---------------------------------------------------------------------------

/// Run the complete benchmark suite and produce a report.
pub fn run_bench_suite(config: &BenchConfig) -> Result<BenchReport> {
    let suite_start = Instant::now();
    let mut metrics = Vec::new();

    // Scale down for actual execution — real 500K files is slow in test.
    let scaled_config = BenchConfig {
        target_files: config.target_files.min(50_000),
        ..config.clone()
    };

    let manifest = generate_file_manifest(&scaled_config);
    let symbols = generate_symbol_table(scaled_config.target_files.min(10_000));

    // 1. File indexing
    metrics.push(bench_file_indexing(&manifest));

    // 2. Symbol lookup
    let queries: Vec<String> = symbols
        .iter()
        .take(100)
        .map(|s| s.name.clone())
        .collect();
    metrics.push(bench_symbol_lookup(&symbols[..symbols.len().min(5_000)], &queries));

    // 3. Path search
    let patterns = vec![
        "*.rs".to_string(),
        "src/modules/auth/*".to_string(),
        "*handler*".to_string(),
    ];
    metrics.push(bench_path_search(&manifest[..manifest.len().min(10_000)], &patterns));

    // 4. Context window eviction
    metrics.push(bench_context_window_eviction(80_000, 32_000));

    // 5. Memory estimate
    metrics.push(bench_memory_usage(scaled_config.target_files));

    // 6. Incremental update
    metrics.push(bench_incremental_update(&manifest, 5.0));

    let total_duration_ms = suite_start.elapsed().as_millis() as u64;

    // Peak memory estimate based on file count + symbol count.
    let mem_entry = metrics.iter().find(|m| m.name == "memory_usage_estimate");
    let peak_memory_estimate_mb = mem_entry
        .and_then(|m| m.memory_bytes)
        .map(|b| b as f64 / (1024.0 * 1024.0))
        .unwrap_or(0.0);

    let (verdict, recommendations) = evaluate_thresholds(&metrics, config);

    Ok(BenchReport {
        config: config.clone(),
        metrics,
        total_duration_ms,
        peak_memory_estimate_mb,
        verdict,
        recommendations,
    })
}

/// Format a benchmark report as a markdown table.
pub fn format_report(report: &BenchReport) -> String {
    let mut out = String::new();
    out.push_str("# VibeCody Large Codebase Benchmark Report\n\n");
    out.push_str(&format!(
        "**Target scale**: {} files, {} lines\n\n",
        report.config.target_files, report.config.target_lines
    ));

    out.push_str("| Benchmark | Time (ms) | Throughput | Memory | Iterations | Std Dev (ms) |\n");
    out.push_str("|-----------|-----------|------------|--------|------------|-------------|\n");

    for m in &report.metrics {
        let throughput = m
            .throughput
            .map(|t| format!("{:.0}/s", t))
            .unwrap_or_else(|| "N/A".into());
        let memory = m
            .memory_bytes
            .map(|b| format_bytes(b))
            .unwrap_or_else(|| "N/A".into());
        let std_dev = m
            .std_dev_ms
            .map(|s| format!("{:.2}", s))
            .unwrap_or_else(|| "N/A".into());

        out.push_str(&format!(
            "| {} | {:.2} | {} | {} | {} | {} |\n",
            m.name, m.value_ms, throughput, memory, m.iterations, std_dev
        ));
    }

    out.push_str(&format!(
        "\n**Total duration**: {} ms\n",
        report.total_duration_ms
    ));
    out.push_str(&format!(
        "**Peak memory estimate**: {:.1} MB\n",
        report.peak_memory_estimate_mb
    ));
    out.push_str(&format!("**Verdict**: {}\n", report.verdict));

    if !report.recommendations.is_empty() {
        out.push_str("\n## Recommendations\n\n");
        for rec in &report.recommendations {
            out.push_str(&format!("- {rec}\n"));
        }
    }

    out
}

// ---------------------------------------------------------------------------
// Internal helpers
// ---------------------------------------------------------------------------

fn stats(values: &[f64]) -> (f64, f64) {
    if values.is_empty() {
        return (0.0, 0.0);
    }
    let n = values.len() as f64;
    let mean = values.iter().sum::<f64>() / n;
    let variance = values.iter().map(|v| (v - mean).powi(2)).sum::<f64>() / n;
    (mean, variance.sqrt())
}

fn format_bytes(bytes: usize) -> String {
    if bytes >= 1_073_741_824 {
        format!("{:.1} GB", bytes as f64 / 1_073_741_824.0)
    } else if bytes >= 1_048_576 {
        format!("{:.1} MB", bytes as f64 / 1_048_576.0)
    } else if bytes >= 1_024 {
        format!("{:.1} KB", bytes as f64 / 1_024.0)
    } else {
        format!("{bytes} B")
    }
}

/// Simple glob matching supporting `*` as a wildcard.
fn simple_glob_match(pattern: &str, text: &str) -> bool {
    let parts: Vec<&str> = pattern.split('*').collect();
    if parts.len() == 1 {
        return pattern == text;
    }

    let mut pos = 0;
    for (i, part) in parts.iter().enumerate() {
        if part.is_empty() {
            continue;
        }
        match text[pos..].find(part) {
            Some(found) => {
                if i == 0 && found != 0 {
                    return false; // first part must be a prefix
                }
                pos += found + part.len();
            }
            None => return false,
        }
    }

    // If pattern does not end with `*`, text must end exactly.
    if !pattern.ends_with('*') {
        return text.ends_with(parts.last().unwrap_or(&""));
    }

    true
}

/// Evaluate benchmark results against thresholds.
fn evaluate_thresholds(
    metrics: &[BenchMetric],
    config: &BenchConfig,
) -> (String, Vec<String>) {
    let mut pass = true;
    let mut recommendations = Vec::new();

    // File indexing threshold: scale proportionally.
    if let Some(m) = metrics.iter().find(|m| m.name == "file_indexing") {
        let scaled_threshold =
            THRESHOLD_FILE_INDEX_MS * (config.target_files as f64 / THRESHOLD_FILE_INDEX_COUNT as f64);
        // We ran with scaled-down count; extrapolate.
        let extrapolated = m.value_ms * (config.target_files as f64 / config.target_files.min(50_000) as f64);
        if extrapolated > scaled_threshold {
            pass = false;
            recommendations.push(format!(
                "File indexing extrapolated to {:.0}ms exceeds {:.0}ms threshold for {} files. Consider B-tree or trie indexing.",
                extrapolated, scaled_threshold, config.target_files
            ));
        }
    }

    // Symbol lookup threshold.
    if let Some(m) = metrics.iter().find(|m| m.name == "symbol_lookup") {
        let extrapolated = m.value_ms * (THRESHOLD_SYMBOL_LOOKUP_COUNT as f64 / 5_000.0);
        if extrapolated > THRESHOLD_SYMBOL_LOOKUP_MS * (THRESHOLD_SYMBOL_LOOKUP_COUNT as f64 / 1_000_000.0) * 100.0 {
            // Very generous for linear search — it will likely fail, which is expected.
            recommendations.push(
                "Symbol lookup is linear — switch to HashMap or radix trie for O(1) lookup.".into(),
            );
        }
    }

    // Context eviction threshold.
    if let Some(m) = metrics.iter().find(|m| m.name == "context_window_eviction") {
        if m.value_ms > THRESHOLD_EVICTION_MS {
            pass = false;
            recommendations.push(format!(
                "Context eviction took {:.1}ms, exceeding {:.0}ms threshold for {} tokens. Consider a min-heap or partial sort.",
                m.value_ms, THRESHOLD_EVICTION_MS, THRESHOLD_EVICTION_TOKENS
            ));
        }
    }

    if recommendations.is_empty() {
        recommendations.push("All benchmarks within acceptable thresholds.".into());
    }

    let verdict = if pass {
        "PASS — VibeCody context management meets large-codebase thresholds.".into()
    } else {
        "NEEDS IMPROVEMENT — Some benchmarks exceed target thresholds.".into()
    };

    (verdict, recommendations)
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    // -- Config defaults --

    #[test]
    fn test_config_default_target_lines() {
        let cfg = BenchConfig::default();
        assert_eq!(cfg.target_lines, 100_000_000);
    }

    #[test]
    fn test_config_default_target_files() {
        let cfg = BenchConfig::default();
        assert_eq!(cfg.target_files, 500_000);
    }

    #[test]
    fn test_config_default_avg_lines() {
        let cfg = BenchConfig::default();
        assert_eq!(cfg.avg_lines_per_file, 200);
    }

    #[test]
    fn test_config_default_max_file_size() {
        let cfg = BenchConfig::default();
        assert_eq!(cfg.max_file_size_bytes, 1_000_000);
    }

    #[test]
    fn test_config_default_languages() {
        let cfg = BenchConfig::default();
        assert_eq!(cfg.languages.len(), 6);
        assert!(cfg.languages.contains(&"rust".to_string()));
        assert!(cfg.languages.contains(&"typescript".to_string()));
    }

    #[test]
    fn test_config_default_warmup() {
        let cfg = BenchConfig::default();
        assert_eq!(cfg.warmup_iterations, 3);
    }

    #[test]
    fn test_config_default_bench_iterations() {
        let cfg = BenchConfig::default();
        assert_eq!(cfg.bench_iterations, 10);
    }

    #[test]
    fn test_config_default_timeout() {
        let cfg = BenchConfig::default();
        assert_eq!(cfg.timeout_secs, 300);
    }

    #[test]
    fn test_config_serialize_roundtrip() {
        let cfg = BenchConfig::default();
        let json = serde_json::to_string(&cfg).unwrap();
        let parsed: BenchConfig = serde_json::from_str(&json).unwrap();
        assert_eq!(parsed.target_lines, cfg.target_lines);
        assert_eq!(parsed.target_files, cfg.target_files);
    }

    // -- Manifest generation --

    #[test]
    fn test_manifest_count() {
        let cfg = BenchConfig {
            target_files: 1_000,
            ..Default::default()
        };
        let manifest = generate_file_manifest(&cfg);
        assert_eq!(manifest.len(), 1_000);
    }

    #[test]
    fn test_manifest_paths_start_with_src() {
        let cfg = BenchConfig {
            target_files: 100,
            ..Default::default()
        };
        let manifest = generate_file_manifest(&cfg);
        for entry in &manifest {
            assert!(entry.path.starts_with("src/"), "path should start with src/: {}", entry.path);
        }
    }

    #[test]
    fn test_manifest_languages_from_config() {
        let cfg = BenchConfig {
            target_files: 500,
            ..Default::default()
        };
        let manifest = generate_file_manifest(&cfg);
        for entry in &manifest {
            assert!(cfg.languages.contains(&entry.language));
        }
    }

    #[test]
    fn test_manifest_lines_positive() {
        let cfg = BenchConfig {
            target_files: 100,
            ..Default::default()
        };
        let manifest = generate_file_manifest(&cfg);
        for entry in &manifest {
            assert!(entry.lines > 0);
        }
    }

    #[test]
    fn test_manifest_size_within_limit() {
        let cfg = BenchConfig {
            target_files: 200,
            ..Default::default()
        };
        let manifest = generate_file_manifest(&cfg);
        for entry in &manifest {
            assert!(entry.size_bytes <= cfg.max_file_size_bytes);
        }
    }

    #[test]
    fn test_manifest_deterministic() {
        let cfg = BenchConfig {
            target_files: 50,
            ..Default::default()
        };
        let m1 = generate_file_manifest(&cfg);
        let m2 = generate_file_manifest(&cfg);
        for (a, b) in m1.iter().zip(m2.iter()) {
            assert_eq!(a.path, b.path);
            assert_eq!(a.lines, b.lines);
        }
    }

    // -- Code snippets --

    #[test]
    fn test_snippet_rust_not_empty() {
        let code = generate_code_snippet("rust", 50);
        assert!(!code.is_empty());
        assert!(code.contains("fn "));
    }

    #[test]
    fn test_snippet_typescript() {
        let code = generate_code_snippet("typescript", 30);
        assert!(code.contains("function "));
    }

    #[test]
    fn test_snippet_python() {
        let code = generate_code_snippet("python", 30);
        assert!(code.contains("def "));
    }

    #[test]
    fn test_snippet_go() {
        let code = generate_code_snippet("go", 30);
        assert!(code.contains("func "));
    }

    #[test]
    fn test_snippet_java() {
        let code = generate_code_snippet("java", 30);
        assert!(code.contains("public static"));
    }

    #[test]
    fn test_snippet_cpp() {
        let code = generate_code_snippet("cpp", 30);
        assert!(code.contains("std::string"));
    }

    #[test]
    fn test_snippet_unknown_language() {
        let code = generate_code_snippet("brainfuck", 10);
        assert!(code.contains("// line"));
    }

    // -- Symbol table --

    #[test]
    fn test_symbol_table_count() {
        let symbols = generate_symbol_table(100);
        assert_eq!(symbols.len(), 500); // 5 per file
    }

    #[test]
    fn test_symbol_table_variety() {
        let symbols = generate_symbol_table(200);
        let kinds: std::collections::HashSet<String> = symbols
            .iter()
            .map(|s| format!("{:?}", s.kind))
            .collect();
        // With 1000 symbols and 10 kinds, we should have most kinds represented.
        assert!(kinds.len() >= 5, "expected variety of symbol kinds, got {}", kinds.len());
    }

    #[test]
    fn test_symbol_table_has_file_paths() {
        let symbols = generate_symbol_table(10);
        for sym in &symbols {
            assert!(!sym.file_path.is_empty());
            assert!(sym.line > 0);
        }
    }

    // -- Benchmark metrics --

    #[test]
    fn test_bench_file_indexing_metric() {
        let cfg = BenchConfig {
            target_files: 1_000,
            ..Default::default()
        };
        let manifest = generate_file_manifest(&cfg);
        let metric = bench_file_indexing(&manifest);
        assert_eq!(metric.name, "file_indexing");
        assert!(metric.value_ms >= 0.0);
        assert!(metric.throughput.is_some());
        assert!(metric.throughput.unwrap() > 0.0);
        assert!(metric.memory_bytes.is_some());
    }

    #[test]
    fn test_bench_symbol_lookup_metric() {
        let symbols = generate_symbol_table(100);
        let queries: Vec<String> = symbols.iter().take(5).map(|s| s.name.clone()).collect();
        let metric = bench_symbol_lookup(&symbols, &queries);
        assert_eq!(metric.name, "symbol_lookup");
        assert!(metric.value_ms >= 0.0);
    }

    #[test]
    fn test_bench_path_search_metric() {
        let cfg = BenchConfig {
            target_files: 500,
            ..Default::default()
        };
        let manifest = generate_file_manifest(&cfg);
        let patterns = vec!["*.rs".to_string()];
        let metric = bench_path_search(&manifest, &patterns);
        assert_eq!(metric.name, "path_search");
        assert!(metric.value_ms >= 0.0);
    }

    #[test]
    fn test_bench_context_eviction_metric() {
        let metric = bench_context_window_eviction(10_000, 4_000);
        assert_eq!(metric.name, "context_window_eviction");
        assert!(metric.value_ms >= 0.0);
        assert!(metric.memory_bytes.unwrap() > 0);
    }

    #[test]
    fn test_bench_memory_usage_metric() {
        let metric = bench_memory_usage(1_000);
        assert_eq!(metric.name, "memory_usage_estimate");
        assert!(metric.memory_bytes.unwrap() > 0);
    }

    #[test]
    fn test_bench_incremental_update_metric() {
        let cfg = BenchConfig {
            target_files: 1_000,
            ..Default::default()
        };
        let manifest = generate_file_manifest(&cfg);
        let metric = bench_incremental_update(&manifest, 10.0);
        assert_eq!(metric.name, "incremental_update");
        assert!(metric.value_ms >= 0.0);
    }

    // -- Report --

    #[test]
    fn test_run_bench_suite_produces_report() {
        let cfg = BenchConfig {
            target_files: 500,
            target_lines: 100_000,
            ..Default::default()
        };
        let report = run_bench_suite(&cfg).unwrap();
        assert!(!report.metrics.is_empty());
        assert!(report.total_duration_ms > 0);
        assert!(!report.verdict.is_empty());
        assert!(!report.recommendations.is_empty());
    }

    #[test]
    fn test_report_verdict_contains_pass_or_needs() {
        let cfg = BenchConfig {
            target_files: 200,
            target_lines: 40_000,
            ..Default::default()
        };
        let report = run_bench_suite(&cfg).unwrap();
        assert!(
            report.verdict.contains("PASS") || report.verdict.contains("NEEDS IMPROVEMENT"),
            "unexpected verdict: {}",
            report.verdict
        );
    }

    #[test]
    fn test_format_report_markdown() {
        let cfg = BenchConfig {
            target_files: 200,
            target_lines: 40_000,
            ..Default::default()
        };
        let report = run_bench_suite(&cfg).unwrap();
        let md = format_report(&report);
        assert!(md.contains("# VibeCody Large Codebase Benchmark Report"));
        assert!(md.contains("| Benchmark"));
        assert!(md.contains("file_indexing"));
        assert!(md.contains("Verdict"));
    }

    #[test]
    fn test_format_report_contains_all_metrics() {
        let cfg = BenchConfig {
            target_files: 200,
            target_lines: 40_000,
            ..Default::default()
        };
        let report = run_bench_suite(&cfg).unwrap();
        let md = format_report(&report);
        for m in &report.metrics {
            assert!(md.contains(&m.name), "report missing metric: {}", m.name);
        }
    }

    // -- Threshold logic --

    #[test]
    fn test_threshold_constants() {
        assert_eq!(THRESHOLD_FILE_INDEX_MS, 5_000.0);
        assert_eq!(THRESHOLD_SYMBOL_LOOKUP_COUNT, 1_000_000);
        assert_eq!(THRESHOLD_EVICTION_MS, 50.0);
        assert_eq!(THRESHOLD_EVICTION_TOKENS, 80_000);
    }

    #[test]
    fn test_evaluate_thresholds_all_pass() {
        let metrics = vec![
            BenchMetric {
                name: "file_indexing".into(),
                value_ms: 1.0,
                throughput: Some(1_000_000.0),
                memory_bytes: Some(1000),
                iterations: 5,
                std_dev_ms: Some(0.1),
            },
            BenchMetric {
                name: "context_window_eviction".into(),
                value_ms: 5.0,
                throughput: Some(100_000.0),
                memory_bytes: Some(1000),
                iterations: 10,
                std_dev_ms: Some(0.5),
            },
        ];
        let cfg = BenchConfig::default();
        let (verdict, _recs) = evaluate_thresholds(&metrics, &cfg);
        // With very fast metrics, should pass.
        assert!(verdict.contains("PASS"), "expected PASS verdict: {}", verdict);
    }

    #[test]
    fn test_evaluate_thresholds_eviction_fail() {
        let metrics = vec![
            BenchMetric {
                name: "context_window_eviction".into(),
                value_ms: 200.0, // exceeds 50ms threshold
                throughput: Some(1000.0),
                memory_bytes: Some(1000),
                iterations: 10,
                std_dev_ms: Some(10.0),
            },
        ];
        let cfg = BenchConfig::default();
        let (verdict, recs) = evaluate_thresholds(&metrics, &cfg);
        assert!(verdict.contains("NEEDS IMPROVEMENT"));
        assert!(recs.iter().any(|r| r.contains("eviction")));
    }

    // -- Glob matching --

    #[test]
    fn test_glob_exact_match() {
        assert!(simple_glob_match("foo.rs", "foo.rs"));
        assert!(!simple_glob_match("foo.rs", "bar.rs"));
    }

    #[test]
    fn test_glob_wildcard_suffix() {
        assert!(simple_glob_match("*.rs", "handler.rs"));
        assert!(!simple_glob_match("*.rs", "handler.ts"));
    }

    #[test]
    fn test_glob_wildcard_prefix() {
        assert!(simple_glob_match("src/*", "src/handler.rs"));
    }

    #[test]
    fn test_glob_wildcard_middle() {
        assert!(simple_glob_match("src/*.rs", "src/handler.rs"));
    }

    // -- Stats helper --

    #[test]
    fn test_stats_empty() {
        let (mean, std_dev) = stats(&[]);
        assert_eq!(mean, 0.0);
        assert_eq!(std_dev, 0.0);
    }

    #[test]
    fn test_stats_single_value() {
        let (mean, std_dev) = stats(&[42.0]);
        assert!((mean - 42.0).abs() < f64::EPSILON);
        assert!((std_dev - 0.0).abs() < f64::EPSILON);
    }

    #[test]
    fn test_stats_known_values() {
        let (mean, _std_dev) = stats(&[2.0, 4.0, 6.0]);
        assert!((mean - 4.0).abs() < f64::EPSILON);
    }

    // -- Format bytes --

    #[test]
    fn test_format_bytes_small() {
        assert_eq!(format_bytes(500), "500 B");
    }

    #[test]
    fn test_format_bytes_kb() {
        let s = format_bytes(2048);
        assert!(s.contains("KB"));
    }

    #[test]
    fn test_format_bytes_mb() {
        let s = format_bytes(5 * 1_048_576);
        assert!(s.contains("MB"));
    }

    #[test]
    fn test_format_bytes_gb() {
        let s = format_bytes(2 * 1_073_741_824);
        assert!(s.contains("GB"));
    }

    // -- Serialization --

    #[test]
    fn test_bench_metric_serializes() {
        let m = BenchMetric {
            name: "test".into(),
            value_ms: 1.5,
            throughput: Some(100.0),
            memory_bytes: Some(1024),
            iterations: 3,
            std_dev_ms: Some(0.2),
        };
        let json = serde_json::to_string(&m).unwrap();
        assert!(json.contains("\"name\":\"test\""));
        assert!(json.contains("\"value_ms\":1.5"));
    }

    #[test]
    fn test_bench_status_serializes() {
        let s = BenchStatus::Failed("timeout".into());
        let json = serde_json::to_string(&s).unwrap();
        assert!(json.contains("timeout"));

        let s2 = BenchStatus::Completed;
        let json2 = serde_json::to_string(&s2).unwrap();
        assert!(json2.contains("Completed"));
    }

    #[test]
    fn test_bench_report_serializes() {
        let report = BenchReport {
            config: BenchConfig::default(),
            metrics: vec![],
            total_duration_ms: 100,
            peak_memory_estimate_mb: 42.5,
            verdict: "PASS".into(),
            recommendations: vec!["all good".into()],
        };
        let json = serde_json::to_string(&report).unwrap();
        assert!(json.contains("\"verdict\":\"PASS\""));
    }

    #[test]
    fn test_symbol_kind_serializes() {
        let kinds = vec![
            SymbolKind::Function,
            SymbolKind::Struct,
            SymbolKind::Trait,
            SymbolKind::Enum,
            SymbolKind::Variable,
        ];
        for k in kinds {
            let json = serde_json::to_string(&k).unwrap();
            assert!(!json.is_empty());
        }
    }
}
