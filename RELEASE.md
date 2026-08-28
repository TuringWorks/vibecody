# VibeCody v0.5.11 Release

**137 commits since v0.5.11.**

---

## What's in v0.5.11

The voice release. An open microphone you can talk over, a neural voice to
answer in, and a spoken turn that can read the project you have open and act on
it. Around that: composers rebuilt around one `+` menu across all three shells,
Charcoal as the default theme everywhere, DOCX/EPUB/Pages editing, and every
model's context budget read from its own provider instead of a constant nobody
had checked.

The theme underneath is the same one as the last two releases, and it kept
paying out. A feature that renders is not a feature that works — and this time
the sharpest example is a feature that had never run at all: for the whole life
of the speech pipeline, nothing built or installed the sidecar it needs, so
every spoken reply came from the slowest path in the system default voice. The
code was correct, inert, and the symptom was "the assistant sounds mechanical".

### Headline features

- **Full-duplex voice.** The microphone stays open while the assistant speaks
  and you can interrupt it mid-sentence. The whole pipeline lives in the daemon
  — voice-activity detection, turn-taking, transcription, the model call,
  synthesis — so a client contributes a microphone and speakers and nothing
  else. **Measured end of speech to first audio: 134–158 ms.** The first build
  measured 1076 ms, of which whole-utterance Whisper was 978 ms; the win was not
  a faster model but overlapping recognition with the speech, so end of turn
  only has to finalise. It is off until you turn it on, because it holds the
  microphone open for the whole session. Measured on macOS only.

- **Any language you speak in.** Detection runs *every* turn across 99
  languages, the reply is instructed into that language, and a voice that speaks
  it is chosen to read it. Not pinned after the first turn: forcing a language
  suppresses the engine's own detection, so a user who said one Hindi sentence
  and then an English one had the English turn labelled Hindi and answered in
  Hindi. Code-switching is the normal case for multilingual speakers.

- **A neural voice, and something that installs it.** Kokoro-82M through MLX —
  28 voices, 9 languages, Apache-2.0, Apple Silicon. **Settings → Voice** in all
  three shells picks engine, language and voice, and each engine row says
  whether it can run here *and why not*, because a greyed-out option with no
  reason reads as a bug. `make voice-sidecar` and `make voice-kokoro` install
  the engines; `make voice-status` answers which one will actually run. On macOS
  try the free thing first: Apple's Enhanced and Premium voices are neural, cost
  nothing at synthesis time, and are separate downloads most Macs have never
  made.

- **A spoken turn can act on the project.** "Open that file" opens it in the
  editor rather than describing it — opening is not reading, and the prompt has
  to say so. The assistant also sees the workspace, the file tree and the open
  file, context every other path already had and the voice path never did. The
  path is checked, not trusted: "open my ssh key" is a sentence a microphone can
  pick up.

- **Composers rebuilt, in all three shells.** One `+` menu replaces the row of
  permanent buttons, controls are grouped by what they describe — the message,
  the run, the standing conditions — rather than listed as eight equal pills,
  and the send button no longer clips out of a narrow sidebar. VibeCoder's chat
  card is a query container, so its controls collapse on *its* width and not the
  window's.

- **Charcoal is the default theme**, on every surface including the first paint
  before any JavaScript runs. The default is named once, because the old
  fallbacks had already drifted into three different palettes. Existing installs
  keep the theme they chose.

- **DOCX, EPUB and Pages documents open in the editor** as documents rather than
  as a placeholder, with EPUBs rendered as books.

- **Every model gets its own context budget, read from the provider.** The agent
  loop pruned at a flat 200 000 tokens and the chat panel compacted at 80 000
  characters; neither number had anything to do with the model in front of you
  and both failed silently. Ollama drops the *front* of an oversized prompt,
  taking the system prompt and the tool contract with it, so the symptom was a
  model that appeared to forget how to call tools. There is deliberately no
  table of numbers.

### Notable fixes

- **The voice assistant could not use a single one of the tools it was given.**
  Two independent defects, either sufficient alone: the streaming filter that
  suppresses `<tool_call>` sat upstream of the tool gate and swallowed every
  call before the gate could see it, and the spoken contract advertised
  `list_dir`, a name the parser has never known. The examples in the contract
  are now parsed by a test — a prompt is an interface.

