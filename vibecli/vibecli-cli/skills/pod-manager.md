# Pod Manager — vLLM GPU Pod Deployment

Deploy and manage vLLM on remote GPU pods (RunPod, Lambda Labs, Vast.ai) with automatic VRAM validation, tool-call-parser selection, multi-GPU assignment, and build-variant management.

## When to Use
- Deploying open-weight LLMs (Qwen, Mistral, Llama, GLM, GPT-OSS) to GPU cloud pods
- Validating whether a GPU tier can run a model before paying for pod time
- Generating correct `docker run` commands with all required vLLM flags
- Assigning multiple models across a multi-GPU node without index conflicts
- Selecting the right vLLM build variant for a given model type

## Commands
- `/pod deploy <model> --gpu <tier> [--count N] [--build release|nightly|gpt-oss]` — Deploy a model to a GPU pod
- `/pod preflight <model> --gpu <tier> [--count N]` — Validate VRAM without deploying
- `/pod assign <model1> <model2> ... --total-gpus N --vram-per-gpu G` — Plan multi-model GPU assignment
- `/pod models` — List all well-known model configurations
- `/pod tiers` — List GPU tiers with VRAM capacities

## Examples
```
/pod preflight Qwen/Qwen3-Coder-32B-Instruct --gpu a100-80gb --count 1
# Preflight FAILED — 77/80 GB VRAM: Insufficient VRAM: 80 GB available across 1 GPU(s)
# but 77 GB required (model 64 GB + 20% overhead)

/pod preflight Qwen/Qwen3-Coder-32B-Instruct --gpu a100-80gb --count 2
# Preflight OK — 77/160 GB VRAM, TP=1, 1 warning(s)
# Warning: GPU VRAM utilisation below 50% — consider a smaller GPU tier

/pod deploy mistralai/Mistral-7B-Instruct-v0.3 --gpu a10-24gb --build release --port 8000
# docker run --gpus all -p 8000:8000 vllm/vllm-openai:latest \
#   --model mistralai/Mistral-7B-Instruct-v0.3 \
#   --tool-call-parser hermes --enable-auto-tool-choice --port 8000

/pod assign mistralai/Mistral-7B-Instruct-v0.3 meta-llama/Meta-Llama-3-8B-Instruct \
  --total-gpus 4 --vram-per-gpu 24
# Model 0 (mistral-7b) → CUDA:0
# Model 1 (llama-3-8b) → CUDA:1
```

## Rules

### 1. GPU Tier Selection
Match the GPU tier to the model size before ordering pod time:

| Model size (params) | Min VRAM | Recommended GPU |
|---------------------|----------|-----------------|
| 7–9 B               | 14 GB    | A10 24 GB       |
| 13–14 B             | 28 GB    | A10 ×2 or A100 40 GB |
| 32 B                | 64 GB    | A100 80 GB      |
| 70–72 B             | 140 GB   | H100 ×2 or H200 |
| 141 B (MoE active)  | 80 GB    | A100 80 GB (sparse) |

For MoE models count *active* parameter VRAM, not total.

### 2. VRAM Headroom Formula
Always provision with **20 % overhead** above the raw model weight size:

```
vram_required = ceil(model_weight_gb * 1.20)
```

The extra 20 % covers: KV cache for the default context window, CUDA context (~500 MB), vLLM scheduler state, and activation buffers. For long-context runs (>32 K tokens) add an extra 10–20 % for KV cache growth.

### 3. Tensor Parallel Sizing
- Single GPU: `--tensor-parallel-size` is omitted (defaults to 1).
- Multi-GPU: use the smallest power-of-two that provides enough total VRAM.
- Always prefer TP that divides evenly into `attention_heads` for the model.
- For pipeline parallelism across nodes use `--pipeline-parallel-size` in addition to TP.

```
tp = smallest_power_of_two where tp * vram_per_gpu >= vram_required
tp = min(tp, gpu_count)
```

### 4. Tool-Call-Parser Selection

| Model family        | `--tool-call-parser` value | Notes |
|---------------------|----------------------------|-------|
| Qwen3-Coder         | `qwen3-coder`              | Always pair with `--enable-auto-tool-choice` |
| GLM-4-MoE           | `glm4-moe`                 | Always pair with `--enable-auto-tool-choice` |
| GPT-OSS             | `openai-responses`         | Use with gpt-oss build; `/v1/responses` endpoint |
| Qwen, Mistral, Llama | `hermes`                  | Broadest compatibility |
| Unknown / other     | `auto`                     | Let vLLM detect at startup |
| Tool-call-unsupported | *(omit flag)*             | No `--enable-auto-tool-choice` either |

Always include `--enable-auto-tool-choice` whenever `--tool-call-parser` is set.

### 5. GPT-OSS vs Release Build

Use `VllmBuild::GptOss` (`vllm/vllm-openai:gpt-oss`) exclusively for models that:
- Require the `/v1/responses` endpoint (OpenAI Responses API).
- Are labelled `gpt-oss` in their model card.

Use `VllmBuild::Release` (`vllm/vllm-openai:latest`) for all other models. The GPT-OSS build is **not** interchangeable with the standard release — it patches the OpenAI-compat server and may break non-GPT-OSS models.

### 6. Nightly Build Trade-offs

`VllmBuild::Nightly` (`vllm/vllm-openai:nightly`) gives access to:
- Latest speculative decoding improvements
- Unreleased model architecture support
- Experimental chunked prefill and prefix caching

Trade-offs:
- No stability guarantee — breaking changes can appear daily
- Not suitable for production; use only for prototype/research pods
- Pin the nightly tag by date (`nightly-YYYYMMDD`) for reproducibility

### 7. Pod Provider Comparison

| Provider     | Strengths | Weaknesses | Best For |
|--------------|-----------|------------|----------|
| **RunPod**   | Large GPU catalogue, per-minute billing, community templates | Variable availability for H100/H200 | Experimentation, burst workloads |
| **Lambda Labs** | Reserved H100/A100 clusters, stable networking, S3-compatible storage | Higher minimum commitment, fewer GPU types | Production inference, long-running jobs |
| **Vast.ai**  | Cheapest spot pricing, bidding market | No SLA, host reliability varies | Cost-sensitive dev/test, batch jobs |

For production inference prefer Lambda Labs or a reserved RunPod instance. For exploratory work and nightly builds, Vast.ai spot instances cut costs significantly.

### 8. Models Path Mount Convention

Mount HuggingFace model cache using the standard convention:

```
docker run ... -v /path/to/hf-cache:/workspace/models ...
```

- Inside the container, vLLM looks for models in `/workspace/models` by default.
- Set `HF_HOME=/workspace/models` or `TRANSFORMERS_CACHE=/workspace/models` as an environment variable to override HuggingFace's default cache path inside the container.
- The `extract_models_path("host:/container")` helper parses the host-side path from a volume-mount argument, making it safe to pass user-supplied `-v` strings without manual string splitting.
- For Lambda Labs and RunPod persistent storage, mount the shared volume at `/workspace` and pass `--models-path /workspace/models` rather than a full Docker volume arg.
