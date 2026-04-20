//! End-to-end smoke test for the candle MiniLM backend.
//!
//! Run with:
//! ```sh
//! cargo run -p vibe-infer --features candle --example embed -- "hello world"
//! cargo run -p vibe-infer --features candle-metal --example embed -- "hello world"
//! ```
//!
//! First invocation downloads ~22 MB from Hugging Face into `~/.cache/huggingface`.
//! Subsequent invocations are offline. Prints the embedding dimension, L2 norm
//! (should be ~1.0), and the first 8 components so a human can eyeball the result.
//!
//! Without `--features candle` the example refuses to compile — the stub backend
//! has no useful output and would mislead a smoke-tester into thinking the
//! download / inference path worked.

#[cfg(not(feature = "candle"))]
fn main() {
    eprintln!("vibe-infer/examples/embed requires --features candle");
    eprintln!("try: cargo run -p vibe-infer --features candle --example embed -- \"hello world\"");
    std::process::exit(2);
}

#[cfg(feature = "candle")]
#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    use vibe_infer::{minilm::MiniLmEmbedder, Embedder};

    let text = std::env::args().nth(1).unwrap_or_else(|| "hello world".to_string());

    eprintln!("loading sentence-transformers/all-MiniLM-L6-v2 …");
    let t0 = std::time::Instant::now();
    let embedder = MiniLmEmbedder::load().await?;
    eprintln!("  ready in {:.2}s", t0.elapsed().as_secs_f32());

    let t1 = std::time::Instant::now();
    let v = embedder.embed(&text).await?;
    let elapsed_ms = t1.elapsed().as_secs_f32() * 1000.0;

    let norm: f32 = v.iter().map(|x| x * x).sum::<f32>().sqrt();
    let preview: Vec<String> = v.iter().take(8).map(|x| format!("{x:+.4}")).collect();

    println!("input     : {text:?}");
    println!("dim       : {}", v.len());
    println!("l2_norm   : {norm:.6}");
    println!("infer_ms  : {elapsed_ms:.1}");
    println!("preview   : [{}, …]", preview.join(", "));

    Ok(())
}
