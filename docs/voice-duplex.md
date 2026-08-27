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

## It is off until you turn it on

Duplex holds the microphone open for the whole session — that is what makes it
interruptible — so it is **disabled by default** and stays that way until
someone enables it. "Idle until clicked" would not be the same promise: it
leaves a live control one misclick away from an open mic.

The control in the chat window has two states:

* **Voice off** — one click enables the feature. It does *not* open the
  microphone; starting is a second, deliberate click.
* **Enabled** — a start/stop button showing the turn state, and an **×** that
  turns the feature back off. Switching it off closes the microphone rather
  than merely hiding the control that was holding it open.

The preference is stored per machine under `vibe.voice.duplexEnabled` and shared
across the shells: someone who turns voice off in VibeCoder does not expect
VibeDesk to keep offering it. Anything other than exactly `true` reads as off —
a microphone is not the place to be generous about what counts as consent.

The hook enforces this too, not just the button: hiding a control is not the
same as refusing to open a device, and the hook is what actually opens one.

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

## Choosing a voice engine

Two engines, and the choice is a latency/quality trade with no free answer.

| `[voice] tts_engine` | first audio | needs | platforms |
|---|---:|---|---|
| `system` *(default)* | **21 ms** | nothing | macOS · Windows · Linux |
| `kokoro` | **165–230 ms** | Python + `mlx-audio` | Apple Silicon |

`system` is the platform voice. On macOS that is `AVSpeechSynthesizer`, and the
single biggest quality win available there costs nothing and is not a code
change: **Apple's Enhanced and Premium voices are neural, free, and separate
downloads.** A Mac with none installed speaks in the compact tier, which is what
"the assistant sounds mechanical" usually means. System Settings →
Accessibility → Spoken Content → System Voice → Manage Voices. The daemon picks
the best installed voice automatically.

`kokoro` is Kokoro-82M running through MLX. Neural, 54 voices, Apache-2.0, and
about 8× slower to first audio than the platform engine — see the numbers in
`tools/tts-bench`, which measures both rather than estimating.

### Turning on Kokoro

The daemon cannot ship a Python environment, so this is explicit setup:

```bash
uv venv --python 3.12 ~/.vibecli/tts
VIRTUAL_ENV=~/.vibecli/tts uv pip install mlx-audio "misaki[en]"
```

```toml
[voice]
tts_engine = "kokoro"
tts_sidecar = "~/.vibecli/tts/bin/python"
tts_sidecar_args = ["/path/to/tools/voice-duplex/sidecar/tts_kokoro.py"]
kokoro_voice = "af_heart"
```

If the interpreter or the packages are missing, the daemon **says so on the
socket** and falls back to the platform voice. It does not fall back silently:
that failure is inaudible in the only sense that matters, because it sounds
exactly like never having configured anything.

### Why it splits sentences at commas

Kokoro is non-autoregressive — a sentence is produced in one pass, so first
audio is the whole sentence's synthesis time. Measured on an M-series Mac, a
full sentence takes 386–416 ms; split at the comma and the first clause goes out
in **165–228 ms** while the rest synthesises behind it. Total synthesis rises
about 15%, which costs nothing at a real-time factor near 0.1 — playback never
catches up with generation.

The cost is prosody: every clause gets sentence-final intonation, so a long
reply is slightly choppier than one pass would be. Send
`{"cmd":"clauses","on":false}` to the sidecar to hear the difference.

### The frame carries its own sample rate

`AVSpeechSynthesizer` produces 22.05 kHz and Kokoro 24 kHz. A wrong sample rate
does not fail — it plays at the wrong pitch and speed — so audio frames are
`AUR`: a `u32` rate followed by `f32` samples. The original `AUD` frame is still
read as 22.05 kHz, so a daemon can drive a sidecar built before this existed.

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

## What the assistant knows about your project

The socket carries a `set_context` control message, and the client sends it on
connect and again whenever the workspace changes:

