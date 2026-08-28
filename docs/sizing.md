---
layout: page
title: "Sizing & Hardware"
permalink: /sizing/
---

How much machine VibeCody needs, and where it goes. Two kinds of number appear
on this page and they are kept apart on purpose:

- **Measured** — taken from a run in this repository, with the source or the
  harness named. Trust these.
- **Estimated** — arithmetic from parameter counts and quantisation, or a
  vendor's published figure. Useful for planning, wrong in the third decimal
  place. Every estimate says so.

Nothing here is a benchmark of *your* hardware. `vibecli --setup` reads the
machine in front of it and recommends from that.

---

## The short answer

| You are running | RAM | Disk | GPU |
|---|---|---|---|
| A desktop shell, cloud provider only | 4 GB free | ~1 GB | Not needed |
| The daemon alone, cloud provider only | 1–2 GB | ~500 MB | Not needed |
| Voice (speech in and out) | +1–3 GB | +0.2–3 GB | Not needed |
| A 7B model locally | 8–16 GB | ~5 GB | Strongly wanted |
| A 13B model locally | 16–32 GB | ~9 GB | Yes |

**The daemon itself is small.** Nearly everything on this page is about the
*models* you choose to run beside it. A daemon pointed at a cloud provider will
run comfortably on a Raspberry Pi.

---

## What the setup wizard already decides

`vibecli --setup` detects OS, architecture, RAM and GPU, then picks a deployment
tier and a local model. This is the policy in
`vibecli/vibecli-cli/src/setup.rs`, not a recommendation invented for this page:

**Tier, from system RAM**

| RAM | Tier |
|---|---|
| ≥ 16 GB | `max` |
| ≥ 8 GB | `pro` |
| < 8 GB | `lite` |

Those tier names reach the deployment templates. On Oracle Cloud
(`deploy/oracle-cloud/main.tf`) they resolve to:

| Tier | OCPUs | Memory |
|---|---|---|
| `lite` | 2 | 4 GB |
| `pro` | 4 | 8 GB |
| `max` | 4 | 24 GB |

**Local model, from system RAM**

| Machine | RAM | Wizard picks |
|---|---|---|
| Raspberry Pi | < 2 GB | `tinyllama:1.1b` |
| Raspberry Pi | < 6 GB | `phi:2.7b` |
| Raspberry Pi | ≥ 6 GB | `mistral:7b` |
| Anything else | ≥ 8 GB | `codellama:7b` |
| Anything else | ≥ 16 GB | `codellama:13b` |
| Anything else | ≥ 32 GB | a cloud model |
| Anything else | < 8 GB | a cloud model |

> **Two things to know about that table.** A machine with **32 GB or more is
> sent to a cloud model**, not a larger local one — the same answer it gives a
> machine with under 8 GB, for the opposite reason. And **the wizard's GPU
> detection does not feed this decision at all**: `detect_gpu()` fills in a
> field that is printed for you and never read by `recommended_model()`. On a
> box with a small system RAM and a large discrete GPU, or the reverse, size it
> yourself from the tables below rather than taking the wizard's word.

---

## GPU: what actually needs one

Three separate workloads can use a GPU, and only one of them usually matters.

