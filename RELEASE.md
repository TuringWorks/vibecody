# VibeCody v0.5.8 Release

**The largest release so far — 410 commits since v0.5.7.**

---

## What's in v0.5.8

Voice input on every client, SkillForge, the kodegraph code-graph substrate,
goal-driven loops, a provider-agnostic embedding layer, and the
VibeApp → VibeAIChat rename that brings **VibeDesk** in as a third desktop
shell. Developer ID code signing is wired end to end for macOS for the first
time — including the `vibecli` binary, which had no signature at all.

### Headline features

- **Voice input everywhere.** One daemon route (`POST /voice/transcribe`,
  `GET /voice/status`) and one shared React hook behind mic buttons in
  VibeCoder, VibeAIChat, VibeDesk, VibeMobile, and the VS Code / JetBrains /
  Neovim plugins. Groq Whisper with a local whisper.cpp fallback, so dictation
  works offline. Previously the whole voice stack was reachable only from the
  REPL and the daemon had no voice route at all.

- **Pick your embedding model.** Semantic search, `@codebase:` and memory
  recall run on Ollama, OpenAI (or any OpenAI-compatible endpoint — Azure,
  LiteLLM, vLLM, TEI), Voyage, Cohere, Gemini, or an in-process local model.
  Indexes are kept **per model** and coexist on disk, so trying a different
  model is instant and switching back never re-embeds. Models not in the
  shipped catalog work too — `ollama pull` anything. See
  [docs/embeddings.md](docs/embeddings.md).

- **SkillForge.** Analyse and train agent-skill documents (SkillLens +
  SkillOpt) from a VibeCoder panel, the REPL, the TUI, and ten daemon routes,
  with per-epoch SSE streaming and true cancellation.

- **Code graph (kodegraph).** A tree-sitter → SQLite knowledge graph built in
  the background on daemon start, feeding a compact god-node / community
  summary into the agent prompt in place of a flat directory tree. Eight
  `/v1/graph/*` routes, fanned out to seven clients.

- **Goal-driven loops.** `/loop goal <id>` runs until a goal's
  `success_criteria` verifiably hold — judged by a separate validator turn, not
  by the worker's own opinion. Only confirmed success writes back; an exhausted
  budget is not evidence of completion.

- **VibeDesk ships for the first time**, alongside VibeCoder and VibeAIChat.

### Notable fixes

- **The code index persisted API keys in plaintext.** The provider struct was
  serialized whole into `.vibecli/index.json`, key included. Index headers now
  store a model reference only; pre-existing indexes migrate and the credential
  is dropped.
- **Changing embedding model silently returned nonsense.** No dimension or
  format version was recorded, TurboQuant's dimension error was discarded at
  both call sites, and `vibe-memory` had no model column — so a dimension
  change made every existing memory unreachable while the rows sat in the
  database. All three now carry and check the model identity.
- **VibeAIChat never started the daemon.** It had no autostart path at all, so
  launching it on its own produced a blanket 401. It now uses the same shared
  `daemon_bootstrap` as VibeCoder and VibeDesk.
- **Stale bearer tokens caused a permanent 401 loop.** The token rotates on
  every daemon start; VibeDesk retried on none of its 20 daemon calls, and the
  shared voice module on neither of its two. Both now retry once with a freshly
  read token.
- **VibeCoder chat rendered markdown as raw text** — tables arrived as walls of
  `| --- |`. It now renders through the same shared component as the other two
  shells.
- **Two design tokens were referenced but never defined** (`--accent`,
  `--border`, across 33 sites), so those button fills and borders never
  rendered; and `.panel-btn` declared no background, so 43 buttons fell through
  to native OS chrome on a dark theme.

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
| Docker | `vibecli-docker-v0.5.8.tar.gz` |

### VibeCoder — Desktop Code Editor

| Platform | File |
|----------|------|
| macOS (Apple Silicon) | `VibeCoder_0.5.8_aarch64.dmg` |
| macOS (Intel) | `VibeCoder_0.5.8_x64.dmg` |
| macOS (`.app`) | `VibeCoder-macOS-{arm64,x64}.app.zip` |
| Linux x64 / arm64 (`.deb`) | `VibeCoder_0.5.8_{amd64,arm64}.deb` |
| Linux x64 / arm64 (`.AppImage`) | `VibeCoder_0.5.8_{amd64,aarch64}.AppImage` |
| Windows x64 | `VibeCoder_0.5.8_x64_en-US.msi` · `VibeCoder_0.5.8_x64-setup.exe` |