```json
{ "type": "set_context", "context": "Open file: src/main.rs\n\nProject files (400 of 5121):\n…" }
```

The daemon folds it into the turn's system prompt, bounds it at 32k characters,
and treats an empty block as *clear* rather than *unchanged* — closing a project
mid-conversation must not leave the assistant answering about the old one.

VibeCoder sends the same material the typed chat path sends: pinned memory, the
context block, the open file, and the head of the file tree. VibeDesk and
VibeAIChat send nothing, because neither has a workspace open to describe.

Without this the assistant answered *"I don't have any information about that"*
about the project on screen beside it — voice was the one surface that never
received the context every other path already had.

The prompt asks it to answer from the context and to say when the context does
not say. It is not asked to be helpful about files it cannot see: an assistant
that invents a file name out loud is harder to catch than one that prints it.

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

## Reasoning models say the answer, not the deliberation

A reasoning model narrates its way to an answer, and Ollama returns that
narration in a separate `thinking` field which the provider splices into the
token stream as `<thinking>…</thinking>`. Spoken unfiltered, the assistant read
its own deliberation aloud — *"The user says: Hey, how are you doing? As a voice
assistant, respond in one or two short spoken sentences…"* — and only then
answered.

Reasoning is now filtered out of the stream before the sentence splitter sees
it, so it is neither spoken nor shown. `<think>`, `<thinking>` and namespaced
forms like `<mm:think>` are all suppressed, as is `<tool_call>` markup.

The filter is a state machine rather than a per-chunk strip, because tokens are
not tag-aligned: `<thin` and `king>` routinely arrive in different chunks, and a
per-chunk strip both misses the tag and leaks its two halves into the speaker.
An unterminated block is discarded — it *is* reasoning.

`llm_ttft_ms` is still measured on the raw stream, so on a reasoning model it
sits far below `first_audio_ms`. That gap is real: the model was producing
tokens the whole time, just none you were meant to hear.

**A reply that was nothing but reasoning says so.** Filtering can leave nothing
behind, and a turn that ends with an empty reply is a chat log that skipped a
turn and a speaker that stayed quiet — from where the user sits, identical to a
microphone that never worked. The daemon distinguishes the two silences it can
tell apart: tokens arrived and none survived the filter, or no tokens arrived at
all. Neither is reported as an answer.

A turn failure like this leaves the socket open — the conversation is still
live, and the next thing you say is a new turn — so the control on screen stays
the one that stops it. Deriving that from the turn state alone used to offer
"start" on a microphone that was already open, with nothing left that could
close it.

## One reply is one turn

`speaking` fires **once per sentence**, because that is what drives streaming
TTS — waiting for the whole reply would make first audio inherit the entire
generation time. It is live text, not a turn.

`reply` fires once, with the model's own text, and that is the turn. A host
building a chat log should append on `reply` and use `speaking` only for live
display; the shared `useVoiceDuplex` hook already draws this line, exposing
sentences on `turns` and calling `onTurn` only for completed turns.

Appending each `speaking` event instead rendered a single three-sentence answer
as three separate chat bubbles. `reduceTurns` in the hook is a pure function so
the property — one reply, one turn — is unit-tested without a microphone.

## A spoken turn leaves a record

Everything above is audio, and audio has already stopped by the time you want
to reread it. Two surfaces write it down, and they are deliberately different
things:

**The chat log** gets each turn once it is complete, via `onTurn` — your
transcription when it lands, then the whole reply. It is indistinguishable from
a typed exchange, so a session where you spoke half the questions still reads as
one thread. All three shells wire it; VibeDesk did not, which is why a whole
voice conversation there used to happen with nothing on screen at all.

**The caption** (`VoiceTranscript`, above the composer) covers the seconds in
between: the sentence being spoken right now, under the question it answers.
It renders the tail of `turns` and nothing older, because everything older is
already in the chat log — showing the lot would render every turn twice.

