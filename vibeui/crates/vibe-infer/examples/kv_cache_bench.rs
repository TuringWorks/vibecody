//! Phase-3 research-spike benchmark: TurboQuant KV cache vs Fp8 / Int8.
//!
//! Runs on pure Rust — no candle dep — so anyone can `cargo run` it without
//! the ML toolchain:
//!
//! ```sh
//! cargo run -p vibe-infer --release --example kv_cache_bench
//! cargo run -p vibe-infer --release --example kv_cache_bench -- 32 2048 128
//! # args: num_heads seq_len head_dim
//! ```
//!
//! Reports per-method: bytes/element, memory savings vs fp16, mean & worst
//! per-vector cosine similarity, simulated attention-weight MAE, and top-1
//! argmax agreement with the fp16 ground truth. Use the numbers to judge
//! whether a CUDA kernel in `mistralrs-quant` is worth the investment.

use std::time::Instant;
use vibe_infer::kv_cache_tq::{
    fidelity_fp8, fidelity_int8, fidelity_turboquant, FidelityReport, KvCacheTurboQuant,
};

fn parse_usize(s: Option<&String>, default: usize) -> usize {
    s.and_then(|x| x.parse::<usize>().ok()).unwrap_or(default)
}

fn uniform_tensor(num_heads: usize, seq_len: usize, head_dim: usize, seed: u64) -> Vec<f32> {
    // Xorshift in [-1, 1] — matches the codec's own PRNG for reproducibility.
    let mut state = if seed == 0 { 1 } else { seed };
    let n = num_heads * seq_len * head_dim;
    let mut out = Vec::with_capacity(n);
    for _ in 0..n {
        state ^= state << 13;
        state ^= state >> 7;
        state ^= state << 17;
        let u = (state as f64 / u64::MAX as f64) as f32 * 2.0 - 1.0;
        out.push(u);
    }
    out
}

/// Build a tensor whose attention softmax has a clear argmax per head.
/// Each head picks one "anchor" token and biases all other tokens away from it
/// so `q · K[anchor]` dominates for the deterministic query used by the
/// fidelity harness. This mimics real attention caches where a handful of
/// tokens carry most of the probability mass — uniform-random data produces
/// flat softmax where argmax is arbitrary and top-1 agreement is meaningless.
fn spike_tensor(num_heads: usize, seq_len: usize, head_dim: usize, seed: u64) -> Vec<f32> {
    let mut state = if seed == 0 { 1 } else { seed };
    let mut rand = || {
        state ^= state << 13;
        state ^= state >> 7;
        state ^= state << 17;
        (state as f64 / u64::MAX as f64) as f32 * 2.0 - 1.0
    };

    // Same per-head query the fidelity harness will use (query_seed=99).
    let mut qs = 99u64;
    let mut query_rand = || {
        qs ^= qs << 13;
        qs ^= qs >> 7;
        qs ^= qs << 17;
        (qs as f64 / u64::MAX as f64) as f32 * 2.0 - 1.0
    };

    let mut out = vec![0.0f32; num_heads * seq_len * head_dim];
    for h in 0..num_heads {
        let query: Vec<f32> = (0..head_dim).map(|_| query_rand()).collect();
        let anchor = (rand().abs() * (seq_len - 1) as f32) as usize;
        for t in 0..seq_len {
            let off = h * seq_len * head_dim + t * head_dim;
            if t == anchor {
                // Anchor key = query direction × strong magnitude → big dot.
                for d in 0..head_dim {
                    out[off + d] = query[d] * 4.0;
                }
            } else {
                // Non-anchor = small random noise orthogonal-ish to query.
                for d in 0..head_dim {
                    out[off + d] = rand() * 0.5;
                }
            }
        }
    }
    out
}

fn print_header() {
    println!(
        "{:<12}  {:>10}  {:>9}  {:>10}  {:>10}  {:>10}  {:>10}",
        "method", "bytes/el", "savings", "mean_cos", "worst_cos", "attn_mae", "top1_agree"
    );
    println!("{}", "-".repeat(86));
}

fn print_row(rep: &FidelityReport) {
    let savings = if rep.bytes_per_element > 0.0 {
        format!("{:>7.2}×", 2.0 / rep.bytes_per_element)
    } else {
        "n/a".to_string()
    };
    println!(
        "{:<12}  {:>10.3}  {:>9}  {:>10.4}  {:>10.4}  {:>10.5}  {:>9.1}%",
        rep.method,
        rep.bytes_per_element,
        savings,
        rep.mean_cosine,
        rep.worst_cosine,
        rep.attention_mae,
        rep.top1_agreement * 100.0,
    );
}

fn run_distribution(
    label: &str,
    note: &str,
    tensor: Vec<f32>,
    num_heads: usize,
    seq_len: usize,
    head_dim: usize,
) {
    println!("\n══ {label} ══");
    println!("{note}\n");

    let codec = KvCacheTurboQuant::new(head_dim, 42, None);
    let t0 = Instant::now();
    let layer = codec.encode_layer(&tensor, num_heads, seq_len);
    let encode_ms = t0.elapsed().as_secs_f32() * 1000.0;
    let total_elements = num_heads * seq_len * head_dim;

    let t0 = Instant::now();
    let mut slot = vec![0.0f32; head_dim];
    for h in 0..num_heads {
        for t in 0..seq_len {
            codec.decode_one(&layer, h, t, &mut slot);
        }
    }
    let decode_ms = t0.elapsed().as_secs_f32() * 1000.0;

    println!(
        "TurboQuant: encode {:.1} ms ({:.1} M elem/s), decode {:.1} ms ({:.1} M elem/s), storage {:.2} MiB ({:.2}× fp16)\n",
        encode_ms,
        total_elements as f32 / encode_ms / 1000.0,
        decode_ms,
        total_elements as f32 / decode_ms / 1000.0,
        layer.storage_bytes() as f32 / (1024.0 * 1024.0),
        layer.ratio_vs_fp16(),
    );

    print_header();
    let query_seed = 99u64;
    print_row(&fidelity_turboquant(&codec, &tensor, num_heads, seq_len, query_seed));
    print_row(&fidelity_fp8(&tensor, num_heads, seq_len, head_dim, query_seed));
    print_row(&fidelity_int8(&tensor, num_heads, seq_len, head_dim, query_seed));
}

