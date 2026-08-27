"""Clause splitting: does emitting the first clause sooner actually pay?

Kokoro synthesises a whole sentence in one pass, so first audio is the whole
sentence's synthesis time. Splitting at commas lets the first clause go out
early and the rest synthesise while it plays. The cost is prosody — every clause
gets sentence-final intonation — so the .wav matters as much as the numbers.
"""
import re, time, wave
import numpy as np
from mlx_audio.tts.utils import load_model

SENTS = [
    "That change looks safe, but it will need a migration for the existing rows.",
    "I could not tell from the file tree alone, so I opened the README to check.",
]
# Split after a comma only when both sides are substantial: "Yes, ok" split in
# two is three round trips of overhead for no gain.
def clauses(s, minlen=22):
    parts, buf = [], ""
    for piece in re.split(r"(?<=,)\s+", s):
        buf = (buf + " " + piece).strip() if buf else piece
        if len(buf) >= minlen:
            parts.append(buf); buf = ""
    if buf:
        if parts: parts[-1] += " " + buf
        else: parts.append(buf)
    return parts

m = load_model("prince-canuma/Kokoro-82M")
list(m.generate(text="ok", voice="af_heart", speed=1.0, lang_code="a"))

for si, s in enumerate(SENTS):
    t0 = time.perf_counter(); whole = []
    for seg in m.generate(text=s, voice="af_heart", speed=1.0, lang_code="a"):
        whole.append(np.asarray(seg.audio).reshape(-1))
    whole_ms = (time.perf_counter() - t0) * 1000

    cs = clauses(s)
    t0 = time.perf_counter(); first_ms = None; out = []
    for c in cs:
        for seg in m.generate(text=c, voice="af_heart", speed=1.0, lang_code="a"):
            out.append(np.asarray(seg.audio).reshape(-1))
        if first_ms is None: first_ms = (time.perf_counter() - t0) * 1000
    split_all = (time.perf_counter() - t0) * 1000
    cat = np.concatenate(out)
    with wave.open(f"out/clause-split-{si}.wav", "wb") as w:
        w.setnchannels(1); w.setsampwidth(2); w.setframerate(24000)
        w.writeframes((np.clip(cat, -1, 1) * 32767).astype("<i2").tobytes())
    with wave.open(f"out/clause-whole-{si}.wav", "wb") as w:
        w.setnchannels(1); w.setsampwidth(2); w.setframerate(24000)
        w.writeframes((np.clip(np.concatenate(whole), -1, 1) * 32767).astype("<i2").tobytes())
    print(f"{len(cs)} clauses | whole {whole_ms:6.1f}ms | split first {first_ms:6.1f}ms total {split_all:6.1f}ms")
    print(f"   {cs}")
