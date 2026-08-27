"""Does mlx-audio yield Kokoro segments progressively, or only at the end?

The headline 471 ms came from list(model.generate(...)), which collects every
segment before the timer stops. If segments arrive one at a time, first audio is
the *first* segment and the shipping number is much lower than the bench said.
"""
import time
import numpy as np
from mlx_audio.tts.utils import load_model

SENTS = [
    "Yes.",
    "That change looks safe, but it will need a migration for the existing rows.",
    "I could not tell from the file tree alone, so I opened the README to check.",
]
m = load_model("prince-canuma/Kokoro-82M")
list(m.generate(text="ok", voice="af_heart", speed=1.0, lang_code="a"))  # warm

for s in SENTS:
    t0 = time.perf_counter()
    marks, total = [], 0.0
    for seg in m.generate(text=s, voice="af_heart", speed=1.0, lang_code="a"):
        a = np.asarray(seg.audio).reshape(-1)
        total += len(a) / 24000
        marks.append((time.perf_counter() - t0) * 1000)
    print(f"segments={len(marks):2d}  first={marks[0]:7.1f}ms  all={marks[-1]:7.1f}ms  audio={total:.2f}s  | {s[:44]}")