The caption is also where a failure shows up. It survives `active` going false,
since the hook tears the conversation down on a failed start — which is exactly
the moment there is something to explain.

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

**How the engine is resolved**, in order:

1. **A server already listening** on `whisper_server_port` is used as-is —
   whether or not a binary can be found to start another. Asking the port first
   means an existing server is never ignored because of a path that did not
   resolve.
2. Otherwise `whisper_server_bin` is resolved: a **bare name** is looked up on
   `PATH`; anything containing a separator is taken as a literal path and used
   only if it exists.
3. If neither yields a server, duplex voice reports what it looked for and the
   route stays unavailable. Push-to-talk (`POST /voice/transcribe`) is
   unaffected — it has its own engine resolution.

**TTS defaults to batch.** Without `tts_sidecar` the whole utterance is
synthesised before the first sample goes out, so first-audio is a few hundred
milliseconds rather than ~20 ms. Correct, just slower; `ready` reports which
path is in use (`"tts":"streaming"` or `"tts":"batch"`).

## Troubleshooting

**"No speech engine."** The message names the binary it looked for, the model
path, and the port nothing was listening on. Usually one of: whisper.cpp is not
installed, the model has not been downloaded to
`~/.vibecli/models/ggml-small.bin`, or `whisper_server_bin` points somewhere
that does not exist. A bare name on `PATH` is fine — that is resolved.

**"Audio capture was blocked by this app's content security policy."** The host
ships `script-src` without `blob:`, so the AudioWorklet module cannot be
fetched. Capture falls back to a `ScriptProcessorNode` automatically, so this
should not surface as a failure; if it does, the host's policy is blocking
something else as well.

**"Could not reach the daemon's voice route."** The daemon is not running, or
predates `/ws/voice/duplex`. Check `GET /health` and its `version`.

**Push-to-talk stopped working after a failed voice attempt.** Fixed — a
half-started duplex attempt used to keep the microphone open, which then denied
it to push-to-talk. If you see it again on an older build, quitting and
reopening the app releases the device.