| Workload | Uses the GPU | Notes |
|---|---|---|
| **Local LLM inference** | Yes, decisively | The reason to buy one. See below |
| Speech-to-text (whisper) | Optionally | Runs acceptably on CPU; see [Voice](#voice-speech-in-and-out) |
| Text-to-speech (Kokoro) | Yes, via MLX | Apple Silicon only. 82M parameters — small |
| Embeddings / indexing | Optionally | Short bursts, not a sizing constraint |
| Everything else the daemon does | **No** | Agent loop, git, skills, HTTP are CPU and I/O |

**If you use cloud providers only, you do not need a GPU at all.** Nothing in
the agent loop, the editor, the review pipeline or the skills system is
GPU-accelerated, because none of it is arithmetic on tensors.

### What VibeCody can drive

| Path | Hardware | How |
|---|---|---|
| Metal | Apple Silicon | In-process via `mistralrs`, and Kokoro TTS via MLX. Enabled by default in macOS builds |
| CUDA | NVIDIA | Ollama sidecar, or the `vibe-mistralrs-cuda` feature |
| ROCm | AMD | Ollama sidecar |
| CPU | Anything | Works. Slower, and the gap widens with model size |

### Estimating VRAM for a local model

**Estimated, not measured.** Weights dominate, so the first approximation is
parameters × bytes-per-weight, and the quantisation picks the second term:

| Quantisation | ≈ bytes/param | Quality cost |
|---|---:|---|
| FP16 | 2.00 | None — the reference |
| Q8_0 | ~1.06 | Negligible for most work |
| Q5_K_M | ~0.73 | Small |
| Q4_K_M | ~0.60 | Noticeable on hard reasoning; usually fine for code |

Weights alone, rounded:

| Model | FP16 | Q8_0 | Q5_K_M | Q4_K_M |
|---|---:|---:|---:|---:|
| 1.1B (TinyLlama) | 2.2 GB | 1.2 GB | 0.8 GB | 0.7 GB |
| 2.7B (Phi) | 5.4 GB | 2.9 GB | 2.0 GB | 1.6 GB |
| 7B | 14 GB | 7.4 GB | 5.1 GB | 4.2 GB |
| 13B | 26 GB | 14 GB | 9.5 GB | 7.8 GB |
| 34B | 68 GB | 36 GB | 25 GB | 20 GB |
| 70B | 140 GB | 74 GB | 51 GB | 42 GB |

**Then add context.** The KV cache grows with the context you actually use, and
it is not small at long context — budget **1–2 GB on top for ordinary use**, and
considerably more if you intend to fill a 128k window. A 7B at Q4_K_M wants
roughly **6 GB of VRAM** in practice, not 4.2.

On **Apple Silicon** the GPU shares system memory, so "VRAM" is your RAM, minus
what macOS and everything else is using. A 16 GB Mac runs a 7B at Q4 comfortably
and a 13B at Q4 tightly.

> **VRAM is a cliff, not a slope.** A model that fits runs at GPU speed; one
> that overflows by a little spills to system memory or CPU and can get
> dramatically slower — not a few percent. Size to fit, then raise quality.

---

## Voice, speech in and out

Voice is the one subsystem where this repository has real measurements. From
[Full-duplex voice]({{ site.baseurl }}/voice-duplex/):

**Measured**, on an M-series MacBook Air:

| Stage | Time |
|---|---|
| End of speech → first audio, whole pipeline | **134–158 ms** |
| Speech recognition alone | 0.58 s |
| Platform TTS, first audio | 21 ms |
| Kokoro TTS, first audio | 165–230 ms |
| Kokoro model load, first sample (paid once, at daemon start) | 6.6 s |
| A 20B model answering a spoken turn | 5.0 s warm · 42 s cold |

**The model is nearly all of the latency.** Recognition is well under a second
and speech is milliseconds; a large model is seconds of silence. This is why
Settings → Voice can pin a small model for spoken replies independently of the
one the composer uses for writing code.

**Disk**, from `WhisperModel::size_mb` in `vibecli/vibecli-cli/src/voice_local.rs`:

| Whisper model | Download | Use it when |
|---|---:|---|
| `tiny` | 75 MB | Testing, or a Pi |
| `base` | 142 MB | English, quiet room |
| `small` | 466 MB | **The floor for non-Latin scripts.** `base` renders Devanagari in Arabic script |
| `medium` | 1.5 GB | Diminishing returns against `small` |
| `large` | 2.9 GB | Rarely worth it here |

`small` and `medium` were measured to produce identical correct text on
non-Latin input, and `small` is about 3× faster — start there.

Text-to-speech: the platform engine costs nothing and needs no download. Kokoro
is a **82M-parameter** neural model — negligible next to any LLM, and Apple
Silicon only. See [Choosing a voice
engine]({{ site.baseurl }}/voice-duplex/#choosing-a-voice-engine).

---

## Embeddings and the code index

**Disk is the cost, and it is predictable.** Each index is roughly:

```
chunks × dimensions × 4 bytes
```

A 768-dimension model such as `nomic-embed-text` is therefore ~3 KB per chunk.
Ten thousand chunks is about 30 MB. Dimensions are per-model and configurable —
see [Embeddings]({{ site.baseurl }}/embeddings/).

Building an index is a burst of CPU or a burst of API calls, depending on the
provider. It is not a standing cost, and it is not a reason to buy a GPU.

---

## Per-target recommendations

### Desktop, one person

| | Minimum | Comfortable |
|---|---|---|
| RAM | 8 GB | 16 GB+ |
| Disk | 2 GB | 10 GB with local models |
| GPU | None (cloud providers) | Apple Silicon, or 8 GB+ NVIDIA for local 7B |

The desktop apps start the daemon themselves. Nothing to size separately.

### Always-on server

| | Cloud-provider only | With a local 7B |
|---|---|---|
| vCPU | 1–2 | 4+ |
| RAM | 2 GB | 16 GB |
| Disk | 5 GB | 20 GB |

Oracle Cloud's always-free 4 ARM cores + 24 GB fits the second column at **$0** —
see [Deployment guides]({{ site.baseurl }}/guides/).

### Raspberry Pi

| Board | RAM | Local model | Realistic use |
|---|---|---|---|
| Pi 5 | 8 GB | `mistral:7b` quantised | Slow but real local inference |
| Pi 4 | 4–8 GB | `phi:2.7b` | Light local work |
| Pi 3 | 1 GB | `tinyllama:1.1b` | Test only |
| Any Pi | any | — | **Excellent as a cloud-provider relay** |

A Pi pointed at a cloud provider is a fine always-on daemon and RAM stops
mattering. Local inference on a Pi is a demonstration, not a workflow.

---

## Two costs that are not hardware

**A cold daemon has been measured at ~16 s to first answer `/health`.** That is
not slow hardware; it is startup work. It matters for serverless platforms that
scale to zero, and it is why nothing in this codebase may sleep a fixed guess
and then check once.

**Context windows come from the provider, not from your RAM.** Each model's
budget is read from its vendor's API. See
[Configuration]({{ site.baseurl }}/configuration/).
