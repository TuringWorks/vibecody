"""Kokoro across model precision x execution provider.

The first Kokoro run measured RTF 0.4 against a published 0.08, which is the
signature of an unaccelerated fp32 CPU session rather than a slow model. This
sweeps the combinations before drawing any conclusion about the engine.
"""
import json, sys, time
from pathlib import Path
import numpy as np
import onnxruntime as ort
from kokoro_onnx import Kokoro

SENTENCES = [
    "Yes.",
    "The daemon is running on port seven eight seven eight.",
    "I found three functions that call it, all in the same file.",
    "That change looks safe, but it will need a migration for the existing rows.",
    "I could not tell from the file tree alone, so I opened the README to check.",
]
out = Path(sys.argv[1] if len(sys.argv) > 1 else "out")
MODELS = {"fp32": "kokoro-v1.0.onnx", "fp16": "kokoro-fp16.onnx", "int8": "kokoro-int8.onnx"}
PROVIDERS = {"cpu": ["CPUExecutionProvider"], "coreml": ["CoreMLExecutionProvider", "CPUExecutionProvider"]}

results = []
for mname, mfile in MODELS.items():
    for pname, providers in PROVIDERS.items():
        try:
            t0 = time.perf_counter()
            so = ort.SessionOptions()
            so.log_severity_level = 3
            sess = ort.InferenceSession(str(out / mfile), sess_options=so, providers=providers)
            k = Kokoro.from_session(sess, str(out / "voices-v1.0.bin"))
            load_ms = (time.perf_counter() - t0) * 1000
            # Which provider actually took the graph — asking for CoreML does
            # not mean getting it, and a silent fall back to CPU relabelled as
            # "coreml" would be the whole measurement wrong.
            actual = sess.get_providers()[0]
            k.create("ok", voice="af_heart", speed=1.0, lang="en-us")  # warm
            per = []
            for s in SENTENCES:
                t = time.perf_counter()
                samples, rate = k.create(s, voice="af_heart", speed=1.0, lang="en-us")
                ms = (time.perf_counter() - t) * 1000
                per.append((ms, len(samples) / rate))
            results.append({
                "model": mname, "requested": pname, "actual": actual, "load_ms": load_ms,
                "median_ms": float(np.median([p[0] for p in per])),
                "max_ms": max(p[0] for p in per),
                "rtf": float(np.median([p[0] / 1000 / p[1] for p in per])),
            })
        except Exception as e:
            results.append({"model": mname, "requested": pname, "error": str(e)[:120]})

print(json.dumps(results, indent=None))
