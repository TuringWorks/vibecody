# TurboQuant Vector Compression

## Overview
TurboQuant is a two-stage vector compression algorithm from Google Research (2026)
that achieves ~3 bits per dimension with negligible recall loss for cosine-similarity
search. It combines PolarQuant (random rotation + polar grid quantization) with QJL
(Quantized Johnson-Lindenstrauss 1-bit residual compression).

## When to Use
- Embedding indices exceeding ~50K vectors (where memory is a bottleneck)
- KV-cache compression for local LLM inference (vLLM, llama.cpp)
- Any dense vector store that needs 5-10x memory reduction without retraining
- RAG pipelines with large document corpora on memory-constrained hardware

## Compression Pipeline
```
f32 vector → random rotation → PolarQuant (2-bit grid) → reconstruct
                                                             │
                                              residual = original − reconstructed
                                                             │
                                                        QJL (1-bit signs)
```

## Storage Comparison (384-dim, 10K vectors)
| Method      | Per-Vector | Total    | Ratio |
|-------------|-----------|----------|-------|
| f32         | 1,536 B   | 15.0 MB  | 1.0x  |
| TurboQuant  | ~152 B    | ~1.5 MB  | 10.1x |
| Product Q   | ~96 B     | ~0.9 MB  | 16x   |

TurboQuant has higher recall than Product Quantization at comparable compression.

## REPL Commands
```
/turboquant benchmark 500 128    # Benchmark with 500 vectors, 128-dim
/turboquant benchmark 2000 384   # Benchmark with nomic-embed-text size
/turboquant memory 7             # Compare KV-cache VRAM for 7B model
/turboquant memory 70            # Compare KV-cache VRAM for 70B model
```

## Programmatic Usage (Rust)
```rust
use vibe_core::index::turboquant::{TurboQuantIndex, TurboQuantConfig};

let config = TurboQuantConfig {
    dimension: 384,
    seed: 42,
    qjl_proj_dim: None,
};
let mut index = TurboQuantIndex::new(config);
index.insert("doc_0", &embedding_vec, metadata)?;

let results = index.search(&query_vec, 10);
println!("Compression: {:.1}x", index.compression_ratio());
```

## Converting Existing Indices
```rust
// From EmbeddingIndex
let tq = embedding_index.to_turboquant(42).unwrap();

// From InMemoryVectorDb
let tq = vector_db.to_turboquant(42);
```

## Key Parameters
- **dimension**: Must match your embedding model (384 for nomic-embed-text, 1536 for OpenAI)
- **seed**: Deterministic PRNG seed for rotation/projection matrices; same seed = same compression
- **qjl_proj_dim**: QJL projection dimensions; defaults to same as dimension (1:1 ratio)

## Architecture Notes
- Random rotation uses Gram-Schmidt orthogonalization (norm-preserving)
- PolarQuant: 4 quantization levels per dimension (2 bits), packed 4 per byte
- QJL: 1-bit signs of Johnson-Lindenstrauss projected residual, packed 8 per byte
- Serializable via serde; matrices are regenerated from seed after deserialization
- Search decompresses vectors and computes exact cosine similarity