### VibeAIChat — Desktop AI Assistant

| Platform | File |
|----------|------|
| macOS (Apple Silicon) | `VibeAIChat_0.5.8_aarch64.dmg` |
| macOS (Intel) | `VibeAIChat_0.5.8_x64.dmg` |
| Linux x64 / arm64 (`.deb`) | `VibeAIChat_0.5.8_{amd64,arm64}.deb` |
| Linux x64 / arm64 (`.AppImage`) | `VibeAIChat_0.5.8_{amd64,aarch64}.AppImage` |
| Windows x64 | `VibeAIChat_0.5.8_x64_en-US.msi` · `VibeAIChat_0.5.8_x64-setup.exe` |

### VibeDesk — Desktop Task Shell *(new)*

| Platform | File |
|----------|------|
| macOS (Apple Silicon) | `VibeDesk_0.5.8_aarch64.dmg` |
| macOS (Intel) | `VibeDesk_0.5.8_x64.dmg` |
| Linux x64 / arm64 (`.deb`) | `VibeDesk_0.5.8_{amd64,arm64}.deb` |
| Linux x64 / arm64 (`.AppImage`) | `VibeDesk_0.5.8_{amd64,aarch64}.AppImage` |
| Windows x64 | `VibeDesk_0.5.8_x64_en-US.msi` · `VibeDesk_0.5.8_x64-setup.exe` |

### VibeCody Mobile

| Platform | File |
|----------|------|
| iOS (unsigned `.ipa`) | `VibeCody-Mobile-v0.5.8-ios.ipa` |
| Android | `VibeCody-Mobile-v0.5.8-android.apk` · `.aab` |

### VibeCody Watch

| Platform | File |
|----------|------|
| watchOS 10+ (unsigned `.app.zip`) | `VibeCody-WatchOS-v0.5.8.app.zip` |
| Wear OS 3+ | `VibeCody-Wear-v0.5.8.apk` · `.aab` |

---

## Quick Install

```bash
# One-liner (Linux/macOS)
curl -fsSL https://raw.githubusercontent.com/TuringWorks/vibecody/main/install.sh | sh

# Docker (air-gapped / on-prem)
docker load < vibecli-docker-v0.5.8.tar.gz
docker run -p 7878:7878 vibecli:v0.5.8

# Verify
vibecli --version   # Should print: vibecli 0.5.8
```

---

## macOS code signing

v0.5.8 adds Developer ID signing across the whole macOS surface. The `vibecli`
binary previously shipped with **no signature at all**; the three app bundles
were ad-hoc.

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

### From v0.5.7

No breaking API changes. Drop-in replace the binary and restart:

```bash
curl -fsSL https://raw.githubusercontent.com/TuringWorks/vibecody/main/install.sh | sh
```

Three things to know:

1. **Existing semantic indexes migrate on first use.** A pre-existing
   `.vibecli/index.json` is moved to `.vibecli/index/index__<provider>__<model>.json`
   and the API key that the old format stored in plaintext is discarded. If you
   built an index against a cloud provider before v0.5.8, **rotate that key** —
   it was on disk in cleartext.
2. **Memory rows written before v0.5.8 carry no embedding-model tag.** They are
   treated as comparable when their vector length matches the active model, so
   existing memories keep working. Change the embedding model and they are
   excluded from search rather than silently mis-scored — the daemon logs how
   many were skipped.
3. **`[index] embedding_provider` is now enforced.** It was previously dead
   config read only by tests. An unrecognised provider name is now a startup
   error instead of a silent fallback to Ollama.

### From v0.5.6 or earlier

See the [v0.5.7 release notes](https://github.com/TuringWorks/vibecody/releases/tag/v0.5.7)
for the intermediate delta — every entry there applies.

---

## Full Changelog

See [docs/CHANGELOG.md](docs/CHANGELOG.md) for the complete history.
See [compare view](../../compare/v0.5.7...v0.5.8) for the v0.5.8 diff.
