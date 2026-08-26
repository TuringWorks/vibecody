# voice-duplex

A full-duplex voice loop against a fully local stack. The microphone never
closes — including while the assistant is speaking — so you can interrupt it
mid-sentence and it stops.

```
mic → AEC → 16 kHz PCM → ws → VAD → whisper → ollama → AVSpeech → ws → speakers
                               ↑                                        │
                               └──────── barge-in cancels ──────────────┘
```

## Why this is possible at all

The open mic only works because the webview's echo canceller removes our own
output from the capture stream — measured at **≥40 dB**, and, critically, it
covers **WebAudio-rendered** output rather than only WebRTC remote tracks
(`tools/webview-probe --arm aec`). Without AEC an open mic makes the agent
interrupt itself on its own voice, and barge-in is impossible.

## Transport: WebSocket PCM, not WebRTC

Deliberate. On loopback, WebRTC's loss concealment and jitter buffer buy
nothing, and AEC lives in the *capture* pipeline so it applies either way.
WebRTC remains the right answer for the **remote** case — phone or watch to a
daemon over Tailscale — which is a different problem with real packet loss.

## Measured, on an M-series MacBook Air

End of speech → first audio, steady state over 4 runs:

| Stage | Streaming (default) | Batch (`--asr whisper`) |
|---|---:|---:|
| ASR | **35–54 ms** | 284–419 ms |
| LLM first token — ollama `granite4.1:3b` | 80–88 ms | 80–88 ms |
| TTS first audio | 15–25 ms | 15–25 ms |
| **Total** | **134–158 ms** | 380–530 ms |
| VAD hangover before any of it starts | 600 ms | 600 ms |

The first build of this loop measured **1076 ms**, of which whole-utterance
Whisper was 978 ms — 91%. That inverted the usual claim that the LLM is the
bottleneck, which quietly assumes *streaming* ASR.

**The win is not a faster model. It is that recognition overlaps the speech**,
so end of utterance only has to finalise. `sidecar/asr` streams frames to
on-device `SFSpeechRecognizer` while the user is still talking and emits interim
hypotheses (the UI shows them appearing); at end of turn the final arrives in
~35 ms because almost all the work is already done.

Three warm-ups happen before anyone is invited to speak: `AVSpeechSynthesizer`
(~300 ms first utterance, 15–25 ms after), ollama (**3796 ms** cold vs ~85 ms
warm), and the recogniser session.

Streaming ASR here is **macOS-only**. Linux and Windows need Moonshine v2 or
Parakeet TDT to reach the same shape; `--asr whisper` is the portable fallback
and the table above is what it costs.

## Run

```bash
swiftc -O -o sidecar/tts sidecar/tts.swift
cargo run --release                      # opens the window
cargo run --release -- --selftest        # headless: proves the pipeline, prints the budget
```

Flags: `--model`, `--ollama`, `--whisper`, `--whisper-model`, `--tts`,
`--http-port`, `--ws-port`. `VD_TRACE=1` traces sidecar frames.

## Interruption vs. finishing a thought

Barge-in conflated two different events. A turn superseded *while the assistant
is speaking* is a real interruption and its reply is abandoned. A turn
superseded *before any audio went out* is a user still forming their thought —
the 600 ms hangover ends a turn on a breath — and its words are now carried into
the next turn instead of being dropped.

Dropping them was not cosmetic: `"plus fifty one."` <pause> `"minus fifty
four."` answered `32 - 54` instead of `32 + 51 - 54`, with the transcript on
screen and no reply and no explanation.

## Four traps this found, all worth remembering

Three are in `AVSpeechSynthesizer.write`:

- It **never invokes the callback at all** for an utterance with nothing
  speakable in it — no buffers, no completion. A reader waiting for a
  terminator deadlocks. The sidecar answers that case itself, and
  `Tts::next_chunk` is bounded so no stage can hang the loop.
- It calls back **more than once** with a zero-length buffer per utterance, and
  those late duplicates arrive *after the next utterance has started*. Hence
  per-utterance ids: a bare "done" flag gets reset by the next utterance and the
  stale callback then truncates it to silence.
- The `AVAudioPCMBuffer` is **recycled when the callback returns**. Deferring
  the read to another queue yields `frameLength == 0`, which is exactly the
  end-of-utterance sentinel. Copy synchronously, dispatch second.

The fourth was mine, in the *test harness*, and it cost more than the other
three combined: a consumer that does not drain stdout continuously blocks this
process on a full 64 KB pipe partway through an utterance. That looks identical
to "the synthesizer stopped delivering callbacks". It sent me to a fresh
synthesizer per utterance (~185 ms) before a correct harness showed reuse was
fine all along at 15–21 ms. **When a measurement indicts a dependency, check
the instrument first.**

## Not yet

Linux and Windows are untested here; the engine differences are real and
`tools/webview-probe` is what measures them. WebKit exposes no
`autoGainControl` or `noiseSuppression`, so on that engine both would have to
be done in the pipeline rather than requested from the browser.