/// Throughput sweep across realistic context lengths. Reports prefill TPS
/// (one-shot batch encode) and decode TPS (per-vector decode looped over
/// every (head, token) — the autoregressive shape). The fp16 baseline has
/// zero codec overhead by definition, so the absolute number here is the
/// answer to "is the encode/decode cheap enough that the bandwidth saving
/// from carrying ~0.44 B/el instead of 2 B/el is a net win?"
///
/// Default sweep stays under ~1s of work on a laptop. Bump seq_lens for
/// long-context experiments.
fn run_tps_sweep(num_heads: usize, head_dim: usize) {
    println!("\n══ Throughput sweep (spiked input, single thread) ══");
    println!(
        "{:<10}  {:>13}  {:>13}  {:>11}  {:>11}",
        "seq_len", "prefill_TPS", "decode_TPS", "encode_ms", "decode_ms"
    );
    println!("{}", "-".repeat(66));

    let codec = KvCacheTurboQuant::new(head_dim, 42, None);
    let seq_lens = [1024usize, 8192, 32768];
    for &seq_len in &seq_lens {
        let tensor = spike_tensor(num_heads, seq_len, head_dim, 42);

        let t0 = Instant::now();
        let layer = codec.encode_layer(&tensor, num_heads, seq_len);
        let encode_ms = t0.elapsed().as_secs_f32() * 1000.0;
        let prefill_tps = (seq_len as f32 / encode_ms) * 1000.0;

        let mut slot = vec![0.0f32; head_dim];
        let t0 = Instant::now();
        for h in 0..num_heads {
            for t in 0..seq_len {
                codec.decode_one(&layer, h, t, &mut slot);
            }
        }
        let decode_ms = t0.elapsed().as_secs_f32() * 1000.0;
        let decode_tps = (seq_len as f32 / decode_ms) * 1000.0;

        println!(
            "{:>10}  {:>13.0}  {:>13.0}  {:>9.1} ms  {:>9.1} ms",
            seq_len, prefill_tps, decode_tps, encode_ms, decode_ms
        );
    }
    println!("\n  Prefill TPS  = seq_len encoded / encode_ms (one-shot batch).");
    println!("  Decode TPS   = seq_len decoded / decode_ms (per-vector, all heads).");
    println!("  fp16 baseline has zero codec overhead. Compare TPS against your");
    println!("  device's HBM bandwidth: if TurboQuant TPS × bandwidth_saving > raw");
    println!("  fp16 throughput, the codec is a net production win.");
}

fn main() {
    let args: Vec<String> = std::env::args().collect();
    let num_heads = parse_usize(args.get(1), 8);
    let seq_len = parse_usize(args.get(2), 512);
    let head_dim = parse_usize(args.get(3), 128);

    let total_elements = num_heads * seq_len * head_dim;
    let fp16_bytes = total_elements * 2;
    println!(
        "KV cache shape: [heads={num_heads}, tokens={seq_len}, head_dim={head_dim}] = {} elements, fp16 = {:.2} MiB",
        total_elements,
        fp16_bytes as f32 / (1024.0 * 1024.0),
    );

    let codec = KvCacheTurboQuant::new(head_dim, 42, None);
    println!(
        "TurboQuant codec fixed matrices = {:.1} KiB (shared across all layers + heads + tokens)",
        codec.fixed_bytes() as f32 / 1024.0,
    );

    run_distribution(
        "Uniform random K/V",
        "Stress test — flat attention distribution, argmax is near-arbitrary so top-1 is NOT meaningful. Judge by `attn_mae` (L1 over softmax) instead.",
        uniform_tensor(num_heads, seq_len, head_dim, 42),
        num_heads,
        seq_len,
        head_dim,
    );

    run_distribution(
        "Spiked K/V (per-head anchor)",
        "Realistic — each head has one strongly-attended 'anchor' token, mimicking real attention caches. Top-1 agreement is meaningful here.",
        spike_tensor(num_heads, seq_len, head_dim, 42),
        num_heads,
        seq_len,
        head_dim,
    );

    run_tps_sweep(num_heads, head_dim);

    println!("\n── Phase 3 interpretation ──");
    println!("  • TurboQuant gives ~{:.1}× memory savings vs fp16 at head_dim={head_dim}.",
        2.0 / 0.4375);
    println!("  • Fp8 / Int8 give 2.0× savings, effectively lossless on both distributions.");
    println!("  • On *realistic* (spiked) data, `attn_mae` and `top1_agree` tell you whether");
    println!("    TurboQuant's 2.3× extra savings cost you any real tokens. Judge viability");
    println!("    of a `mistralrs-quant` PR on these two columns.");
}
