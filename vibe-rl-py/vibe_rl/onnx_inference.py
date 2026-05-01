"""ONNX runtime variant of the inference sidecar.

Mirrors `vibe_rl.inference` but loads via onnxruntime-python instead of
PyTorch. The wire protocol is identical:

    stdout → {"t":"ready", ...}                   (once, at startup)
    stdin  ← {"obs": [<floats>]}
    stdout → {"action": <int | [<scalars>]>}
    stdout → {"error": "<msg>"}

This path activates when the daemon's deployment row has
`runtime = "onnx"`. It pairs naturally with slice 7a's quantize output
(`final-int8.onnx`) and any FP32 ONNX export the user provides. The
sidecar metadata sidecar (`<model>.json` next to the .onnx) tells us
the obs shape and action kind.

We use the CPU execution provider by default. CUDA / CoreML are
available if the host onnxruntime build supports them, but the slice
6.5 deployment story is "make Python serve real" — perf comes later.
"""

from __future__ import annotations

import argparse
import json
import sys
from pathlib import Path
from typing import Any

import numpy as np


def _load_metadata(model_path: Path) -> dict[str, Any]:
    """Read the `<model>.json` sidecar that slice 7a's quantize writes."""
    sidecar = model_path.with_suffix(".json")
    if not sidecar.is_file():
        raise FileNotFoundError(
            f"ONNX metadata sidecar missing: {sidecar}. Slice 7a's quantize "
            f"writes both .onnx and .json — point at the .onnx file."
        )
    return json.loads(sidecar.read_text())


def main(argv: list[str] | None = None) -> int:
    parser = argparse.ArgumentParser(prog="python -m vibe_rl onnx-inference")
    parser.add_argument(
        "--model",
        required=True,
        help="path to the .onnx file (must have a sibling .json metadata)",
    )
    args = parser.parse_args(argv)

    try:
        import onnxruntime as ort
    except ImportError as e:
        sys.stdout.write(
            json.dumps(
                {
                    "t": "error",
                    "error": (
                        f"onnxruntime not installed — install with "
                        f"`cd vibe-rl-py && uv sync --extra opt`. "
                        f"({type(e).__name__}: {e})"
                    ),
                }
            )
            + "\n"
        )
        sys.stdout.flush()
        return 1

    model_path = Path(args.model)
    if not model_path.is_file():
        sys.stdout.write(
            json.dumps({"t": "error", "error": f"model not found: {model_path}"}) + "\n"
        )
        sys.stdout.flush()
        return 1

    try:
        metadata = _load_metadata(model_path)
        action_kind = metadata.get("action_kind", "discrete")
        # Pick the available providers in the order: CUDA → CoreML → CPU.
        available = set(ort.get_available_providers())
        providers: list[str] = []
        for p in ("CUDAExecutionProvider", "CoreMLExecutionProvider", "CPUExecutionProvider"):
            if p in available:
                providers.append(p)
        sess_options = ort.SessionOptions()
        sess_options.graph_optimization_level = ort.GraphOptimizationLevel.ORT_ENABLE_ALL
        session = ort.InferenceSession(
            str(model_path),
            sess_options=sess_options,
            providers=providers or ["CPUExecutionProvider"],
        )
        input_name = session.get_inputs()[0].name
        output_meta = session.get_outputs()[0]
    except Exception as e:  # noqa: BLE001
        sys.stdout.write(
            json.dumps({"t": "error", "error": f"{type(e).__name__}: {e}"}) + "\n"
        )
        sys.stdout.flush()
        return 1

    sys.stdout.write(
        json.dumps(
            {
                "t": "ready",
                "framework": "onnx",
                "action_kind": action_kind,
                "device": session.get_providers()[0] if session.get_providers() else "cpu",
                "model": str(model_path),
                "input_name": input_name,
                "output_name": output_meta.name,
            }
        )
        + "\n"
    )
    sys.stdout.flush()

    for line in sys.stdin:
        line = line.strip()
        if not line:
            continue
        try:
            req = json.loads(line)
        except json.JSONDecodeError as e:
            sys.stdout.write(json.dumps({"error": f"invalid JSON: {e}"}) + "\n")
            sys.stdout.flush()
            continue

        obs_raw = req.get("obs")
        if obs_raw is None:
            sys.stdout.write(json.dumps({"error": "obs field required"}) + "\n")
            sys.stdout.flush()
            continue

        try:
            obs = np.asarray(obs_raw, dtype=np.float32)
            if obs.ndim == 1:
                obs = obs.reshape(1, -1)
            outputs = session.run(None, {input_name: obs})
            head = outputs[0]
            # For discrete spaces the actor head emits logits; we pick
            # argmax for deterministic serve-time action selection (mirrors
            # the PyTorch sidecar's `agent.act()` behaviour).
            if action_kind == "discrete":
                action_arr = head.argmax(axis=-1)
            else:
                action_arr = head
            action_np = np.asarray(action_arr)
            if action_np.size == 1:
                action_out: int | float | list[float] = action_np.item()
            else:
                action_out = action_np.flatten().tolist()
            sys.stdout.write(json.dumps({"action": action_out}) + "\n")
            sys.stdout.flush()
        except Exception as e:  # noqa: BLE001
            sys.stdout.write(json.dumps({"error": f"{type(e).__name__}: {e}"}) + "\n")
            sys.stdout.flush()

    return 0


if __name__ == "__main__":
    raise SystemExit(main())
