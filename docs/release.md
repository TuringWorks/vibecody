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

## v0.5.4 — Latest

**Released:** April 3, 2026 &middot; [Release notes](https://github.com/TuringWorks/vibecody/releases/tag/v0.5.4) &middot; [Changelog](https://github.com/TuringWorks/vibecody/compare/v0.5.3...v0.5.4)

Claude Code prompt integration, Apply crash fix, GLM/Qwen tool call support, incremental streaming saves.

### Highlights

- **254 Claude Code system prompts** integrated as reference skills — core behavioral guidelines baked into TOOL_SYSTEM_PROMPT
- **Apply crash resolved** — DiffReviewPanel overlays editor with deferred unmount; React.StrictMode removed
- **GLM/Qwen tool calls** — `<|tag|>` delimiters normalized so XML tool calls execute correctly
- **Incremental streaming saves** — `<write_file>` blocks flush to disk as closing tag arrives; partial work survives failures
- **Error Boundary** — catches render crashes with stack trace instead of blank WebView
- **Provider improvements** — unique names for 14 providers, LSP camelCase params, expanded token limits

### VibeCLI — Terminal AI Assistant

| Platform | Download | Size |
|----------|----------|------|
| macOS (Apple Silicon) | [vibecli-aarch64-apple-darwin.tar.gz](https://github.com/TuringWorks/vibecody/releases/download/v0.5.4/vibecli-aarch64-apple-darwin.tar.gz) | 7.6 MB |
| macOS (Intel) | [vibecli-x86_64-apple-darwin.tar.gz](https://github.com/TuringWorks/vibecody/releases/download/v0.5.4/vibecli-x86_64-apple-darwin.tar.gz) | 7.8 MB |
| Linux x86_64 (musl) | [vibecli-x86_64-linux.tar.gz](https://github.com/TuringWorks/vibecody/releases/download/v0.5.4/vibecli-x86_64-linux.tar.gz) | 8.7 MB |
| Linux ARM64 (musl) | [vibecli-aarch64-linux.tar.gz](https://github.com/TuringWorks/vibecody/releases/download/v0.5.4/vibecli-aarch64-linux.tar.gz) | 9.2 MB |
| Windows x64 | [vibecli-x86_64-windows.zip](https://github.com/TuringWorks/vibecody/releases/download/v0.5.4/vibecli-x86_64-windows.zip) | 6.7 MB |
| Docker | [vibecli-docker-v0.5.4.tar.gz](https://github.com/TuringWorks/vibecody/releases/download/v0.5.4/vibecli-docker-v0.5.4.tar.gz) | 12.9 MB |

### VibeUI — Desktop Code Editor

| Platform | Download | Size |
|----------|----------|------|
| macOS (Apple Silicon) | [VibeUI_0.5.4_aarch64.dmg](https://github.com/TuringWorks/vibecody/releases/download/v0.5.4/VibeUI_0.5.4_aarch64.dmg) | 8.7 MB |
| macOS (Intel) | [VibeUI_0.5.4_x64.dmg](https://github.com/TuringWorks/vibecody/releases/download/v0.5.4/VibeUI_0.5.4_x64.dmg) | 9.0 MB |
| macOS (Apple Silicon, .app) | [VibeUI-macOS-arm64.app.zip](https://github.com/TuringWorks/vibecody/releases/download/v0.5.4/VibeUI-macOS-arm64.app.zip) | 8.6 MB |
| macOS (Intel, .app) | [VibeUI-macOS-x64.app.zip](https://github.com/TuringWorks/vibecody/releases/download/v0.5.4/VibeUI-macOS-x64.app.zip) | 8.9 MB |
| Linux x64 (.deb) | [VibeUI_0.5.4_amd64.deb](https://github.com/TuringWorks/vibecody/releases/download/v0.5.4/VibeUI_0.5.4_amd64.deb) | 10.0 MB |
| Linux x64 (.AppImage) | [VibeUI_0.5.4_amd64.AppImage](https://github.com/TuringWorks/vibecody/releases/download/v0.5.4/VibeUI_0.5.4_amd64.AppImage) | 83.3 MB |
| Windows x64 (.msi) | [VibeUI_0.5.4_x64_en-US.msi](https://github.com/TuringWorks/vibecody/releases/download/v0.5.4/VibeUI_0.5.4_x64_en-US.msi) | 7.6 MB |
| Windows x64 (.exe) | [VibeUI_0.5.4_x64-setup.exe](https://github.com/TuringWorks/vibecody/releases/download/v0.5.4/VibeUI_0.5.4_x64-setup.exe) | 5.7 MB |

### VibeCLI App — Desktop AI Assistant

| Platform | Download | Size |
|----------|----------|------|
| macOS (Apple Silicon) | [VibeCLI_0.5.4_aarch64.dmg](https://github.com/TuringWorks/vibecody/releases/download/v0.5.4/VibeCLI_0.5.4_aarch64.dmg) | 2.4 MB |
| macOS (Intel) | [VibeCLI_0.5.4_x64.dmg](https://github.com/TuringWorks/vibecody/releases/download/v0.5.4/VibeCLI_0.5.4_x64.dmg) | 2.6 MB |
| Linux x64 (.deb) | [VibeCLI_0.5.4_amd64.deb](https://github.com/TuringWorks/vibecody/releases/download/v0.5.4/VibeCLI_0.5.4_amd64.deb) | 2.5 MB |
| Linux x64 (.AppImage) | [VibeCLI_0.5.4_amd64.AppImage](https://github.com/TuringWorks/vibecody/releases/download/v0.5.4/VibeCLI_0.5.4_amd64.AppImage) | 76.9 MB |
| Windows x64 (.msi) | [VibeCLI_0.5.4_x64_en-US.msi](https://github.com/TuringWorks/vibecody/releases/download/v0.5.4/VibeCLI_0.5.4_x64_en-US.msi) | 2.6 MB |
| Windows x64 (.exe) | [VibeCLI_0.5.4_x64-setup.exe](https://github.com/TuringWorks/vibecody/releases/download/v0.5.4/VibeCLI_0.5.4_x64-setup.exe) | 1.9 MB |

[SHA256SUMS.txt](https://github.com/TuringWorks/vibecody/releases/download/v0.5.4/SHA256SUMS.txt)

---

## v0.5.3

**Released:** April 2, 2026 &middot; [Release notes](https://github.com/TuringWorks/vibecody/releases/tag/v0.5.3) &middot; [Changelog](https://github.com/TuringWorks/vibecody/compare/v0.5.2...v0.5.3)

Document/media viewers, RL-OS core modules (660 tests), Sketch Canvas, Training Run Wizard.

---

## v0.5.2

**Released:** March 30, 2026 &middot; [Release notes](https://github.com/TuringWorks/vibecody/releases/tag/v0.5.2) &middot; [Changelog](https://github.com/TuringWorks/vibecody/compare/v0.5.1...v0.5.2)

RL-OS architecture (40+ competitors, 52 gaps, 12 unique capabilities), AI code review, architecture spec, policy engine.

---

## v0.5.0

**Released:** March 24, 2026 &middot; [Release notes](https://github.com/TuringWorks/vibecody/releases/tag/v0.5.0) &middot; [Changelog](https://github.com/TuringWorks/vibecody/compare/v0.4.0...v0.5.0)

Quantum computing tools, panel consolidation (137→36 tabs), full-stack resilience.

---

## Older Releases

See [github.com/TuringWorks/vibecody/releases](https://github.com/TuringWorks/vibecody/releases) for download links for v0.4.0 and earlier.

---

## Verify Downloads

```bash
# Download the checksums file for your version
curl -LO https://github.com/TuringWorks/vibecody/releases/download/v0.5.4/SHA256SUMS.txt

# Verify
sha256sum -c SHA256SUMS.txt
```

## All Releases

See [github.com/TuringWorks/vibecody/releases](https://github.com/TuringWorks/vibecody/releases) for the complete release history.
