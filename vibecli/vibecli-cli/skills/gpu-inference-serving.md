---
triggers: ["inference serving", "model serving", "vLLM", "TGI", "Triton", "model deployment", "inference optimization"]
tools_allowed: ["read_file", "write_file", "bash"]
category: ai
---

# GPU Inference Serving

When deploying and optimizing model inference:

1. **vLLM** — Use vLLM for high-throughput LLM serving. It implements PagedAttention for efficient KV cache management (eliminates memory fragmentation) and continuous batching (processes new requests without waiting for the batch to complete). Launch with `vllm serve model_name --tensor-parallel-size N`. Supports OpenAI-compatible API out of the box. Best for throughput-critical deployments.

2. **Text Generation Inference (TGI)** — HuggingFace's inference server with built-in token streaming, continuous batching, and quantization support. Run with `docker run ghcr.io/huggingface/text-generation-inference --model-id model_name`. Supports flash attention, GPTQ/AWQ quantization, and speculative decoding. Good choice when you need tight HuggingFace ecosystem integration.

3. **Triton Inference Server** — NVIDIA's multi-framework serving platform supporting ONNX, TensorRT, PyTorch, and TensorFlow models simultaneously. Use model ensembles to chain preprocessing, model, and postprocessing steps. Configure instance groups for GPU/CPU placement. Dynamic batching groups requests within a configurable time window. Best for heterogeneous model serving and complex pipelines.

4. **Quantization (GPTQ, AWQ, GGUF, int8/int4)** — GPTQ (post-training, 4-bit, GPU-optimized) and AWQ (activation-aware, better quality at 4-bit) are the top choices for GPU inference. GGUF is optimized for CPU/Apple Silicon via llama.cpp. bitsandbytes provides easy int8/int4 quantization in Python. Quantization typically reduces memory 2-4x with <1% quality loss. Always benchmark your specific use case.

5. **KV cache optimization** — The KV cache grows linearly with sequence length and batch size; it often dominates GPU memory. Use PagedAttention (vLLM) to avoid fragmentation. Set appropriate `max_model_len` to limit cache size. Consider multi-query attention (MQA) or grouped-query attention (GQA) models which have smaller KV caches. Prefix caching reuses KV cache across requests sharing the same system prompt.

6. **Speculative decoding** — Use a small draft model to generate candidate tokens, then verify them in parallel with the large target model. Achieves 2-3x speedup for latency-sensitive applications without changing output quality. Configure in vLLM with `--speculative-model` or in TGI with `--speculation`. Works best when draft and target models share the same tokenizer.

7. **Tensor parallelism for inference** — Split model layers across multiple GPUs to serve models that exceed single-GPU memory or to reduce per-token latency. Set `--tensor-parallel-size N` in vLLM or TGI. Requires fast GPU interconnect (NVLink preferred). Tensor parallelism reduces latency; pipeline parallelism increases throughput. For inference, tensor parallelism is usually preferred.

8. **Batching strategies** — Dynamic batching groups incoming requests into batches with configurable max wait time and batch size. Continuous batching (iteration-level scheduling) processes requests at different stages of generation simultaneously, maximizing GPU utilization. Set `max_num_seqs` (vLLM) or `max_batch_total_tokens` (TGI) based on GPU memory and latency SLOs.

9. **Auto-scaling** — Scale based on GPU utilization, request queue depth, or p99 latency. Use Kubernetes HPA with custom metrics from dcgm-exporter or application-level metrics. Scale to zero for cost savings on infrequent workloads using KEDA. Pre-warm instances by loading models at startup. Consider separate scaling for prefill-heavy vs decode-heavy workloads.

10. **Health checks and monitoring** — Expose `/health` and `/ready` endpoints. Monitor tokens/second, time-to-first-token (TTFT), inter-token latency, queue depth, GPU utilization, GPU memory usage, and cache hit rates. Set up alerts for TTFT > SLO threshold, GPU memory > 90%, and error rate spikes. Log prompt/completion token counts for cost tracking.

11. **A/B testing models** — Route a percentage of traffic to candidate models using a load balancer or service mesh (Istio). Compare quality metrics (user ratings, task success rate) alongside latency and cost. Use shadow mode (duplicate requests to the candidate without serving its responses) for safe evaluation. Automate promotion/rollback based on metric thresholds.

12. **Latency optimization** — Minimize time-to-first-token with model warmup, prompt caching, and prefix caching. Use flash attention for faster attention computation. Compile models with `torch.compile` or TensorRT for kernel fusion. Reduce network overhead with gRPC instead of REST for internal calls. Place inference servers in the same region/zone as clients.
