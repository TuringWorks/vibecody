"""Kokoro via MLX — the Apple-Silicon-native path, and the one speech-swift uses.

ONNX measured RTF 0.26 against a published 0.08. Either the published number is
optimistic or it came from a different runtime; this decides which, on the same
five sentences, before the ONNX result is used to rule the model out.
"""
import json, sys, time
from pathlib import Path
import numpy as np

SENTENCES = [
    "Yes.",
    "The daemon is running on port seven eight seven eight.",
    "I found three functions that call it, all in the same file.",
    "That change looks safe, but it will need a migration for the existing rows.",
    "I could not tell from the file tree alone, so I opened the README to check.",
]
out = Path("out")

from mlx_audio.tts.generate import generate_audio  # noqa: E402
from mlx_audio.tts.utils import load_model  # noqa: E402

MODEL = "prince-canuma/Kokoro-82M"
t0 = time.perf_counter()
model = load_model(MODEL)
load_ms = (time.perf_counter() - t0) * 1000

rows = []
def run(text, i, warm=False):
    t = time.perf_counter()
    segs = list(model.generate(text=text, voice="af_heart", speed=1.0, lang_code="a"))
    ms = (time.perf_counter() - t) * 1000
    audio = np.concatenate([np.asarray(s.audio).reshape(-1) for s in segs]) if segs else np.zeros(1)
    return ms, audio, getattr(segs[0], "sample_rate", 24000) if segs else 24000

run("ok", -1, warm=True)  # warm: first call pays lazy graph build
for i, s in enumerate(SENTENCES):
    ms, audio, rate = run(s, i)
    dur = len(audio) / rate
    import wave
    with wave.open(str(out / f"mlx-kokoro-af_heart-{i}.wav"), "wb") as w:
        w.setnchannels(1); w.setsampwidth(2); w.setframerate(int(rate))
        w.writeframes((np.clip(audio, -1, 1) * 32767).astype("<i2").tobytes())
    rows.append({"engine": "mlx-kokoro", "sentence": i, "first_ms": ms, "total_ms": ms,
                 "audio_sec": dur, "rtf": (ms / 1000) / max(dur, 1e-3)})

print(json.dumps({"rows": rows, "load_ms": load_ms}))
