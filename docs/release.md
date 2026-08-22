---
layout: page
title: "Releases"
permalink: /release/
---

Download VibeCody release packages below. All binaries are built via GitHub Actions with SHA-256 checksums.

**Quick install (Linux/macOS):**

```bash
curl -fsSL https://raw.githubusercontent.com/TuringWorks/vibecody/main/install.sh | sh
```

---

## v0.5.10 — Latest

**Released:** August 21, 2026 &middot; [Release notes](https://github.com/TuringWorks/vibecody/releases/tag/v0.5.10) &middot; [Changelog](https://github.com/TuringWorks/vibecody/compare/v0.5.9...v0.5.10)

32 commits since v0.5.9. A security review that covers the workspace you have
open rather than a snippet, from four panels that each ask a different question;
one "Fix with AI" hand-off wherever something is reported; Settings that can
turn features off; and a run of fixes to features that looked finished and were
not.

### Highlights

- **Workspace security review, from four panels.** Scanner, Red Team, Blue Team and Purple Team each review the project you have open with the model selected in the toolbar. Red finds the exploit, blue names the missing control and what would detect an attack, purple reports the attack that gets through what is already there. Prompts, docs and templates are reviewed alongside code.
- **Findings are verified before they are shown.** The scanner re-checks each finding against the file it names and drops the ones that do not stand up, rather than forwarding whatever the model said.
- **One "Fix with AI" hand-off**, on every panel that reports something to fix: the finding, its file and the requested change go into chat instead of being retyped.
- **Settings can turn features off, and reorder them** — hide panels and tabs, reorder both, and host a tab in a panel other than the one it ships in.
- **vLLM and LM Studio** join the provider list, and eight existing OpenAI-compatible providers moved onto one shared implementation.
- **The last plaintext credentials moved into the encrypted stores**, and `WorkspaceStore`'s `Debug` no longer prints its key.

### Notable fixes

- Generated code was written to `Component4.tsx` instead of the file the model named.
- Build ran the first build system detected, not the one selected in the dropdown.
- Reasoning tags — `<thinking>`, `<think>`, namespaced spellings like `<mm:think>` — reached the chat window, review comments, and the subject line of generated commit messages. Two commits in this repo's own history carry one.
- The long-context router's verdict outlived the token count it was about: move the slider and the banner still described the old number.
- The MCP client matched responses by order, so a server that spoke first was read as an empty reply — a connector with no tools.
- Documentation links that 404'd on the published site: the installer URL, a wrong org, dead release assets, and `.md` links that resolved to the domain root.

macOS signing is unchanged from v0.5.8; see [Code signing](#code-signing) to
check what you downloaded.

### VibeCLI — Terminal AI Assistant

| Platform | Download |
|----------|----------|
| macOS (Apple Silicon) | [`vibecli-aarch64-apple-darwin.tar.gz`](https://github.com/TuringWorks/vibecody/releases/download/v0.5.10/vibecli-aarch64-apple-darwin.tar.gz) |
| Linux (arm64) | [`vibecli-aarch64-linux.tar.gz`](https://github.com/TuringWorks/vibecody/releases/download/v0.5.10/vibecli-aarch64-linux.tar.gz) |
| Docker image (tarball) | [`vibecli-docker-v0.5.10.tar.gz`](https://github.com/TuringWorks/vibecody/releases/download/v0.5.10/vibecli-docker-v0.5.10.tar.gz) |
| macOS (Intel) | [`vibecli-x86_64-apple-darwin.tar.gz`](https://github.com/TuringWorks/vibecody/releases/download/v0.5.10/vibecli-x86_64-apple-darwin.tar.gz) |
| Linux (x86_64) | [`vibecli-x86_64-linux.tar.gz`](https://github.com/TuringWorks/vibecody/releases/download/v0.5.10/vibecli-x86_64-linux.tar.gz) |
| Windows (x86_64) | [`vibecli-x86_64-windows.zip`](https://github.com/TuringWorks/vibecody/releases/download/v0.5.10/vibecli-x86_64-windows.zip) |

### VibeCoder — Desktop Code Editor

| Platform | Download |
|----------|----------|
| macOS (Apple Silicon, .app) | [`VibeCoder-macOS-arm64.app.zip`](https://github.com/TuringWorks/vibecody/releases/download/v0.5.10/VibeCoder-macOS-arm64.app.zip) |
| macOS (Intel, .app) | [`VibeCoder-macOS-x64.app.zip`](https://github.com/TuringWorks/vibecody/releases/download/v0.5.10/VibeCoder-macOS-x64.app.zip) |
| Linux (arm64, AppImage) | [`VibeCoder_0.5.10_aarch64.AppImage`](https://github.com/TuringWorks/vibecody/releases/download/v0.5.10/VibeCoder_0.5.10_aarch64.AppImage) |
| macOS (Apple Silicon) | [`VibeCoder_0.5.10_aarch64.dmg`](https://github.com/TuringWorks/vibecody/releases/download/v0.5.10/VibeCoder_0.5.10_aarch64.dmg) |
| Linux (x86_64, AppImage) | [`VibeCoder_0.5.10_amd64.AppImage`](https://github.com/TuringWorks/vibecody/releases/download/v0.5.10/VibeCoder_0.5.10_amd64.AppImage) |
| Linux (x86_64, deb) | [`VibeCoder_0.5.10_amd64.deb`](https://github.com/TuringWorks/vibecody/releases/download/v0.5.10/VibeCoder_0.5.10_amd64.deb) |
| Linux (arm64, deb) | [`VibeCoder_0.5.10_arm64.deb`](https://github.com/TuringWorks/vibecody/releases/download/v0.5.10/VibeCoder_0.5.10_arm64.deb) |
| Windows (installer) | [`VibeCoder_0.5.10_x64-setup.exe`](https://github.com/TuringWorks/vibecody/releases/download/v0.5.10/VibeCoder_0.5.10_x64-setup.exe) |
| macOS (Intel) | [`VibeCoder_0.5.10_x64.dmg`](https://github.com/TuringWorks/vibecody/releases/download/v0.5.10/VibeCoder_0.5.10_x64.dmg) |
| Windows (MSI) | [`VibeCoder_0.5.10_x64_en-US.msi`](https://github.com/TuringWorks/vibecody/releases/download/v0.5.10/VibeCoder_0.5.10_x64_en-US.msi) |

### VibeAIChat — Desktop AI Assistant

| Platform | Download |
|----------|----------|
| Linux (arm64, AppImage) | [`VibeAIChat_0.5.10_aarch64.AppImage`](https://github.com/TuringWorks/vibecody/releases/download/v0.5.10/VibeAIChat_0.5.10_aarch64.AppImage) |
| macOS (Apple Silicon) | [`VibeAIChat_0.5.10_aarch64.dmg`](https://github.com/TuringWorks/vibecody/releases/download/v0.5.10/VibeAIChat_0.5.10_aarch64.dmg) |
| Linux (x86_64, AppImage) | [`VibeAIChat_0.5.10_amd64.AppImage`](https://github.com/TuringWorks/vibecody/releases/download/v0.5.10/VibeAIChat_0.5.10_amd64.AppImage) |
| Linux (x86_64, deb) | [`VibeAIChat_0.5.10_amd64.deb`](https://github.com/TuringWorks/vibecody/releases/download/v0.5.10/VibeAIChat_0.5.10_amd64.deb) |
| Linux (arm64, deb) | [`VibeAIChat_0.5.10_arm64.deb`](https://github.com/TuringWorks/vibecody/releases/download/v0.5.10/VibeAIChat_0.5.10_arm64.deb) |
| Windows (installer) | [`VibeAIChat_0.5.10_x64-setup.exe`](https://github.com/TuringWorks/vibecody/releases/download/v0.5.10/VibeAIChat_0.5.10_x64-setup.exe) |
| macOS (Intel) | [`VibeAIChat_0.5.10_x64.dmg`](https://github.com/TuringWorks/vibecody/releases/download/v0.5.10/VibeAIChat_0.5.10_x64.dmg) |
| Windows (MSI) | [`VibeAIChat_0.5.10_x64_en-US.msi`](https://github.com/TuringWorks/vibecody/releases/download/v0.5.10/VibeAIChat_0.5.10_x64_en-US.msi) |

### VibeDesk — Desktop Task Shell

| Platform | Download |
|----------|----------|
| Linux (arm64, AppImage) | [`VibeDesk_0.5.10_aarch64.AppImage`](https://github.com/TuringWorks/vibecody/releases/download/v0.5.10/VibeDesk_0.5.10_aarch64.AppImage) |
| macOS (Apple Silicon) | [`VibeDesk_0.5.10_aarch64.dmg`](https://github.com/TuringWorks/vibecody/releases/download/v0.5.10/VibeDesk_0.5.10_aarch64.dmg) |
| Linux (x86_64, AppImage) | [`VibeDesk_0.5.10_amd64.AppImage`](https://github.com/TuringWorks/vibecody/releases/download/v0.5.10/VibeDesk_0.5.10_amd64.AppImage) |
| Linux (x86_64, deb) | [`VibeDesk_0.5.10_amd64.deb`](https://github.com/TuringWorks/vibecody/releases/download/v0.5.10/VibeDesk_0.5.10_amd64.deb) |
| Linux (arm64, deb) | [`VibeDesk_0.5.10_arm64.deb`](https://github.com/TuringWorks/vibecody/releases/download/v0.5.10/VibeDesk_0.5.10_arm64.deb) |
| Windows (installer) | [`VibeDesk_0.5.10_x64-setup.exe`](https://github.com/TuringWorks/vibecody/releases/download/v0.5.10/VibeDesk_0.5.10_x64-setup.exe) |
| macOS (Intel) | [`VibeDesk_0.5.10_x64.dmg`](https://github.com/TuringWorks/vibecody/releases/download/v0.5.10/VibeDesk_0.5.10_x64.dmg) |
| Windows (MSI) | [`VibeDesk_0.5.10_x64_en-US.msi`](https://github.com/TuringWorks/vibecody/releases/download/v0.5.10/VibeDesk_0.5.10_x64_en-US.msi) |

### VibeMobile — Flutter Companion

| Platform | Download |
|----------|----------|
| Android (AAB) | [`VibeCody-Mobile-v0.5.10-android.aab`](https://github.com/TuringWorks/vibecody/releases/download/v0.5.10/VibeCody-Mobile-v0.5.10-android.aab) |
| Android (APK) | [`VibeCody-Mobile-v0.5.10-android.apk`](https://github.com/TuringWorks/vibecody/releases/download/v0.5.10/VibeCody-Mobile-v0.5.10-android.apk) |
| iOS (unsigned — sideload via AltStore / Sideloadly) | [`VibeCody-Mobile-v0.5.10-ios.ipa`](https://github.com/TuringWorks/vibecody/releases/download/v0.5.10/VibeCody-Mobile-v0.5.10-ios.ipa) |

### VibeWatch — Apple Watch & Wear OS

| Platform | Download |
|----------|----------|
| watchOS 10+ (unsigned — sideload via Xcode) | [`VibeCody-WatchOS-v0.5.10.app.zip`](https://github.com/TuringWorks/vibecody/releases/download/v0.5.10/VibeCody-WatchOS-v0.5.10.app.zip) |
| Wear OS 3+ (AAB) | [`VibeCody-Wear-v0.5.10.aab`](https://github.com/TuringWorks/vibecody/releases/download/v0.5.10/VibeCody-Wear-v0.5.10.aab) |
| Wear OS 3+ (APK) | [`VibeCody-Wear-v0.5.10.apk`](https://github.com/TuringWorks/vibecody/releases/download/v0.5.10/VibeCody-Wear-v0.5.10.apk) |

[SHA256SUMS.txt](https://github.com/TuringWorks/vibecody/releases/download/v0.5.10/SHA256SUMS.txt)

---

## v0.5.9

**Released:** August 14, 2026 &middot; [Release notes](https://github.com/TuringWorks/vibecody/releases/tag/v0.5.9) &middot; [Changelog](https://github.com/TuringWorks/vibecody/compare/v0.5.8...v0.5.9)

64 commits since v0.5.8. An evaluation harness, a plugin marketplace that can
actually install something, connectors as first-class MCP integrations, and a
run of agent-reliability work that came out of watching real runs stall and
then claim success.

### Highlights

- **`vibecli eval`** — an evaluation harness covering coding, agentic tool use, knowledge work, safety, and per-surface transport conformance across all fourteen clients. Four verdicts kept apart: `pass`, `fail`, `error` (the harness could not decide) and `skipped` (did not apply), with the last two outside the pass-rate denominator. `make eval-check` validates the suites with no provider and no agent.
- **A plugin marketplace, and connectors.** The Plugins panel used to name a CLI command; it now searches, categorises and installs. Eleven core plugins and seventeen connectors ship in the binary, with connector credentials encrypted in the workspace store. **Bundles** — Engineering, On-call, Security review, Data work, Research — install a set of plugins and set up the connectors that job assumes.
- **Runs are bounded from outside their own loops.** Every previous guard was checked between turns or between chunks, so each depended on some inner loop coming back round. Elapsed-time walls now cover "nothing has changed on disk" and "no tool has run at all", and a finished-but-silent agent is concluded rather than left to burn its budget.
- **The agent stopped claiming work it had not done.** `--exec` double-checks with the project's own build and test before accepting completion — and a check that fails to spawn is reported as unverified rather than counted as a pass.
- **Secrets no longer leave in the agent's own words.** Credential files are redacted on the way *in*, so the model never receives the value; redacting only its output could not survive paraphrase.
- **`/goal <what you want>`** in the CLI, and a VibeCoder panel that shows what a goal is doing and keeps failures on screen.

### Notable fixes

- 27 Tauri commands VibeCoder panels were already calling did not exist — every one of those clicks was a guaranteed rejection, and the panels looked finished.
- Ghost text, ⌘., Counsel, Arena, SuperBrain, Compare, Automations and Code Transforms each ignored the model you selected in the toolbar.
- Code Transforms scanned three different roots, one of which was the app's own working directory, and reported "0 files to transform".
- VibeCoder's Connectors panel reported every connector as connected without a credential and lost them all on restart.
- An autonomous run can no longer remove an authorization guard to make a test pass.
- The welcome screen's heading was sliced off the top on a short window.

Each of the first four is now covered by a test that fails when it comes back —
see [why](https://github.com/TuringWorks/vibecody/blob/main/AGENTS.md).

macOS signing is unchanged from v0.5.8; see [Code signing](#code-signing) to
check what you downloaded.


### VibeCLI — Terminal AI Assistant

| Platform | Download |
|----------|----------|
| macOS (Apple Silicon) | [`vibecli-aarch64-apple-darwin.tar.gz`](https://github.com/TuringWorks/vibecody/releases/download/v0.5.9/vibecli-aarch64-apple-darwin.tar.gz) |
| Linux (arm64) | [`vibecli-aarch64-linux.tar.gz`](https://github.com/TuringWorks/vibecody/releases/download/v0.5.9/vibecli-aarch64-linux.tar.gz) |
| Docker image (tarball) | [`vibecli-docker-v0.5.9.tar.gz`](https://github.com/TuringWorks/vibecody/releases/download/v0.5.9/vibecli-docker-v0.5.9.tar.gz) |
| macOS (Intel) | [`vibecli-x86_64-apple-darwin.tar.gz`](https://github.com/TuringWorks/vibecody/releases/download/v0.5.9/vibecli-x86_64-apple-darwin.tar.gz) |
| Linux (x86_64) | [`vibecli-x86_64-linux.tar.gz`](https://github.com/TuringWorks/vibecody/releases/download/v0.5.9/vibecli-x86_64-linux.tar.gz) |
| Windows (x86_64) | [`vibecli-x86_64-windows.zip`](https://github.com/TuringWorks/vibecody/releases/download/v0.5.9/vibecli-x86_64-windows.zip) |

### VibeCoder — Desktop Code Editor

| Platform | Download |
|----------|----------|
| macOS (Apple Silicon, .app) | [`VibeCoder-macOS-arm64.app.zip`](https://github.com/TuringWorks/vibecody/releases/download/v0.5.9/VibeCoder-macOS-arm64.app.zip) |
| macOS (Intel, .app) | [`VibeCoder-macOS-x64.app.zip`](https://github.com/TuringWorks/vibecody/releases/download/v0.5.9/VibeCoder-macOS-x64.app.zip) |
| Linux (arm64, AppImage) | [`VibeCoder_0.5.9_aarch64.AppImage`](https://github.com/TuringWorks/vibecody/releases/download/v0.5.9/VibeCoder_0.5.9_aarch64.AppImage) |
| macOS (Apple Silicon) | [`VibeCoder_0.5.9_aarch64.dmg`](https://github.com/TuringWorks/vibecody/releases/download/v0.5.9/VibeCoder_0.5.9_aarch64.dmg) |
| Linux (x86_64, AppImage) | [`VibeCoder_0.5.9_amd64.AppImage`](https://github.com/TuringWorks/vibecody/releases/download/v0.5.9/VibeCoder_0.5.9_amd64.AppImage) |
| Linux (x86_64, deb) | [`VibeCoder_0.5.9_amd64.deb`](https://github.com/TuringWorks/vibecody/releases/download/v0.5.9/VibeCoder_0.5.9_amd64.deb) |
| Linux (arm64, deb) | [`VibeCoder_0.5.9_arm64.deb`](https://github.com/TuringWorks/vibecody/releases/download/v0.5.9/VibeCoder_0.5.9_arm64.deb) |
| Windows (installer) | [`VibeCoder_0.5.9_x64-setup.exe`](https://github.com/TuringWorks/vibecody/releases/download/v0.5.9/VibeCoder_0.5.9_x64-setup.exe) |
| macOS (Intel) | [`VibeCoder_0.5.9_x64.dmg`](https://github.com/TuringWorks/vibecody/releases/download/v0.5.9/VibeCoder_0.5.9_x64.dmg) |
| Windows (MSI) | [`VibeCoder_0.5.9_x64_en-US.msi`](https://github.com/TuringWorks/vibecody/releases/download/v0.5.9/VibeCoder_0.5.9_x64_en-US.msi) |

### VibeAIChat — Desktop AI Assistant

| Platform | Download |
|----------|----------|
| Linux (arm64, AppImage) | [`VibeAIChat_0.5.9_aarch64.AppImage`](https://github.com/TuringWorks/vibecody/releases/download/v0.5.9/VibeAIChat_0.5.9_aarch64.AppImage) |
| macOS (Apple Silicon) | [`VibeAIChat_0.5.9_aarch64.dmg`](https://github.com/TuringWorks/vibecody/releases/download/v0.5.9/VibeAIChat_0.5.9_aarch64.dmg) |
| Linux (x86_64, AppImage) | [`VibeAIChat_0.5.9_amd64.AppImage`](https://github.com/TuringWorks/vibecody/releases/download/v0.5.9/VibeAIChat_0.5.9_amd64.AppImage) |
| Linux (x86_64, deb) | [`VibeAIChat_0.5.9_amd64.deb`](https://github.com/TuringWorks/vibecody/releases/download/v0.5.9/VibeAIChat_0.5.9_amd64.deb) |
| Linux (arm64, deb) | [`VibeAIChat_0.5.9_arm64.deb`](https://github.com/TuringWorks/vibecody/releases/download/v0.5.9/VibeAIChat_0.5.9_arm64.deb) |
| Windows (installer) | [`VibeAIChat_0.5.9_x64-setup.exe`](https://github.com/TuringWorks/vibecody/releases/download/v0.5.9/VibeAIChat_0.5.9_x64-setup.exe) |
| macOS (Intel) | [`VibeAIChat_0.5.9_x64.dmg`](https://github.com/TuringWorks/vibecody/releases/download/v0.5.9/VibeAIChat_0.5.9_x64.dmg) |
| Windows (MSI) | [`VibeAIChat_0.5.9_x64_en-US.msi`](https://github.com/TuringWorks/vibecody/releases/download/v0.5.9/VibeAIChat_0.5.9_x64_en-US.msi) |

### VibeDesk — Desktop Task Shell

| Platform | Download |
|----------|----------|
| Linux (arm64, AppImage) | [`VibeDesk_0.5.9_aarch64.AppImage`](https://github.com/TuringWorks/vibecody/releases/download/v0.5.9/VibeDesk_0.5.9_aarch64.AppImage) |
| macOS (Apple Silicon) | [`VibeDesk_0.5.9_aarch64.dmg`](https://github.com/TuringWorks/vibecody/releases/download/v0.5.9/VibeDesk_0.5.9_aarch64.dmg) |
| Linux (x86_64, AppImage) | [`VibeDesk_0.5.9_amd64.AppImage`](https://github.com/TuringWorks/vibecody/releases/download/v0.5.9/VibeDesk_0.5.9_amd64.AppImage) |
| Linux (x86_64, deb) | [`VibeDesk_0.5.9_amd64.deb`](https://github.com/TuringWorks/vibecody/releases/download/v0.5.9/VibeDesk_0.5.9_amd64.deb) |
| Linux (arm64, deb) | [`VibeDesk_0.5.9_arm64.deb`](https://github.com/TuringWorks/vibecody/releases/download/v0.5.9/VibeDesk_0.5.9_arm64.deb) |
| Windows (installer) | [`VibeDesk_0.5.9_x64-setup.exe`](https://github.com/TuringWorks/vibecody/releases/download/v0.5.9/VibeDesk_0.5.9_x64-setup.exe) |
| macOS (Intel) | [`VibeDesk_0.5.9_x64.dmg`](https://github.com/TuringWorks/vibecody/releases/download/v0.5.9/VibeDesk_0.5.9_x64.dmg) |
| Windows (MSI) | [`VibeDesk_0.5.9_x64_en-US.msi`](https://github.com/TuringWorks/vibecody/releases/download/v0.5.9/VibeDesk_0.5.9_x64_en-US.msi) |

### VibeMobile — Flutter Companion

| Platform | Download |
|----------|----------|
| Android (AAB) | [`VibeCody-Mobile-v0.5.9-android.aab`](https://github.com/TuringWorks/vibecody/releases/download/v0.5.9/VibeCody-Mobile-v0.5.9-android.aab) |
| Android (APK) | [`VibeCody-Mobile-v0.5.9-android.apk`](https://github.com/TuringWorks/vibecody/releases/download/v0.5.9/VibeCody-Mobile-v0.5.9-android.apk) |
| iOS (unsigned — sideload via AltStore / Sideloadly) | [`VibeCody-Mobile-v0.5.9-ios.ipa`](https://github.com/TuringWorks/vibecody/releases/download/v0.5.9/VibeCody-Mobile-v0.5.9-ios.ipa) |

### VibeWatch — Apple Watch & Wear OS

| Platform | Download |
|----------|----------|
| watchOS 10+ (unsigned — sideload via Xcode) | [`VibeCody-WatchOS-v0.5.9.app.zip`](https://github.com/TuringWorks/vibecody/releases/download/v0.5.9/VibeCody-WatchOS-v0.5.9.app.zip) |
| Wear OS 3+ (AAB) | [`VibeCody-Wear-v0.5.9.aab`](https://github.com/TuringWorks/vibecody/releases/download/v0.5.9/VibeCody-Wear-v0.5.9.aab) |
| Wear OS 3+ (APK) | [`VibeCody-Wear-v0.5.9.apk`](https://github.com/TuringWorks/vibecody/releases/download/v0.5.9/VibeCody-Wear-v0.5.9.apk) |

[SHA256SUMS.txt](https://github.com/TuringWorks/vibecody/releases/download/v0.5.9/SHA256SUMS.txt)

---

## v0.5.8

**Released:** August 10, 2026 &middot; [Release notes](https://github.com/TuringWorks/vibecody/releases/tag/v0.5.8) &middot; [Changelog](https://github.com/TuringWorks/vibecody/compare/v0.5.7...v0.5.8)

The largest release so far — 410 commits since v0.5.7. Voice input on every
client, SkillForge, the kodegraph code-graph substrate, goal-driven loops, a
provider-agnostic embedding layer, and the VibeApp → VibeAIChat rename that
brings **VibeDesk** in as a third desktop shell.

### Highlights

- **Voice input everywhere** — one daemon route (`POST /voice/transcribe`) and one shared React hook behind mic buttons in all three desktop shells, VibeMobile, and the VS Code / JetBrains / Neovim plugins. Groq Whisper with a local whisper.cpp fallback, so it works offline.
- **Choose your embedding model** — semantic search and `@codebase:` now run on Ollama, OpenAI (or any OpenAI-compatible endpoint), Voyage, Cohere, Gemini, or an in-process local model. Indexes are kept per-model, so switching is instant and switching back never re-embeds. See [Embedding Models]({{ site.baseurl }}/embeddings/).
- **SkillForge** — analyse and train agent-skill documents (SkillLens + SkillOpt) from a VibeCoder panel, the REPL, the TUI, and ten daemon routes.
- **Code graph (kodegraph)** — a tree-sitter → SQLite knowledge graph built in the background on daemon start, feeding a compact repo summary into the agent prompt in place of a flat directory tree.
- **Goal-driven loops** — `/loop goal <id>` runs until a goal's `success_criteria` verifiably hold, judged by a separate validator turn rather than the worker's own opinion.
- **VibeDesk ships for the first time**, alongside VibeCoder and VibeAIChat.

### Notable fixes

- macOS artifacts can now be **Developer ID signed** end to end, `vibecli` included — see [Code signing](#code-signing) to check what you downloaded.
- The code index no longer writes API keys to disk; pre-existing indexes are migrated and the credential dropped.
- Switching embedding model used to return silently wrong results; indexes and memory rows now carry the model that produced them.
- VibeAIChat now autostarts the daemon like the other shells, and all three recover from a rotated bearer token instead of looping on 401.
- VibeCoder chat renders markdown — tables, headings, emphasis — instead of raw text.
- Theme tokens: two names referenced across 33 sites were never defined, so those borders and button fills never rendered.

### VibeCLI — Terminal AI Assistant

| Platform | Download |
|----------|----------|
| macOS (Apple Silicon) | [`vibecli-aarch64-apple-darwin.tar.gz`](https://github.com/TuringWorks/vibecody/releases/download/v0.5.8/vibecli-aarch64-apple-darwin.tar.gz) |
| Linux (arm64) | [`vibecli-aarch64-linux.tar.gz`](https://github.com/TuringWorks/vibecody/releases/download/v0.5.8/vibecli-aarch64-linux.tar.gz) |
| Docker image (tarball) | [`vibecli-docker-v0.5.8.tar.gz`](https://github.com/TuringWorks/vibecody/releases/download/v0.5.8/vibecli-docker-v0.5.8.tar.gz) |
| macOS (Intel) | [`vibecli-x86_64-apple-darwin.tar.gz`](https://github.com/TuringWorks/vibecody/releases/download/v0.5.8/vibecli-x86_64-apple-darwin.tar.gz) |
| Linux (x86_64) | [`vibecli-x86_64-linux.tar.gz`](https://github.com/TuringWorks/vibecody/releases/download/v0.5.8/vibecli-x86_64-linux.tar.gz) |
| Windows (x86_64) | [`vibecli-x86_64-windows.zip`](https://github.com/TuringWorks/vibecody/releases/download/v0.5.8/vibecli-x86_64-windows.zip) |

### VibeCoder — Desktop Code Editor

| Platform | Download |
|----------|----------|
| macOS (Apple Silicon, .app) | [`VibeCoder-macOS-arm64.app.zip`](https://github.com/TuringWorks/vibecody/releases/download/v0.5.8/VibeCoder-macOS-arm64.app.zip) |
| macOS (Intel, .app) | [`VibeCoder-macOS-x64.app.zip`](https://github.com/TuringWorks/vibecody/releases/download/v0.5.8/VibeCoder-macOS-x64.app.zip) |
| Linux (arm64, AppImage) | [`VibeCoder_0.5.8_aarch64.AppImage`](https://github.com/TuringWorks/vibecody/releases/download/v0.5.8/VibeCoder_0.5.8_aarch64.AppImage) |
| macOS (Apple Silicon) | [`VibeCoder_0.5.8_aarch64.dmg`](https://github.com/TuringWorks/vibecody/releases/download/v0.5.8/VibeCoder_0.5.8_aarch64.dmg) |
| Linux (x86_64, AppImage) | [`VibeCoder_0.5.8_amd64.AppImage`](https://github.com/TuringWorks/vibecody/releases/download/v0.5.8/VibeCoder_0.5.8_amd64.AppImage) |
| Linux (x86_64, deb) | [`VibeCoder_0.5.8_amd64.deb`](https://github.com/TuringWorks/vibecody/releases/download/v0.5.8/VibeCoder_0.5.8_amd64.deb) |
| Linux (arm64, deb) | [`VibeCoder_0.5.8_arm64.deb`](https://github.com/TuringWorks/vibecody/releases/download/v0.5.8/VibeCoder_0.5.8_arm64.deb) |
| Windows (installer) | [`VibeCoder_0.5.8_x64-setup.exe`](https://github.com/TuringWorks/vibecody/releases/download/v0.5.8/VibeCoder_0.5.8_x64-setup.exe) |
| macOS (Intel) | [`VibeCoder_0.5.8_x64.dmg`](https://github.com/TuringWorks/vibecody/releases/download/v0.5.8/VibeCoder_0.5.8_x64.dmg) |
| Windows (MSI) | [`VibeCoder_0.5.8_x64_en-US.msi`](https://github.com/TuringWorks/vibecody/releases/download/v0.5.8/VibeCoder_0.5.8_x64_en-US.msi) |

### VibeAIChat — Desktop AI Assistant

| Platform | Download |
|----------|----------|
| Linux (arm64, AppImage) | [`VibeAIChat_0.5.8_aarch64.AppImage`](https://github.com/TuringWorks/vibecody/releases/download/v0.5.8/VibeAIChat_0.5.8_aarch64.AppImage) |
| macOS (Apple Silicon) | [`VibeAIChat_0.5.8_aarch64.dmg`](https://github.com/TuringWorks/vibecody/releases/download/v0.5.8/VibeAIChat_0.5.8_aarch64.dmg) |
| Linux (x86_64, AppImage) | [`VibeAIChat_0.5.8_amd64.AppImage`](https://github.com/TuringWorks/vibecody/releases/download/v0.5.8/VibeAIChat_0.5.8_amd64.AppImage) |
| Linux (x86_64, deb) | [`VibeAIChat_0.5.8_amd64.deb`](https://github.com/TuringWorks/vibecody/releases/download/v0.5.8/VibeAIChat_0.5.8_amd64.deb) |
| Linux (arm64, deb) | [`VibeAIChat_0.5.8_arm64.deb`](https://github.com/TuringWorks/vibecody/releases/download/v0.5.8/VibeAIChat_0.5.8_arm64.deb) |
| Windows (installer) | [`VibeAIChat_0.5.8_x64-setup.exe`](https://github.com/TuringWorks/vibecody/releases/download/v0.5.8/VibeAIChat_0.5.8_x64-setup.exe) |
| macOS (Intel) | [`VibeAIChat_0.5.8_x64.dmg`](https://github.com/TuringWorks/vibecody/releases/download/v0.5.8/VibeAIChat_0.5.8_x64.dmg) |
| Windows (MSI) | [`VibeAIChat_0.5.8_x64_en-US.msi`](https://github.com/TuringWorks/vibecody/releases/download/v0.5.8/VibeAIChat_0.5.8_x64_en-US.msi) |

### VibeDesk — Desktop Task Shell

| Platform | Download |
|----------|----------|
| Linux (arm64, AppImage) | [`VibeDesk_0.5.8_aarch64.AppImage`](https://github.com/TuringWorks/vibecody/releases/download/v0.5.8/VibeDesk_0.5.8_aarch64.AppImage) |
| macOS (Apple Silicon) | [`VibeDesk_0.5.8_aarch64.dmg`](https://github.com/TuringWorks/vibecody/releases/download/v0.5.8/VibeDesk_0.5.8_aarch64.dmg) |
| Linux (x86_64, AppImage) | [`VibeDesk_0.5.8_amd64.AppImage`](https://github.com/TuringWorks/vibecody/releases/download/v0.5.8/VibeDesk_0.5.8_amd64.AppImage) |
| Linux (x86_64, deb) | [`VibeDesk_0.5.8_amd64.deb`](https://github.com/TuringWorks/vibecody/releases/download/v0.5.8/VibeDesk_0.5.8_amd64.deb) |
| Linux (arm64, deb) | [`VibeDesk_0.5.8_arm64.deb`](https://github.com/TuringWorks/vibecody/releases/download/v0.5.8/VibeDesk_0.5.8_arm64.deb) |
| Windows (installer) | [`VibeDesk_0.5.8_x64-setup.exe`](https://github.com/TuringWorks/vibecody/releases/download/v0.5.8/VibeDesk_0.5.8_x64-setup.exe) |
| macOS (Intel) | [`VibeDesk_0.5.8_x64.dmg`](https://github.com/TuringWorks/vibecody/releases/download/v0.5.8/VibeDesk_0.5.8_x64.dmg) |
| Windows (MSI) | [`VibeDesk_0.5.8_x64_en-US.msi`](https://github.com/TuringWorks/vibecody/releases/download/v0.5.8/VibeDesk_0.5.8_x64_en-US.msi) |

### VibeMobile — Flutter Companion

| Platform | Download |
|----------|----------|
| Android (AAB) | [`VibeCody-Mobile-v0.5.8-android.aab`](https://github.com/TuringWorks/vibecody/releases/download/v0.5.8/VibeCody-Mobile-v0.5.8-android.aab) |
| Android (APK) | [`VibeCody-Mobile-v0.5.8-android.apk`](https://github.com/TuringWorks/vibecody/releases/download/v0.5.8/VibeCody-Mobile-v0.5.8-android.apk) |
| iOS (unsigned — sideload via AltStore / Sideloadly) | [`VibeCody-Mobile-v0.5.8-ios.ipa`](https://github.com/TuringWorks/vibecody/releases/download/v0.5.8/VibeCody-Mobile-v0.5.8-ios.ipa) |

### VibeWatch — Apple Watch & Wear OS

| Platform | Download |
|----------|----------|
| watchOS 10+ (unsigned — sideload via Xcode) | [`VibeCody-WatchOS-v0.5.8.app.zip`](https://github.com/TuringWorks/vibecody/releases/download/v0.5.8/VibeCody-WatchOS-v0.5.8.app.zip) |
| Wear OS 3+ (AAB) | [`VibeCody-Wear-v0.5.8.aab`](https://github.com/TuringWorks/vibecody/releases/download/v0.5.8/VibeCody-Wear-v0.5.8.aab) |
| Wear OS 3+ (APK) | [`VibeCody-Wear-v0.5.8.apk`](https://github.com/TuringWorks/vibecody/releases/download/v0.5.8/VibeCody-Wear-v0.5.8.apk) |

[SHA256SUMS.txt](https://github.com/TuringWorks/vibecody/releases/download/v0.5.8/SHA256SUMS.txt)

### Code signing

v0.5.8 adds Developer ID signing to the whole macOS surface — including the
`vibecli` binary, which previously shipped with **no signature at all**.

Whether a specific download is signed depends on the build that produced it:
the release workflow signs only when the `APPLE_CERT_P12_BASE64` secret is
configured, and falls back to ad-hoc (apps) or unsigned (CLI) with a workflow
notice otherwise. Rather than take this page's word for it, check the artifact
you actually have:

```bash
codesign -dv --verbose=4 /Applications/VibeCoder.app
# expect: Authority=Developer ID Application: Ravindra Boddipalli (N7HV58M58W)
# a line reading "Signature=adhoc" means the artifact is NOT the signed release

codesign --verify --deep --strict --verbose=2 /Applications/VibeCoder.app
# expect: "valid on disk" and "satisfies its Designated Requirement"
```

The standalone `vibecli` binary in the macOS tarballs is signed with the same
identity:

```bash
codesign -dv --verbose=4 ./vibecli
```

**Notarization status.** Signing and notarization are separate steps. A signed
but un-notarized app still shows the "unidentified developer" prompt on first
launch — use right-click → **Open**, or strip the quarantine attribute as
described below. Check whether a given build was notarized:

```bash
xcrun stapler validate /Applications/VibeCoder.app   # "worked!" = notarized
spctl -a -vvv -t install /Applications/VibeCoder.app # "source=Notarized Developer ID"
```

Maintainers: see [macOS code signing setup](#macos-code-signing-setup-for-maintainers)
for the full credential list and the local build recipe.

---

## v0.5.7

**Released:** May 29, 2026 &middot; [Release notes](https://github.com/TuringWorks/vibecody/releases/tag/v0.5.7) &middot; [Changelog](https://github.com/TuringWorks/vibecody/compare/v0.5.6...v0.5.7)

Previous release. VibeDesk is not listed: it did not exist yet in v0.5.7.

> Binaries for this release are no longer published — the filenames below are kept as a record of what shipped. See the [release notes](https://github.com/TuringWorks/vibecody/releases/tag/v0.5.7) for details.

### VibeCLI — Terminal AI Assistant

| Platform | Download |
|----------|----------|
| macOS (Apple Silicon) | `vibecli-aarch64-apple-darwin.tar.gz` |
| Linux (arm64) | `vibecli-aarch64-linux.tar.gz` |
| Docker image (tarball) | `vibecli-docker-v0.5.7.tar.gz` |
| macOS (Intel) | `vibecli-x86_64-apple-darwin.tar.gz` |
| Linux (x86_64) | `vibecli-x86_64-linux.tar.gz` |
| Windows (x86_64) | `vibecli-x86_64-windows.zip` |

### VibeCoder — Desktop Code Editor

Shipped as **VibeUI** — the rename to VibeCoder landed after this release, so the filenames still say `VibeUI`.

| Platform | Download |
|----------|----------|
| macOS (Apple Silicon) | `VibeUI-macOS-arm64.app.zip` |
| macOS (Intel) | `VibeUI-macOS-x64.app.zip` |
| Linux (arm64, AppImage) | `VibeUI_0.5.7_aarch64.AppImage` |
| macOS (Apple Silicon) | `VibeUI_0.5.7_aarch64.dmg` |
| Linux (x86_64, AppImage) | `VibeUI_0.5.7_amd64.AppImage` |
| Linux (x86_64, deb) | `VibeUI_0.5.7_amd64.deb` |
| Linux (arm64, deb) | `VibeUI_0.5.7_arm64.deb` |
| Windows (installer) | `VibeUI_0.5.7_x64-setup.exe` |
| macOS (Intel) | `VibeUI_0.5.7_x64.dmg` |
| Windows (MSI) | `VibeUI_0.5.7_x64_en-US.msi` |

### VibeAIChat — Desktop AI Assistant

Shipped as **Vibe.App** — renamed to VibeAIChat after this release.

| Platform | Download |
|----------|----------|
| Linux (arm64, AppImage) | `Vibe.App_0.5.7_aarch64.AppImage` |
| macOS (Apple Silicon) | `Vibe.App_0.5.7_aarch64.dmg` |
| Linux (x86_64, AppImage) | `Vibe.App_0.5.7_amd64.AppImage` |
| Linux (x86_64, deb) | `Vibe.App_0.5.7_amd64.deb` |
| Linux (arm64, deb) | `Vibe.App_0.5.7_arm64.deb` |
| Windows (installer) | `Vibe.App_0.5.7_x64-setup.exe` |
| macOS (Intel) | `Vibe.App_0.5.7_x64.dmg` |
| Windows (MSI) | `Vibe.App_0.5.7_x64_en-US.msi` |

### VibeMobile — Flutter Companion

| Platform | Download |
|----------|----------|
| Android (AAB) | `VibeCody-Mobile-v0.5.7-android.aab` |
| Android (APK) | `VibeCody-Mobile-v0.5.7-android.apk` |
| iOS (unsigned — sideload via AltStore / Sideloadly) | `VibeCody-Mobile-v0.5.7-ios.ipa` |

### VibeWatch — Apple Watch & Wear OS

| Platform | Download |
|----------|----------|
| watchOS 10+ (unsigned — sideload via Xcode) | `VibeCody-WatchOS-v0.5.7.app.zip` |
| Wear OS 3+ (AAB) | `VibeCody-Wear-v0.5.7.aab` |
| Wear OS 3+ (APK) | `VibeCody-Wear-v0.5.7.apk` |

`SHA256SUMS.txt`

---

## Installing on macOS

Applies to every release; not specific to the version above.

### macOS install: the app is killed on first launch

**Symptom.** The app bounces once in the Dock and quits, or Finder says
*"«App» is damaged and can't be opened. You should move it to the Trash."*

**It is not damaged, and re-downloading will not help.** Every macOS artifact
built by CI is **ad-hoc signed** with the hardened runtime enabled, because the
release workflow has no Developer ID certificate (`APPLE_CERT_P12_BASE64` is
unset — see [macOS code signing setup](#macos-code-signing-setup-for-maintainers)).
macOS refuses to run that combination while the download-quarantine flag is set,
and it does so by **killing the process** — verified: the binary exits with
signal 9 (`exit=137`) while quarantined, and starts normally the moment the flag
is removed.

**Fix — remove the quarantine flag:**

```bash
xattr -dr com.apple.quarantine /Applications/VibeCoder.app
xattr -dr com.apple.quarantine /Applications/VibeAIChat.app
xattr -dr com.apple.quarantine /Applications/VibeDesk.app
```

Run it once per app, after moving it to `/Applications`. That is the whole fix;
the app launches immediately afterwards.

**Why not "right-click → Open"?** That is the standard advice for an
*unidentified developer* prompt, and it is what this page used to recommend
first. It does not reliably clear this state: the hardened runtime turns the
Gatekeeper rejection into a kill rather than a prompt, so there is often no
"Open anyway" dialog to click. Use the `xattr` command.

`spctl` will still report `rejected` afterwards. That is expected — it means
"not Developer ID signed", and it stops mattering once the quarantine flag is
gone, because Gatekeeper only assesses quarantined files.

**The real fix** is on the maintainer side: configure the signing secrets below
so releases ship Developer ID signed and notarized, at which point none of this
is necessary for anyone.

### macOS code signing setup (for maintainers)

**One-shot setup:** run [`scripts/setup-apple-signing.sh`](https://github.com/TuringWorks/vibecody/blob/main/scripts/setup-apple-signing.sh)
on a Mac that already has the Developer ID certificate installed. It picks the
identity, derives the Team ID from it, verifies the exported `.p12` actually
contains a private key, checks the notarization credentials against Apple
*before* uploading anything, and then sets all six secrets with `gh`. Nothing
leaves the machine except the encrypted secrets themselves.

**Coverage.** These six secrets sign **and notarize** VibeCoder, VibeAIChat,
VibeDesk *and* `vibecli`. A bare binary cannot carry a *stapled* ticket
(stapling only works for `.app`/`.dmg`/`.pkg`), so `vibecli`'s ticket is fetched
from Apple on first run — but it is notarized, which is what Gatekeeper checks.

**iOS and watchOS are a separate, optional phase** — and they need a *different
certificate*. Developer ID signs macOS apps distributed outside the App Store
and **cannot sign an iOS or watchOS bundle at all**; those need **Apple
Distribution**. The script uploads each to its own secret so every job imports
the identity it can actually use:

| Secret | Certificate | Used by |
|---|---|---|
| `APPLE_CERT_P12_BASE64` | Developer ID Application | VibeCoder, VibeAIChat, VibeDesk, vibecli |
| `APPLE_DIST_CERT_P12_BASE64` | Apple Distribution | VibeMobile (iOS), VibeCodyWatch |

On top of the certificate, each mobile target needs a distribution provisioning
profile for its App ID:

| Platform | App ID | Secret |
|---|---|---|
| iOS | `dev.vibecody.vibecodyMobile` | `APPLE_IOS_PROFILE_BASE64` |
| watchOS | `com.turingworks.vibecody.watch` | `APPLE_PROVISIONING_PROFILE_BASE64` |

Each is gated independently — missing secrets mean that platform ships unsigned
(and still sideloadable), never a failed release:

| Secrets present | Result |
|---|---|
| none | iOS/watchOS unsigned, as before |
| + distribution cert + profile | signed artifact |
| + `APPLE_ASC_*` | watchOS also pushed to TestFlight |

Android ships unsigned by design, for sideloading.

Each desktop job now verifies its own `.app` after building and **fails the
release** if it is still ad-hoc — the check `vibecli` always had and the app
bundles never did, which is why ad-hoc builds shipped unnoticed.

`APPLE_KEYCHAIN_PASSWORD` is **not** required — the workflow generates a
throwaway password per job for a keychain that never outlives it.

The manual equivalent is below.

**If Keychain Access greys out the `.p12` option,** you do not need it — the
script exports via the `security` CLI, which only asks for your keychain
password. The greying is usually one of:

| Cause | Fix |
|---|---|
| Viewing the **Certificates** category | Switch to **My Certificates** — only that view shows identities (certificate *plus* private key) |
| Selected the certificate, not the identity | Click the disclosure triangle so the certificate and its key are selected together |
| Several items selected at once | Select exactly one |
| Private key is **non-extractable** | Nothing can export it. Issue a new Developer ID certificate — generate the CSR from Keychain Access (*Certificate Assistant → Request a Certificate from a Certificate Authority*), which produces an extractable key |

The script exports every identity, keeps only the one being used (matching the
certificate to its private key by public modulus, since order is not
guaranteed), and rebuilds a `.p12` with a single private key. Other
certificates travel as chain material only — public data, no keys.


To ship fully Apple-notarized builds (no first-launch warning at all), add the following repository secrets:

| Secret | What it is |
|---|---|
| `APPLE_TEAM_ID` | Your 10-char Apple Developer Team ID |
| `APPLE_SIGNING_IDENTITY` | Full identity string, e.g. `Developer ID Application: Acme Inc (TEAMID)` |
| `APPLE_CERT_P12_BASE64` | `base64 -i DeveloperID.p12` of your exported Developer ID Application certificate |
| `APPLE_CERT_P12_PASSWORD` | Password for the `.p12` |
| `APPLE_KEYCHAIN_PASSWORD` | Any random string — used to lock the throwaway runner keychain |
| `APPLE_ID` | Your Apple ID email (for notarization) |
| `APPLE_APP_SPECIFIC_PASSWORD` | App-specific password generated at appleid.apple.com (NOT your regular Apple ID password) |

**As of v0.5.8 none of these secrets are configured on the repository**, so a tag push produces ad-hoc app bundles and an unsigned `vibecli`. Confirm before cutting a release you intend to be signed:

```bash
gh api repos/TuringWorks/vibecody/actions/secrets --jq '.secrets[].name'
```

Upload them with `gh secret set NAME` (the `.p12` export and its password come from Keychain Access → export the Developer ID Application identity).

The `build-cli`, `build-vibecoder`, `build-vibeaichat` and `build-vibedesk` jobs auto-detect these secrets — when `APPLE_CERT_P12_BASE64` is unset, the build emits a workflow `::notice::` and falls back to ad-hoc signing. The watchOS-signed track uses a parallel set of secrets (`APPLE_PROVISIONING_PROFILE_BASE64` + App Store Connect API key) — see `.github/workflows/release.yml` `build-watchos-signed` for that path.

> **VibeDesk ships from v0.5.8.** It has a `build-vibedesk` release job (same five-platform matrix and dual-mode signing as the other two shells) and a `vibedesk-checks` CI job; it was absent from v0.5.7 and earlier. Two things to know about it:
>
> - It is **not** in the `release` job's critical `if:` gate yet. The job has never run on Linux or Windows, so a failure there must not block the whole release. Promote it by adding `needs.build-vibedesk.result == 'success'` once it has shipped green.
> - Its `src-tauri` embeds the whole `vibecli` crate for direct ProfileStore access — exactly as VibeCoder does — so its build cost tracks VibeCoder's. On macOS that includes the Metal mistral.rs backend, which `vibecli` pulls unconditionally through a `[target.'cfg(target_os = "macos")']` block regardless of feature flags.
>
>   As of v0.5.8 **all three shells** carry that cost: VibeAIChat took the `vibecli` dependency too, so it can autostart the daemon through the shared `daemon_bootstrap` instead of having no way to start one. It previously embedded no Rust of its own and built in a fraction of the time.

#### Signing and notarizing locally

CI is not required — a local `make build-ui` will sign and notarize once the same variables are in your environment. Two prerequisites:

1. **The Developer ID Application certificate must be in your login keychain.** The `APPLE_CERT_P12_BASE64` secret is a CI-only mechanism for importing it into a throwaway runner keychain; locally you install the cert once from your Apple Developer account. Confirm it is there:
   ```bash
   security find-identity -v -p codesigning | grep "Developer ID Application"
   ```
2. **`APPLE_PASSWORD` must be an app-specific password**, generated at [appleid.apple.com](https://appleid.apple.com) → Sign-In and Security → App-Specific Passwords. Your normal Apple ID password is rejected.

Then export the four variables and build:

```bash
export APPLE_SIGNING_IDENTITY="Developer ID Application: Acme Inc (TEAMID)"
export APPLE_TEAM_ID="XXXXXXXXXX"
export APPLE_ID="you@example.com"
export APPLE_PASSWORD="xxxx-xxxx-xxxx-xxxx"   # app-specific, not your Apple ID password

make build-ui
```

`APPLE_SIGNING_IDENTITY` **overrides** the `"signingIdentity": "-"` in `tauri.conf.json`, so the committed config stays ad-hoc and no one needs a certificate for an ordinary development build. Leave the variable unset and you get today's behaviour.

Notarization is a round trip to Apple's servers — budget **2–15 minutes per build** on top of compile time. If you only want a runnable app, skip it; the warning in the build log (`skipping app notarization…`) is informational, not an error.

Verify the result:

```bash
codesign -dv --verbose=4 target/release/bundle/macos/VibeCoder.app   # expect your Developer ID, not "adhoc"
xcrun stapler validate target/release/bundle/macos/VibeCoder.app     # expect "The validate action worked!"
spctl -a -vvv -t install target/release/bundle/macos/VibeCoder.app   # expect "accepted / source=Notarized Developer ID"
```

#### Building all three shells locally keeps only the last DMG

The three Tauri shells share the workspace target dir, and Tauri clears
`target/release/bundle/dmg/` before writing its own image. Build them in
sequence and you get three DMGs built, three DMGs signed — and one DMG on
disk. The `.app` bundles live in a sibling directory and all three survive,
which is what makes the loss easy to miss: the build log reports every DMG
created and signed, and checking the apps shows all three present.

`make build-apps` stages each shell's bundles into `dist/` before the next
build clears them. If you drive `tauri build` by hand, copy the DMG out
between shells.

CI is unaffected: each shell builds on its own runner with its own target dir.

Related: `tauri build --bundles dmg` *deletes* the `.app` after bundling it, so
re-running it for one shell removes that shell's app bundle. The app is inside
the DMG; mount it if you need the bundle back.

#### DMG bundling fails with `error running bundle_dmg.sh`

`bundle_dmg.sh` drives Finder over AppleScript to position icons and set the window background. When that `osascript` call fails the script exits `64`, and Tauri reports only `failed to run …/bundle_dmg.sh` — the underlying reason is not surfaced.

**The reliable fix is to skip the AppleScript**, which Tauri does when it is told it is running in CI:

```bash
CI=true make build-ui        # note: CI=true, not CI=1
```

Tauri binds the `CI` environment variable to its own `--ci` flag, which accepts only `true` or `false` — `CI=1` fails immediately with `error: invalid value '1' for '--ci'`. With `CI=true` the bundler passes `--skip-jenkins`, the `osascript` call never happens, and the DMG builds. You lose only the cosmetics: custom window size, icon positions and background image. Drag-to-Applications still works, because the `Applications` symlink is created before the AppleScript runs.

This was verified on a machine where the styled path failed twice and `CI=true` succeeded immediately.

Other things worth checking:

- **A leaked scratch volume.** A failed run leaves its disk image mounted, which breaks every retry with a confusing second error. Always clean up before rebuilding:
  ```bash
  ls -d /Volumes/dmg.*                     # any match is a leftover
  hdiutil detach /Volumes/dmg.XXXXXX
  rm -f target/release/bundle/macos/rw.*.dmg
  ```
- **Automation permission**, if you want the styled DMG: System Settings → Privacy & Security → Automation → your terminal app → enable Finder. A previous "Don't Allow" is remembered and must be re-enabled here. Note this is per-app, and the process that actually sends the Apple event is `node`/`osascript` under your shell — a grant for one terminal does not cover a build launched from another.
- **The `-1728` "Can't get disk" race.** `bundle_dmg.sh` guards it with a fixed 2-second sleep after mounting, which is not always enough on a machine that has just finished a heavy LTO link. Running the same script standalone on an idle machine can succeed where it failed under the build — so an intermittent failure here does not mean your permissions changed.
- **No GUI session at all** (SSH, a headless runner): there is no Finder to talk to, so `CI=true` is the only option.

---

## Verify Downloads

```bash
# Download the checksums file
curl -LO https://github.com/TuringWorks/vibecody/releases/download/v0.5.8/SHA256SUMS.txt

# Verify
sha256sum -c SHA256SUMS.txt
```

---

## Release History

For older releases (v0.5.6 and earlier), see [github.com/TuringWorks/vibecody/releases](https://github.com/TuringWorks/vibecody/releases).
