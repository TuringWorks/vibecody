# RL Optimization & Distillation

Optimize RL policies with policy distillation, RL-aware quantization, structured pruning, and multi-format export for deployment.

## When to Use
- Distilling large teacher policies into smaller student models
- Quantizing policies (INT8/INT4/FP16) with RL-aware sensitivity analysis
- Pruning policy networks based on action sensitivity
- Exporting models to ONNX, TorchScript, WASM, TFLite
- Running benchmarks across hardware targets (GPU, CPU, edge, embedded)
- Building declarative optimization pipelines (distill → quantize → prune → export)

## Commands
- `/rlos optimize distill <config.yaml>` — Run policy distillation pipeline
- `/rlos optimize quantize <policy> --int8` — Quantize with RL-aware precision
- `/rlos optimize prune <policy> --target 0.3` — Prune 30% of parameters
- `/rlos optimize benchmark <policy>` — Benchmark across hardware targets
- `/rlos optimize export <policy> --format onnx,wasm` — Export to formats
- `/rlos optimize pipeline <pipeline.yaml>` — Run full optimization pipeline