**The assistant answers the wrong thing after you pause mid-sentence.** See
[Interruption, and what is *not* an interruption](#interruption-and-what-is-not-an-interruption)
— fragments are carried forward now, but a pause longer than the reply takes to
start will still be answered as its own turn.

## Looking at the project

A spoken turn can read the workspace before it answers. Without it, "summarise
this project" got what the client happened to preload — a list of paths — and
the assistant said so: *"just a collection of directories and files."*

Two control messages set this up, both sent on connect and again whenever they
change:

| message | payload | effect |
|---|---|---|
| `set_context` | `{ "context": "<text>" }` | the `<workspace>` block in the system prompt. Empty clears it |
| `set_workspace` | `{ "root": "/abs/path" }` | enables the tools, jailed to that directory. Empty or missing → no tools |
| `set_capabilities` | `{ "open_file": true }` | offers `open_file`. Absent means no |

With a root, the turn may answer with **one tool call and nothing else**:

```xml
<tool_call name="read_file"><path>README.md</path></tool_call>
<tool_call name="list_directory"><path>src</path></tool_call>
<tool_call name="search_files"><query>fn main</query></tool_call>
```

The names are the agent's own — `read_file`, `list_directory`, `search_files`,
`write_file`, `apply_patch` — and that is not a detail. The contract shipped
advertising `list_dir`, which `parse_tool_calls` does not know, so a model that
did exactly what it was told produced a call that parsed to nothing: no tool
ran, no answer was spoken, and the user was told the model "never answered" for
following the instructions. **A prompt is an interface**, so the examples in the
contract are now parsed by a test — the specification is checked against the
implementation that has to honour it.

The daemon executes it through the same path-guarded `ToolExecutor` the agent
uses, feeds the result back, and asks again. Bounds, because every round is
silence in a conversation rather than a progress bar: **2 rounds**, **2 calls
per round**, **4k characters per result**, and the final pass must answer. A
`{"type":"tool","text":"Reading README.md"}` event goes to the client so the
caption can say what the pause is for.

**The reasoning filter has two modes, and the voice turn needs the other one.**
`StreamFilter` suppresses `<think>`, `<thinking>` *and* `<tool_call>` — right
for the agent console, which renders tool use as its own structured line and
must never print the raw call. The voice turn has to **run** the call, and the
filter sits upstream of the tool gate, so the default mode ate every call
before the gate could see one. `StreamFilter::reasoning_only()` drops reasoning
and passes tool markup through; keeping it away from the speaker is the gate's
job, which is what the gate is for.

**When to look is a rule, and it changes with the tools.** Without a root the
assistant is told to say it cannot tell from what it can see. With one, that
sentence is only true *after* it has looked — so the rule becomes: answer from
the `<workspace>` block when it says enough, and open the file that would
answer when it does not. The two have to be written as one instruction. When
"say you cannot tell" was stated first and the tool contract came after, asking
the assistant to summarise a project made it take the earlier, easier rule and
refuse — with the README a single `read_file` away.

### Changing something

`write_file` and `apply_patch` are available too — but nothing changes until
the user agrees:

1. the daemon **speaks** the question ("May I write `src/main.rs`? It replaces
   the file with 12 lines.") — the user may not be looking at the window;
2. it sends `{"type":"approval_request","question":…}`, and the client renders
   Yes / No;
3. the client answers `{"type":"approval","approved":true|false}`, and the
   daemon replies `{"type":"approval_resolved","approved":…}` so the prompt
   leaves the screen whatever happened.

Consent is a click, not a word. "Yes" is a word a microphone can mishear, and
the cost of mishearing it is an overwritten file — so hearing the question is
how you learn there is one, and clicking is how you agree. **A timeout (90 s),
a closed socket and a malformed answer are all refusals**, and a refusal is
reported to the model in words so it tells the user rather than trying again.

`bash` is not reachable from a spoken turn at all, approval or not: a spoken
"yes" to `rm -rf` is the same word as a spoken "yes" to a formatter, and a
speaker cannot show you which one you are agreeing to.

### Showing something

*"Can you open `serve.rs`."* — the assistant read it, described it, and the
editor never moved. Every half of that worked: the tool ran, the answer was
spoken, the chat log recorded it. What was missing was the idea that the
assistant could **do** something to the screen rather than only report on it.

`open_file` is the one tool the daemon cannot execute. It resolves the path
against the workspace root, confirms the file exists, and sends the client an
action:

```jsonc
{ "type": "ui", "action": "open_file", "path": "/abs/path/src/main.rs", "relative": "src/main.rs" }
```

Three things are deliberate about it.

**Opening is not reading, and the prompt has to say so.** A model asked to
"open the config" reaches for `read_file` and describes what it found — an
answer to a question nobody asked. The contract distinguishes the two by what
they are *for*: `open_file` is what the user asked for, `read_file` is for when
the assistant needs the contents in order to answer.

**The client says whether it has an editor; the daemon does not guess.**
VibeDesk and VibeAIChat run the same hook against the same daemon and have
nowhere to put a file, so the clause is only added to the contract for a client
that sent `set_capabilities`. In the shared hook that declaration is derived
from the presence of an `onOpenFile` handler rather than configured beside it —
two switches for one fact drift apart, and the shape they drift into is an
assistant that says "I've opened that for you" over an editor that did not
move.

**The path is checked here, not trusted.** "Open my ssh key" is a sentence a
microphone can pick up. The path is canonicalised and required to name an
existing file inside the workspace root — the same rule the executor applies to
a read — and a path that fails is reported to the model as *not* opened, so it
tells the user rather than claiming success. Both forms travel: the client
opens by absolute path because its file tree is built from them, and the
caption and transcript read the relative one.
