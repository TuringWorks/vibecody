"""Encode the bench's .wav output as base64 data URIs for the report page.

Split from build_report.py because the encode is the slow part (~4 MB of
base64) and the page layout is the part that gets iterated on.
"""
import base64, json, pathlib
out = pathlib.Path("out")

SENTENCES = [
    "Yes.",
    "The daemon is running on port seven eight seven eight.",
    "I found three functions that call it, all in the same file.",
    "That change looks safe, but it will need a migration for the existing rows.",
    "I could not tell from the file tree alone, so I opened the README to check.",
]
ENGINES = [
    ("Apple Samantha", "compact · tts_engine = system", "apple-compact-Samantha-{}.wav"),
    ("Kokoro af_heart", "MLX + clauses · tts_engine = kokoro", "ship-kokoro-{}.wav"),
    ("Kokoro af_heart", "MLX · whole sentence", "mlx-kokoro-af_heart-{}.wav"),
    ("Kokoro am_michael", "ONNX fp32", "kokoro-am_michael-{}.wav"),
]

def b64(p):
    return "data:audio/wav;base64," + base64.b64encode((out / p).read_bytes()).decode()

clips = [[b64(f.format(i)) for _, _, f in ENGINES] for i in range(len(SENTENCES))]
total_mb = sum(len(c) for row in clips for c in row) / 1e6
print(f"embedded audio: {total_mb:.1f} MB base64")
pathlib.Path("out/clips.json").write_text(json.dumps({
    "sentences": SENTENCES,
    "engines": [{"name": n, "sub": s} for n, s, _ in ENGINES],
    "clips": clips,
}))
