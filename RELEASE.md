# VibeCody v0.5.10 Release

**32 commits since v0.5.9.**

---

## What's in v0.5.10

Four security panels that review the workspace you actually have open, findings
that are re-checked before they reach you, one "Fix with AI" hand-off wherever
something is reported, and Settings that can turn features off and move them
around.

Underneath it, the theme of the last release again: a feature that renders is
not a feature that works. Generated code was written to `Component4.tsx`
instead of the file the model named. Build ran the first build system detected
rather than the one you picked. A model's private reasoning was committed as a
git subject line — twice, in this repository's own history. Each is now covered
by a test that fails when it comes back.

### Headline features

- **Workspace security review, from four panels.** Scanner, Red Team, Blue Team
  and Purple Team each review the open project with the provider and model
  selected in the toolbar, and each asks its own question: red finds the
  exploit, blue names the missing or weak control and what would detect an
  attack, purple reports the residual risk — the attack that gets through what
  is already there. Content is in scope alongside code, so prompts, docs and
  templates are checked for missing guardrails. The three prompts are pinned
  distinct by test: a refactor that collapsed them would quietly make "Blue
  Team" hunt exploits.

- **Findings are verified before they are reported.** The scanner re-checks each
  finding against the file it names and drops the ones it cannot substantiate,
  rather than forwarding the model's first answer.

- **One "Fix with AI" hand-off**, on every panel that reports something to fix.
  The finding, the file and the requested change are written into chat. Nothing
  is edited behind your back.

- **Settings turns features off, and reorders them.** Hide panels and tabs you
  do not use, reorder both, and host a tab in a panel other than the one it
  ships in.

- **vLLM and LM Studio**, on the shared OpenAI-compatible implementation that
  eight existing providers were folded onto — a fix to one is now a fix to all
  of them.

### Notable fixes

- **Generated code went to the wrong file.** The writer used a placeholder name
  (`Component4.tsx`) instead of the path the model had just named.

- **Build ignored the build system you selected**, running whichever one
  detection happened to order first — cargo, in a Rust project with a Makefile.

- **A model's reasoning reached user-visible output**: the chat window, review
  comments, and the subject line of AI-generated commit messages. Every spelling
  we have seen is now stripped — `<think>`, `<thinking>`, namespaced forms like
  `<mm:think>`, and blocks left unclosed or orphaned by the provider — and an
  empty result is reported rather than committed.

- **A routing verdict outlived its question.** "No configured model has a
  context window large enough for 1,010,000 tokens" stayed on screen after the
  slider moved, and followed the reader onto tabs where it described nothing.

- **The MCP client matched responses by order, not by id**, so a server that
  spoke before it was spoken to was read as an empty reply — which surfaced as
  a connector with no tools.

- **Panels that ran something showed no sign of it.** The test runner collected
  its output with a blocking call and emitted every line after the process
  exited; it now streams line by line with elapsed time and a line count, and
  Suspend kills the process group instead of only flipping a flag in the UI.

- **Documentation links that 404'd on the published site** — the installer URL,
  a wrong org in repository links, dead release assets, `.md` links that
  resolved to the domain root, and a plugin registry that was advertised but
  does not exist.

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
| Docker | `vibecli-docker-v0.5.10.tar.gz` |

### VibeCoder — Desktop Code Editor

| Platform | File |
|----------|------|
| macOS (Apple Silicon) | `VibeCoder_0.5.10_aarch64.dmg` |
| macOS (Intel) | `VibeCoder_0.5.10_x64.dmg` |
| macOS (`.app`) | `VibeCoder-macOS-{arm64,x64}.app.zip` |
| Linux x64 / arm64 (`.deb`) | `VibeCoder_0.5.10_{amd64,arm64}.deb` |
| Linux x64 / arm64 (`.AppImage`) | `VibeCoder_0.5.10_{amd64,aarch64}.AppImage` |
| Windows x64 | `VibeCoder_0.5.10_x64_en-US.msi` · `VibeCoder_0.5.10_x64-setup.exe` |

### VibeAIChat — Desktop AI Assistant

| Platform | File |
|----------|------|
| macOS (Apple Silicon) | `VibeAIChat_0.5.10_aarch64.dmg` |
| macOS (Intel) | `VibeAIChat_0.5.10_x64.dmg` |
| Linux x64 / arm64 (`.deb`) | `VibeAIChat_0.5.10_{amd64,arm64}.deb` |
| Linux x64 / arm64 (`.AppImage`) | `VibeAIChat_0.5.10_{amd64,aarch64}.AppImage` |
| Windows x64 | `VibeAIChat_0.5.10_x64_en-US.msi` · `VibeAIChat_0.5.10_x64-setup.exe` |

### VibeDesk — Desktop Task Shell

| Platform | File |
|----------|------|
| macOS (Apple Silicon) | `VibeDesk_0.5.10_aarch64.dmg` |
| macOS (Intel) | `VibeDesk_0.5.10_x64.dmg` |
| Linux x64 / arm64 (`.deb`) | `VibeDesk_0.5.10_{amd64,arm64}.deb` |
| Linux x64 / arm64 (`.AppImage`) | `VibeDesk_0.5.10_{amd64,aarch64}.AppImage` |
| Windows x64 | `VibeDesk_0.5.10_x64_en-US.msi` · `VibeDesk_0.5.10_x64-setup.exe` |

### VibeCody Mobile

| Platform | File |
|----------|------|
| iOS (unsigned `.ipa`) | `VibeCody-Mobile-v0.5.10-ios.ipa` |
| Android | `VibeCody-Mobile-v0.5.10-android.apk` · `.aab` |

### VibeCody Watch

| Platform | File |
|----------|------|
| watchOS 10+ (unsigned `.app.zip`) | `VibeCody-WatchOS-v0.5.10.app.zip` |
| Wear OS 3+ | `VibeCody-Wear-v0.5.10.apk` · `.aab` |

---

## Quick Install

```bash
# One-liner (Linux/macOS)
curl -fsSL https://raw.githubusercontent.com/TuringWorks/vibecody/main/install.sh | sh

# Docker (air-gapped / on-prem)
docker load < vibecli-docker-v0.5.10.tar.gz
docker run -p 7878:7878 vibecli:v0.5.10

# Verify
vibecli --version   # Should print: vibecli 0.5.10
```

---

## macOS code signing

Developer ID signing has covered the whole macOS surface since v0.5.8, the
`vibecli` binary included; before that it shipped with **no signature at all**
and the three app bundles were ad-hoc. Nothing about signing changed in v0.5.10.

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

### From v0.5.9

No breaking API changes. Drop-in replace the binary and restart:

```bash
curl -fsSL https://raw.githubusercontent.com/TuringWorks/vibecody/main/install.sh | sh
```

Two things to know:

1. **Restart the daemon.** The desktop shells autostart it, but an already
   running daemon from a previous install keeps serving the old route set.
2. **The security panels use the toolbar's provider and model.** With none
   selected they show a "select a model" empty state rather than falling back to
   a provider you did not choose — pick one before running a workspace review.

### From v0.5.8 or earlier

See the [v0.5.9 release notes](https://github.com/TuringWorks/vibecody/releases/tag/v0.5.9)
for the intermediate delta — every entry there applies, including the daemon
restart the plugin and connector routes need, and the per-workspace connector
definitions with encrypted credentials.

---

## Full Changelog

See [docs/CHANGELOG.md](docs/CHANGELOG.md) for the complete history.
See [compare view](../../compare/v0.5.9...v0.5.10) for the v0.5.10 diff.