- **It read its own reasoning aloud.** Asked "how are you doing?", a reasoning
  model spoke two paragraphs of deliberation before the one-line answer.

- **A question asked in Hindi was answered in fluent, correct English.** All
  three shells sent `language: "en"`, which suppresses detection rather than
  biasing it.

- **Every `PUT` and `DELETE` route in the daemon was unreachable from every
  browser client.** They were missing from the CORS method list, so the browser
  refused the preflight before sending and the caller got a bare transport error
  with no status code to explain it. Three PUT routes and fifteen DELETE ones.

- **The git diff showed almost every line as added.** The "before" side was
  rebuilt in the frontend by walking the unified diff — which carries only the
  changed hunks. Measured on a 500-line file with one line changed: ~492 lines
  reported as added.

- **A commit in the history diffed against the working tree, not the commit**,
  so while browsing history a commit looked like it had changed nothing.

- **VibeCoder rendered the voice controls with no stylesheet at all**, for the
  whole life of the feature. The stylesheet was correct and fully themed, which
  is why nothing looked wrong in review.

- **Streamed replies re-parsed the whole transcript per token** in VibeCoder and
  VibeAIChat.

- **VibeAIChat's menu-bar icon was a white square** — macOS draws a template
  image from its alpha channel alone, and the icon was 96% opaque.

---

## Downloads

### VibeCLI — Terminal AI Assistant

| Platform | File |
|----------|------|
| macOS (Apple Silicon) | `vibecli-aarch64-apple-darwin.tar.gz` |
| macOS (Intel) | `vibecli-x86_64-apple-darwin.tar.gz` |
| Linux x86_64 (static musl) | `vibecli-x86_64-linux.tar.gz` |
| Linux ARM64 (static musl) | `vibecli-aarch64-linux.tar.gz` |
| Windows x64 | `vibecli-x86_64-windows.zip` |
| Docker | `vibecli-docker-v0.5.11.tar.gz` |

### VibeCoder — Desktop Code Editor

| Platform | File |
|----------|------|
| macOS (Apple Silicon) | `VibeCoder_0.5.11_aarch64.dmg` |
| macOS (Intel) | `VibeCoder_0.5.11_x64.dmg` |
| macOS (`.app`) | `VibeCoder-macOS-{arm64,x64}.app.zip` |
| Linux x64 / arm64 (`.deb`) | `VibeCoder_0.5.11_{amd64,arm64}.deb` |
| Linux x64 / arm64 (`.AppImage`) | `VibeCoder_0.5.11_{amd64,aarch64}.AppImage` |
| Windows x64 | `VibeCoder_0.5.11_x64_en-US.msi` · `VibeCoder_0.5.11_x64-setup.exe` |

### VibeAIChat — Desktop AI Assistant

| Platform | File |
|----------|------|
| macOS (Apple Silicon) | `VibeAIChat_0.5.11_aarch64.dmg` |
| macOS (Intel) | `VibeAIChat_0.5.11_x64.dmg` |
| Linux x64 / arm64 (`.deb`) | `VibeAIChat_0.5.11_{amd64,arm64}.deb` |
| Linux x64 / arm64 (`.AppImage`) | `VibeAIChat_0.5.11_{amd64,aarch64}.AppImage` |
| Windows x64 | `VibeAIChat_0.5.11_x64_en-US.msi` · `VibeAIChat_0.5.11_x64-setup.exe` |

### VibeDesk — Desktop Task Shell

| Platform | File |
|----------|------|
| macOS (Apple Silicon) | `VibeDesk_0.5.11_aarch64.dmg` |
| macOS (Intel) | `VibeDesk_0.5.11_x64.dmg` |
| Linux x64 / arm64 (`.deb`) | `VibeDesk_0.5.11_{amd64,arm64}.deb` |
| Linux x64 / arm64 (`.AppImage`) | `VibeDesk_0.5.11_{amd64,aarch64}.AppImage` |
| Windows x64 | `VibeDesk_0.5.11_x64_en-US.msi` · `VibeDesk_0.5.11_x64-setup.exe` |

