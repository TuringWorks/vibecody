//! End-to-end smoke test for the Mistral.rs text-generation backend.
//!
//! Run with:
//! ```sh
//! cargo run -p vibe-infer --features mistralrs         --example generate -- "Say hi in one word."
//! cargo run -p vibe-infer --features mistralrs-metal   --example generate -- "Say hi in one word."
//! cargo run -p vibe-infer --features mistralrs-cuda    --example generate -- "Say hi in one word."
//! ```
//!
//! First invocation downloads the model (varies by model; Qwen2.5-0.5B-Instruct
//! is ~1 GB) into `~/.cache/huggingface`. Override the model via
//! `VIBE_INFER_MODEL=<hf-repo-id>`; default is a small model for fast iteration.
//!
//! Without `--features mistralrs` the example refuses to compile — the stub
//! backend would produce no meaningful output and mislead the smoke-tester.

#[cfg(not(feature = "mistralrs"))]
fn main() {
    eprintln!("vibe-infer/examples/generate requires --features mistralrs");
    eprintln!("try: cargo run -p vibe-infer --features mistralrs --example generate -- \"hello\"");
    std::process::exit(2);
}

#[cfg(feature = "mistralrs")]
#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    use vibe_infer::{mistral::MistralGenerator, GenerationRequest, TextGenerator};

    let prompt = std::env::args()
        .nth(1)
        .unwrap_or_else(|| "Say hi in one word.".to_string());
    let model_id =
        std::env::var("VIBE_INFER_MODEL").unwrap_or_else(|_| "Qwen/Qwen2.5-0.5B-Instruct".to_string());

    eprintln!("loading {model_id} via Mistral.rs …");
    let t0 = std::time::Instant::now();
    let generator = MistralGenerator::load(&model_id).await?;
    eprintln!("  ready in {:.2}s", t0.elapsed().as_secs_f32());

    let t1 = std::time::Instant::now();
    let out = generator
        .generate(GenerationRequest {
            prompt: prompt.clone(),
            max_tokens: 64,
            temperature: 0.7,
            stop: vec![],
        })
        .await?;
    let elapsed_ms = t1.elapsed().as_secs_f32() * 1000.0;

    println!("prompt    : {prompt:?}");
    println!("model     : {}", generator.model_id());
    println!("tokens    : {}", out.tokens_generated);
    println!("finish    : {:?}", out.finish_reason);
    println!("infer_ms  : {elapsed_ms:.1}");
    if out.tokens_generated > 0 {
        let toks_per_sec = out.tokens_generated as f32 / (elapsed_ms / 1000.0);
        println!("tok/s     : {toks_per_sec:.1}");
    }
    println!("---");
    println!("{}", out.text);

    Ok(())
}
