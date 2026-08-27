# tts-bench — which voice, and what it costs

Measures candidate speech engines for the full-duplex voice loop on the same
five sentences, and writes a `.wav` per engine per sentence so the voices can be
judged by ear rather than by real-time factor.

Run `./bench.sh`. First run fetches ~600 MB of models.

## Why two latency numbers

`first_ms` is the time until the first sample can be played — what a user
experiences as the assistant's response time. `rtf` is synthesis seconds per
second of audio; under 1.0 the engine keeps ahead of playback once started.

They are the same number for a one-pass engine and very different for a
streaming one, and conflating them is how a slow engine looks fast. The daemon
speaks sentence by sentence, so only the **first** sentence's synthesis is
latency the user waits through; everything after overlaps with playback, which
any engine with RTF < 1 can sustain.

## Results — M-series, macOS 26.6.2, 2026-08-27

| engine | first audio (median) | RTF | notes |
|---|---:|---:|---|
| **Apple Samantha, compact** (`tts_engine = "system"`) | **21 ms** | 0.019 | streams buffers within an utterance |
| Apple enhanced / premium | *not measured* | | none installed on the test machine |
| **Kokoro-82M, MLX + clause split** (`tts_engine = "kokoro"`) | **218 ms** | 0.106 | what ships |
| Kokoro-82M, MLX, whole sentence | 471 ms | 0.106 | the same model, one pass |
| Kokoro-82M, ONNX fp16 CPU | 826 ms | 0.262 | best ONNX result |
| Kokoro-82M, ONNX fp32 CPU | 869 ms | 0.293 | |
| Kokoro-82M, ONNX fp16 CoreML | 930 ms | 0.297 | CoreML is *slower* than CPU |
| Kokoro-82M, ONNX int8 CPU | 1927 ms | 0.614 | quantisation makes it worse, not better |
| Kokoro-82M, ONNX int8 CoreML | 2181 ms | 0.695 | |

Three of those cut against expectation and are the reason this was measured
rather than estimated:

* **CoreML is slower than CPU** for this graph, at every precision, and costs
  1.6–2.7 s of extra session load. Asking for an accelerator is not getting one.
* **int8 is 2.3× slower than fp16.** The small model is not compute-bound in the
  way quantisation helps.
* **MLX is 1.75× faster than the best ONNX build.** The published "0.08 RTF"
  figure is reachable, but not through ONNX Runtime.

## Clause splitting is worth 2x

Kokoro is non-autoregressive, so a sentence is produced in one pass and first
audio is the *whole sentence's* synthesis time. Splitting at commas lets the
first clause go out while the rest is still being made: 218 ms median against
471 ms, on the same model and machine. Total synthesis rises ~15%, which costs
nothing at RTF 0.106 — playback never catches up.

The cost is prosody: every clause gets sentence-final intonation. `ship-*.wav`
and `mlx-kokoro-*.wav` are the same sentences with and without the split, which
is the comparison the numbers cannot make.

## Two instrument bugs, both caught by disbelieving the number

Recorded because in both cases the harness was wrong in the direction that
would have changed the decision.

**Apple measured 245 ms.** The bench allocated a fresh `AVSpeechSynthesizer`
per utterance. The shipping sidecar's own comments say that costs ~185 ms every
time, which is why it keeps one. Sharing it gave 21 ms — a 12× error, all of it
flattering to the neural alternatives.

**The first Apple run reported zero rows.** `write` was dispatched to the main
queue while the main thread was blocked on the semaphore waiting for it, so
nothing could ever be scheduled. It reported an empty table rather than a
deadlock; the bench now runs on a background thread and leaves a RunLoop on main.

## What is still unmeasured

**Apple's enhanced and premium voices.** They are neural, free, separate
downloads, and none is installed on the machine this ran on — so the cheapest
option in the table is the one row that is empty. Install one via System
Settings → Accessibility → Spoken Content → System Voice → Manage Voices, then
re-run: `apple_bench.swift` picks up every installed `com.apple.voice.*` voice
automatically and will report it alongside compact.

Quality is not measured here at all. RTF says nothing about whether a voice
sounds mechanical, which was the original complaint — that is what the `.wav`
files are for.