### VibeCody Mobile

| Platform | File |
|----------|------|
| iOS (unsigned `.ipa`) | `VibeCody-Mobile-v0.5.11-ios.ipa` |
| Android | `VibeCody-Mobile-v0.5.11-android.apk` · `.aab` |

### VibeCody Watch

| Platform | File |
|----------|------|
| watchOS 10+ (unsigned `.app.zip`) | `VibeCody-WatchOS-v0.5.11.app.zip` |
| Wear OS 3+ | `VibeCody-Wear-v0.5.11.apk` · `.aab` |

---

## Quick Install

```bash
# One-liner (Linux/macOS)
curl -fsSL https://raw.githubusercontent.com/TuringWorks/vibecody/main/install.sh | sh

# Docker (air-gapped / on-prem)
docker load < vibecli-docker-v0.5.11.tar.gz
docker run -p 7878:7878 vibecli:v0.5.11

# Verify
vibecli --version   # Should print: vibecli 0.5.11
```

---

## macOS code signing

Developer ID signing has covered the whole macOS surface since v0.5.8, the
`vibecli` binary included; before that it shipped with **no signature at all**
and the three app bundles were ad-hoc. Nothing about signing changed in v0.5.11.

Whether a given download is signed depends on the build that produced it — the
release workflow signs only when the signing secrets are configured, and says
so in the build log when they are not. Check the artifact in your hands:

```bash
codesign -dv --verbose=4 /Applications/VibeCoder.app
# expect: Authority=Developer ID Application: Ravindra Boddipalli (N7HV58M58W)
# "Signature=adhoc" means this is NOT the signed release artifact

codesign --verify --deep --strict --verbose=2 /Applications/VibeCoder.app
```

Signing and notarization are separate. A signed but un-notarized app still
shows the "unidentified developer" prompt on first launch — right-click →
**Open** once, or `xattr -dr com.apple.quarantine /Applications/VibeCoder.app`.
To check which you have:

```bash
xcrun stapler validate /Applications/VibeCoder.app    # "worked!" = notarized
spctl -a -vvv -t install /Applications/VibeCoder.app  # "source=Notarized Developer ID"
```

Full details, including the maintainer credential list, in
[docs/release.md](docs/release.md#code-signing).

---

## Upgrade Guide

### From v0.5.10

No breaking API changes. Drop-in replace the binary and restart:

```bash
curl -fsSL https://raw.githubusercontent.com/TuringWorks/vibecody/main/install.sh | sh
```

Four things to know:

1. **Restart the daemon.** The desktop shells autostart it, but an already
   running daemon from a previous install keeps serving the old route set — and
   this release adds `GET`/`PUT /voice/settings` and fixes the CORS method list,
   neither of which reaches you until the process actually restarts.
2. **Speech needs an install step, and it always did.** `make voice-sidecar`
   builds the streaming engine; without it the daemon falls back to batch
   synthesis in the system default voice. `make voice-kokoro` adds the neural
   engine (Apple Silicon). `make voice-status` tells you which one will run —
   ask it before concluding the voice sounds wrong.
3. **Full-duplex voice is off until you turn it on.** It holds the microphone
   open for the whole session, so it is opt-in per shell; push-to-talk is
   unchanged and needs nothing.
4. **iOS builds now floor at 15.0** (from 13.0), a consequence of the Flutter
   3.47.1 pin. Devices below iOS 15 cannot run the 0.5.11 mobile app.

### From v0.5.9 or earlier

See the [v0.5.10 release notes](https://github.com/TuringWorks/vibecody/releases/tag/v0.5.10)
for the intermediate delta — every entry there applies, including the workspace
security panels reading the toolbar's provider and model rather than defaulting
to one you did not choose, and (from v0.5.9) the daemon restart the plugin and
connector routes need.

---

## Full Changelog

See [docs/CHANGELOG.md](docs/CHANGELOG.md) for the complete history.
See [compare view](../../compare/v0.5.10...v0.5.11) for the v0.5.11 diff.
