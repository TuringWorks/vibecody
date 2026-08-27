#!/usr/bin/env bash
# Reproduce the TTS comparison. Fetches ~600 MB of models on first run.
#
# Every engine is measured on the same five sentences, and the two numbers
# reported are deliberately different things:
#
#   first_ms  time until the first sample can be played — what the user
#             experiences as the assistant's response time
#   rtf       synthesis seconds per second of audio; below 1.0 means the engine
#             keeps ahead of playback once it has started
#
# For a streaming engine (Apple) first_ms is far below total synthesis time.
# For a one-pass engine (Kokoro) they are the same number, because there is no
# partial audio to start on.
set -euo pipefail
cd "$(dirname "$0")"
mkdir -p out

command -v uv >/dev/null || { echo "needs uv: brew install uv"; exit 1; }

REL=https://github.com/thewh1teagle/kokoro-onnx/releases/download/model-files-v1.0
for f in kokoro-v1.0.onnx voices-v1.0.bin; do
  [ -f "out/$f" ] || curl -sL -o "out/$f" "$REL/$f"
done
[ -f out/kokoro-fp16.onnx ] || curl -sL -o out/kokoro-fp16.onnx "$REL/kokoro-v1.0.fp16.onnx"
[ -f out/kokoro-int8.onnx ] || curl -sL -o out/kokoro-int8.onnx "$REL/kokoro-v1.0.int8.onnx"

[ -d .venv ] || uv venv --python 3.12 .venv
VIRTUAL_ENV=$PWD/.venv uv pip install -q kokoro-onnx soundfile numpy mlx-audio "misaki[en]"

# Apple: shared synthesiser, as the shipping sidecar uses. A fresh one per
# utterance costs ~185 ms every time and measures the harness, not the engine.
swiftc -O apple_bench.swift -o out/apple_bench
./out/apple_bench out > out/apple.json

./.venv/bin/python kokoro_bench.py out > out/kokoro.json
./.venv/bin/python kokoro_sweep.py out > out/sweep.json
# misaki auto-installs a spaCy model on first run and prints to stdout, so the
# JSON is the line that starts with {"rows" rather than the whole file.
./.venv/bin/python mlx_bench.py > out/mlx.json

echo "wrote out/*.json and out/*.wav"
