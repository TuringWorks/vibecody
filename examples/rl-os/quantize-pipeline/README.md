# quantize-pipeline

INT8 dynamic ONNX quantization of the cartpole-baseline checkpoint.
Demonstrates slice 7a — model compression as a chained run that reads a
parent run's `final.pt`, exports to ONNX, and emits a quantized
`final.onnx` plus a compression-ratio metric.

## Prerequisites

```bash
cd vibe-rl-py
uv sync --extra opt   # onnx + onnxruntime
```

And the cartpole baseline must be run first — this example chains off
its `final.pt`:

```bash
../cartpole-baseline/run.sh
```

## Run

```bash
./run.sh
```

## Expected output

```
{"t":"started","run_id":"quantize-001","sidecar_version":"...","device":"cpu"}
{"t":"checkpoint","run_id":"...","rel_path":".vibecli/rl-artifacts/quantize-001/final.onnx","sha256":"...","size_bytes":19329}
{"t":"checkpoint","run_id":"...","rel_path":".vibecli/rl-artifacts/quantize-001/final-int8.onnx","sha256":"...","size_bytes":9099}
{"t":"tick","run_id":"...","payload":{"fp32_size_bytes":19329,"int8_size_bytes":9099,"compression_ratio":2.12,"scheme":"int8_dynamic"}}
{"t":"finished","run_id":"...","reason":"done"}
```

Two checkpoint artifacts land — `final.onnx` (FP32 export) and
`final-int8.onnx` (quantized).

## What to look for

- **`payload.compression_ratio` ≈ 2×** for a small CartPole MLP (FP32 →
  INT8 dynamic). Tiny networks can't hit the theoretical 4× because
  ONNX overhead is a meaningful fraction of the file. Larger
  transformer-class models converge toward the 4× theoretical ceiling.
- **`payload.fp32_size_bytes` vs `payload.int8_size_bytes`** — the raw payoff.
- **`final-int8.onnx` artifact** at
  `.workspace/.vibecli/rl-artifacts/quantize-001/final-int8.onnx` —
  the quantized model, servable via the ONNX runtime path
  (`runtime: "onnx"` in a deployment). The unquantized `final.onnx`
  also lands in the same directory for comparison.

## Re-run

```bash
rm -rf .workspace
./run.sh
```

## Time

~10 s. Most of the wall-clock is `torch.onnx.export`; quantization
itself is sub-second.
