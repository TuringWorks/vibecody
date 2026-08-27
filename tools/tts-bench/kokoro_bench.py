"""Kokoro-82M (ONNX) bench — the same five sentences as apple_bench.swift.

Kokoro is non-autoregressive (StyleTTS2 decoder + ISTFTNet vocoder), so a
sentence is produced in a fixed number of passes rather than token by token.
That makes `first_ms` and `total_ms` the same number for the one-pass API: there
is no partial audio to start playing. `create_stream` chunks the *text* and
synthesises each chunk, which is a different thing and is measured separately —
it is the number that matters for a conversation.

Model load is paid once per process and warmed before timing, matching the
Apple bench, which warms the synthesiser before its first timed utterance.
"""
import json, sys, time, wave, struct
from pathlib import Path
import numpy as np
from kokoro_onnx import Kokoro

SENTENCES = [
    "Yes.",
    "The daemon is running on port seven eight seven eight.",
    "I found three functions that call it, all in the same file.",
    "That change looks safe, but it will need a migration for the existing rows.",
    "I could not tell from the file tree alone, so I opened the README to check.",
]
# Two voices, so the comparison is not one voice's quirk. af_heart is the
# default in most Kokoro demos; am_michael is the male counterpart.
VOICES = ["af_heart", "am_michael"]

out = Path(sys.argv[1] if len(sys.argv) > 1 else "out")
out.mkdir(parents=True, exist_ok=True)


def write_wav(path, samples, rate):
    with wave.open(str(path), "wb") as w:
        w.setnchannels(1)
        w.setsampwidth(2)
        w.setframerate(int(rate))
        pcm = np.clip(samples, -1.0, 1.0)
        w.writeframes((pcm * 32767).astype("<i2").tobytes())


t_load = time.perf_counter()
k = Kokoro(str(out / "kokoro-v1.0.onnx"), str(out / "voices-v1.0.bin"))
load_ms = (time.perf_counter() - t_load) * 1000

rows = []
for voice in VOICES:
    k.create("ok", voice=voice, speed=1.0, lang="en-us")  # warm
    for i, s in enumerate(SENTENCES):
        t0 = time.perf_counter()
        samples, rate = k.create(s, voice=voice, speed=1.0, lang="en-us")
        total_ms = (time.perf_counter() - t0) * 1000
        audio_sec = len(samples) / rate
        write_wav(out / f"kokoro-{voice}-{i}.wav", samples, rate)
        rows.append({
            "engine": "kokoro", "voice": voice, "quality": "neural", "sentence": i,
            # One pass: nothing can be played until the whole sentence is done,
            # so first audio *is* total. Reporting a smaller number here would
            # be inventing a streaming capability this API does not have.
            "first_ms": total_ms, "total_ms": total_ms,
            "audio_sec": audio_sec, "rtf": (total_ms / 1000) / max(audio_sec, 1e-3),
        })

print(json.dumps({"rows": rows, "load_ms": load_ms}))
