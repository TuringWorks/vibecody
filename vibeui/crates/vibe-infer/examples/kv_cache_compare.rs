//! Side-by-side Fp16 vs TurboQuant KV-cache comparison.
//!
//! Loads the same model twice — once with the upstream fp16 KV cache, once
//! with the TurboQuant codec installed on every attention layer — then
//! generates the same prompt with both and reports:
//!
//! - time-to-first-token-ish (full-response latency, we're not streaming)
//! - tokens/sec
//! - finish reason
//! - full text, so a human can eyeball drift
//!
//! Motivation: the codec's unit tests prove the install path works and the
//! per-vector reconstruction matches the Phase-3 spike on uniform noise. What
//! they can't prove is "does quality hold on a real prompt through a real
//! chat template?". This example is the human-in-the-loop answer: run it,
//! read both outputs, and decide whether the savings justify the quality
//! regression for your workload.
//!
//! ```sh
//! cargo run --release -p vibe-infer --features mistralrs-metal \
//!   --example kv_cache_compare -- "Write one sentence about the moon."
//! ```
//!
//! Loads both models sequentially, not concurrently, to keep peak memory
//! bounded — the HF weights cache is already warm after the first load, so
//! the second load is essentially just the engine spin-up cost.

#[cfg(not(feature = "mistralrs"))]
fn main() {
    eprintln!("vibe-infer/examples/kv_cache_compare requires --features mistralrs");
    std::process::exit(2);
}

#[cfg(feature = "mistralrs")]
#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    use vibe_infer::mistral::{KvCacheMode, MistralGenerator};

    let prompt = std::env::args()
        .nth(1)
        .unwrap_or_else(|| "Write one sentence about the moon.".to_string());
    let model_id = std::env::var("VIBE_INFER_MODEL")
        .unwrap_or_else(|_| "Qwen/Qwen2.5-0.5B-Instruct".to_string());
    let seed = std::env::var("VIBE_INFER_KV_CACHE_SEED")
        .ok()
        .and_then(|s| s.parse::<u64>().ok())
        .unwrap_or(42);

    println!("model  : {model_id}");
    println!("prompt : {prompt:?}");
    println!();

    // ---------- Pass 1: Fp16 baseline ----------
    println!("=== Fp16 (upstream baseline) ===");
    let t0 = std::time::Instant::now();
    let fp16 = MistralGenerator::load_with_kv_cache(&model_id, KvCacheMode::Fp16).await?;
    println!("load      : {:.2}s", t0.elapsed().as_secs_f32());
    let (text_fp16, stats_fp16) = run_once(&fp16, &prompt).await?;
    print_stats(&stats_fp16);
    println!("---");
    println!("{text_fp16}");
    println!();

    drop(fp16);

    // ---------- Pass 2: TurboQuant ----------
    println!("=== TurboQuant (codec on every attention layer) ===");
    let t0 = std::time::Instant::now();
    let tq = MistralGenerator::load_with_kv_cache(
        &model_id,
        KvCacheMode::TurboQuant {
            seed,
            qjl_proj_dim: None,
        },
    )
    .await?;
    println!("load      : {:.2}s", t0.elapsed().as_secs_f32());
    let (text_tq, stats_tq) = run_once(&tq, &prompt).await?;
    print_stats(&stats_tq);
    println!("---");
    println!("{text_tq}");
    println!();

    // ---------- Summary ----------
    println!("=== Delta ===");
    let tps_delta =
        (stats_tq.tokens_per_sec - stats_fp16.tokens_per_sec) / stats_fp16.tokens_per_sec * 100.0;
    println!("tok/s     : fp16 {:.1} → tq {:.1} ({:+.1}%)",
        stats_fp16.tokens_per_sec, stats_tq.tokens_per_sec, tps_delta);
    println!("identical : {}", text_fp16 == text_tq);

    Ok(())
}

#[cfg(feature = "mistralrs")]
struct RunStats {
    tokens: usize,
    elapsed_ms: f32,
    tokens_per_sec: f32,
    finish: String,
}

#[cfg(feature = "mistralrs")]
async fn run_once(
    gen: &vibe_infer::mistral::MistralGenerator,
    prompt: &str,
) -> Result<(String, RunStats), Box<dyn std::error::Error>> {
    use vibe_infer::TextGenerator as _;

    let t0 = std::time::Instant::now();
    let out = gen
        .generate(vibe_infer::GenerationRequest {
            prompt: prompt.to_string(),
            max_tokens: 64,
            // Deterministic output so text comparison is meaningful. Temperature
            // jitter would make "identical" a coin flip regardless of codec.
            temperature: 0.0,
            stop: vec![],
        })
        .await?;
    let elapsed_ms = t0.elapsed().as_secs_f32() * 1000.0;
    let tokens_per_sec = if elapsed_ms > 0.0 {
        out.tokens_generated as f32 / (elapsed_ms / 1000.0)
    } else {
        0.0
    };
    Ok((
        out.text,
        RunStats {
            tokens: out.tokens_generated,
            elapsed_ms,
            tokens_per_sec,
            finish: format!("{:?}", out.finish_reason),
        },
    ))
}

#[cfg(feature = "mistralrs")]
fn print_stats(s: &RunStats) {
    println!("tokens    : {}", s.tokens);
    println!("finish    : {}", s.finish);
    println!("infer_ms  : {:.1}", s.elapsed_ms);
    println!("tok/s     : {:.1}", s.tokens_per_sec);
}
