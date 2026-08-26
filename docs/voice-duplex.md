---
layout: page
title: Full-duplex voice
permalink: /voice-duplex/
---

# Full-duplex voice

The microphone stays open while the assistant is speaking, so you can interrupt
it mid-sentence and it stops. That is the difference from push-to-talk
dictation, which is still available and still the right control for composing a
long prompt.

```
client mic → AEC → 16 kHz PCM ─ws─► VAD → ASR → provider → TTS ─ws─► client
                                     ▲                                │
                                     └────── barge-in cancels ────────┘
```

**The pipeline lives in the daemon** (`vibecli/vibecli-cli/src/voice_duplex.rs`,
route `GET /ws/voice/duplex`). Clients contribute a microphone and speakers and
nothing else — turn-taking, transcription, the model call and speech synthesis
all happen in one place, so a surface gains the feature by connecting a socket
rather than by reimplementing a pipeline.

## Why echo cancellation decides which surfaces qualify

With an open microphone the client hears its own playback. Without acoustic
echo cancellation the agent's own voice trips the voice-activity detector and it
interrupts itself on every sentence — so AEC is not a quality nicety here, it is
the precondition.

Measured in a `WKWebView` with `tools/webview-probe --arm aec`: **≥40 dB of
suppression**, and — the part that mattered — it covers **WebAudio-rendered**
playback, not only WebRTC remote tracks. Two valid runs on the same hardware
measured 40.8 dB and 58.2 dB, so read it as "the tone is driven below the noise
floor", not as a calibrated constant.

A surface without AEC should stay on `POST /voice/transcribe` push-to-talk.

## What a host has to allow

Capture prefers an `AudioWorklet`, which runs off the main thread. Its module is
fetched under **`script-src`** — not `worker-src`, which is a common and costly
assumption — so a host shipping `script-src 'self'` rejects the blob: module
with *"Not allowed by CSP"* and voice fails to start.

Hosts should allow it:

```
script-src 'self' blob:
```

Where they do not, capture falls back to a `ScriptProcessorNode`, which fetches
no module and works under any policy. It is deprecated and runs on the main
thread, so the worklet is preferred where the policy permits — but the feature
never depends on a host's CSP to function at all.

## Surfaces

| Surface | Duplex | Why |
|---|:--:|---|
| **VibeCoder** | ✅ | WKWebView AEC, measured |
| **VibeDesk** | ✅ | same |
| **VibeAIChat** | ✅ | same |
| Daemon web client (`/web`) | ⬜ | browser AEC applies; not yet wired |
| VS Code extension | ⬜ | webview can `getUserMedia`; not yet wired |
| JetBrains plugin | ⬜ | `VoiceRecorder` is JVM audio — needs its own AEC |
| VibeMobile (Flutter) | ⬜ | platform AEC exists; needs a native audio path, not a webview |
| VibeWatch / Wear | ❌ | push-to-talk is the right interaction on a watch |
| Neovim plugin | ❌ | no audio surface |
| VibeCLI (terminal) | ❌ | no AEC; `--voice` push-to-talk instead |

⬜ = the transport is ready and the surface is capable; the client work is not
done. ❌ = not appropriate, and saying so is the point — a duplex control that
makes the assistant talk over itself is worse than no control.

**Windows and Linux are unverified.** The engines differ (`WebView2` is
Chromium, Linux is WebKitGTK) and only macOS has been measured. Run
`tools/webview-probe --arm transport` and `--arm aec` on those platforms before
claiming them; the transport arm is a CI gate, the AEC arm needs a real
microphone and speaker in one room.

> **Linux caveat.** WebKitGTK ships media capture *off* and denies the
> permission request unless the embedder answers it, and neither `wry` nor
> Tauri does. Until that is fixed, microphone capture does not work on Linux at
> all — duplex or push-to-talk. See `tools/webview-probe`'s
> `apply_linux_media_fix` for the two calls required.

## Latency

Measured on an M-series MacBook Air, end of speech to first audio:

| Stage | Streaming ASR | Batch ASR |
|---|---:|---:|
| ASR | 35–54 ms | 360–460 ms |
| Model first token | 54–151 ms | same |
| TTS first audio | 15–25 ms | same |
| **Total** | **134–158 ms** | 430–640 ms |
| VAD hangover before any of it | 600 ms | 600 ms |

The first build measured **1076 ms**, of which whole-utterance Whisper was 978 ms.
The win was not a faster model — it was **overlapping recognition with speech**,
so end of turn only has to finalise.

Three things must be warm before a user is invited to speak, and the daemon does
all three on connect: the speech synthesiser (~300 ms first utterance, ~20 ms
after), the model (**3796 ms** cold against ~85 ms warm), and the recogniser.

## Interruption, and what is *not* an interruption

Two things look alike and are not:

* The user talks **while the assistant is speaking** — a real interruption.
  Playback stops, the reply is abandoned, and it does not come back.
* The user talks **while the assistant is still thinking**, before any audio has
  gone out. That is someone finishing a thought, not interrupting one.

The second case used to discard the turn silently. `"plus fifty one."` <pause>
`"minus fifty four."` is one instruction said in two breaths — the 600 ms
hangover ends a turn on the pause — and dropping the first half answered a
different question (`32 - 54` instead of `32 + 51 - 54`) with nothing on screen
to say why.

Words from a turn superseded before it spoke are now **carried into the next
turn**, and the client is told with a `carried` event so nothing vanishes
silently either way.

## Languages

`language=en` keeps the fast path. `language=auto` detects per turn across 99
languages; a code pins one. The reply is instructed into the detected language
and the voice follows it.

**Detection runs every turn in auto mode, deliberately.** Pinning a language
after the first turn suppresses the engine's detection result, so a user who
says one Hindi sentence and then an English one gets the English turn labelled
Hindi and answered in Hindi. Code-switching is the normal case for multilingual
speakers, not an edge case.

Non-English costs ~455 ms rather than ~40 ms, because the fast streaming
recogniser is English-only on most machines: `SFSpeechRecognizer` advertises 63
locales but only those with an installed offline asset can run on-device, and a
default macOS install has four, all English. Installing more (System Settings →
Keyboard → Dictation) moves those languages onto the fast path.

**Model choice matters for non-Latin scripts.** `ggml-base` renders Devanagari
in Arabic script; `ggml-small` and `ggml-medium` produce identical correct text
and `small` is 3× faster, so `small` is the default and the floor.

## Configuration

```toml
[voice]
whisper_server_bin   = "whisper-server"                 # resident, not per-utterance
whisper_server_model = "~/.vibecli/models/ggml-small.bin"
whisper_server_port  = 8923
tts_sidecar          = "/path/to/tts"                   # optional streaming TTS
```

Without `tts_sidecar` every platform still speaks — `say` / `espeak` /
PowerShell to a WAV — just with the whole utterance synthesised before the first
sample goes out. That is a latency difference, not a capability one.

Running `whisper-server` resident rather than spawning `whisper-cli` per
utterance is worth ~1 s: `small` measured 1433 ms total against 570 ms of actual
encode, because every turn was paying model load *and* backend init.
