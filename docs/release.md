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

## v0.5.0 — Latest

**Released:** March 24, 2026 &middot; [Release notes](https://github.com/TuringWorks/vibecody/releases/tag/v0.5.0) &middot; [Changelog](https://github.com/TuringWorks/vibecody/compare/v0.4.0...v0.5.0)

Quantum computing tools, panel consolidation (137→36 tabs), full-stack resilience.

### Highlights
- **9 quantum computing tools** — statevector simulator, visual circuit builder, optimizer, Bloch sphere, cost estimator, project scaffolding, algorithm templates, hardware topology viewer, multi-language code examples
- **Panel consolidation** — 137 panels → 36 composite tabs with internal sub-tabs
- **Resilience** — automatic retry with exponential backoff on all 21 AI providers + 30+ HTTP API calls

### VibeCLI — Terminal AI Assistant

| Platform | Download | Size |
|----------|----------|------|
| macOS (Apple Silicon) | [vibecli-aarch64-apple-darwin.tar.gz](https://github.com/TuringWorks/vibecody/releases/download/v0.5.0/vibecli-aarch64-apple-darwin.tar.gz) | 7.5 MB |
| macOS (Intel) | [vibecli-x86_64-apple-darwin.tar.gz](https://github.com/TuringWorks/vibecody/releases/download/v0.5.0/vibecli-x86_64-apple-darwin.tar.gz) | 7.7 MB |
| Linux x86_64 (musl) | [vibecli-x86_64-linux.tar.gz](https://github.com/TuringWorks/vibecody/releases/download/v0.5.0/vibecli-x86_64-linux.tar.gz) | 8.6 MB |
| Linux ARM64 (musl) | [vibecli-aarch64-linux.tar.gz](https://github.com/TuringWorks/vibecody/releases/download/v0.5.0/vibecli-aarch64-linux.tar.gz) | 9.1 MB |
| Windows x64 | [vibecli-x86_64-windows.zip](https://github.com/TuringWorks/vibecody/releases/download/v0.5.0/vibecli-x86_64-windows.zip) | 6.6 MB |
| Docker | [vibecli-docker-v0.5.0.tar.gz](https://github.com/TuringWorks/vibecody/releases/download/v0.5.0/vibecli-docker-v0.5.0.tar.gz) | 12.8 MB |

### VibeUI — Desktop Code Editor

| Platform | Download | Size |
|----------|----------|------|
| macOS (Apple Silicon) | [VibeUI_0.5.0_aarch64.dmg](https://github.com/TuringWorks/vibecody/releases/download/v0.5.0/VibeUI_0.5.0_aarch64.dmg) | 8.6 MB |
| macOS (Intel) | [VibeUI_0.5.0_x64.dmg](https://github.com/TuringWorks/vibecody/releases/download/v0.5.0/VibeUI_0.5.0_x64.dmg) | 8.9 MB |
| macOS (Apple Silicon, .app) | [VibeUI-macOS-arm64.app.zip](https://github.com/TuringWorks/vibecody/releases/download/v0.5.0/VibeUI-macOS-arm64.app.zip) | 8.5 MB |
| macOS (Intel, .app) | [VibeUI-macOS-x64.app.zip](https://github.com/TuringWorks/vibecody/releases/download/v0.5.0/VibeUI-macOS-x64.app.zip) | 8.8 MB |
| Linux x64 (.deb) | [VibeUI_0.5.0_amd64.deb](https://github.com/TuringWorks/vibecody/releases/download/v0.5.0/VibeUI_0.5.0_amd64.deb) | 9.9 MB |
| Linux x64 (.AppImage) | [VibeUI_0.5.0_amd64.AppImage](https://github.com/TuringWorks/vibecody/releases/download/v0.5.0/VibeUI_0.5.0_amd64.AppImage) | 83.2 MB |
| Windows x64 (.msi) | [VibeUI_0.5.0_x64_en-US.msi](https://github.com/TuringWorks/vibecody/releases/download/v0.5.0/VibeUI_0.5.0_x64_en-US.msi) | 7.5 MB |
| Windows x64 (.exe) | [VibeUI_0.5.0_x64-setup.exe](https://github.com/TuringWorks/vibecody/releases/download/v0.5.0/VibeUI_0.5.0_x64-setup.exe) | 5.6 MB |

### VibeCLI App — Desktop AI Assistant

| Platform | Download | Size |
|----------|----------|------|
| macOS (Apple Silicon) | [VibeCLI_0.5.0_aarch64.dmg](https://github.com/TuringWorks/vibecody/releases/download/v0.5.0/VibeCLI_0.5.0_aarch64.dmg) | 2.3 MB |
| macOS (Intel) | [VibeCLI_0.5.0_x64.dmg](https://github.com/TuringWorks/vibecody/releases/download/v0.5.0/VibeCLI_0.5.0_x64.dmg) | 2.5 MB |
| Linux x64 (.deb) | [VibeCLI_0.5.0_amd64.deb](https://github.com/TuringWorks/vibecody/releases/download/v0.5.0/VibeCLI_0.5.0_amd64.deb) | 2.4 MB |
| Linux x64 (.AppImage) | [VibeCLI_0.5.0_amd64.AppImage](https://github.com/TuringWorks/vibecody/releases/download/v0.5.0/VibeCLI_0.5.0_amd64.AppImage) | 76.8 MB |
| Windows x64 (.msi) | [VibeCLI_0.5.0_x64_en-US.msi](https://github.com/TuringWorks/vibecody/releases/download/v0.5.0/VibeCLI_0.5.0_x64_en-US.msi) | 2.5 MB |
| Windows x64 (.exe) | [VibeCLI_0.5.0_x64-setup.exe](https://github.com/TuringWorks/vibecody/releases/download/v0.5.0/VibeCLI_0.5.0_x64-setup.exe) | 1.8 MB |

[SHA256SUMS.txt](https://github.com/TuringWorks/vibecody/releases/download/v0.5.0/SHA256SUMS.txt)

---

## v0.4.0

**Released:** March 21, 2026 &middot; [Release notes](https://github.com/TuringWorks/vibecody/releases/tag/v0.4.0) &middot; [Changelog](https://github.com/TuringWorks/vibecody/compare/v0.3.3...v0.4.0)

22 AI providers, 155+ panels, 543 skills, 7,400+ tests.

### VibeCLI — Terminal AI Assistant

| Platform | Download | Size |
|----------|----------|------|
| macOS (Apple Silicon) | [vibecli-aarch64-apple-darwin.tar.gz](https://github.com/TuringWorks/vibecody/releases/download/v0.4.0/vibecli-aarch64-apple-darwin.tar.gz) | 7.3 MB |
| macOS (Intel) | [vibecli-x86_64-apple-darwin.tar.gz](https://github.com/TuringWorks/vibecody/releases/download/v0.4.0/vibecli-x86_64-apple-darwin.tar.gz) | 7.5 MB |
| Linux x86_64 (musl) | [vibecli-x86_64-linux.tar.gz](https://github.com/TuringWorks/vibecody/releases/download/v0.4.0/vibecli-x86_64-linux.tar.gz) | 8.4 MB |
| Linux ARM64 (musl) | [vibecli-aarch64-linux.tar.gz](https://github.com/TuringWorks/vibecody/releases/download/v0.4.0/vibecli-aarch64-linux.tar.gz) | 8.9 MB |
| Windows x64 | [vibecli-x86_64-windows.zip](https://github.com/TuringWorks/vibecody/releases/download/v0.4.0/vibecli-x86_64-windows.zip) | 6.4 MB |
| Docker | [vibecli-docker-v0.4.0.tar.gz](https://github.com/TuringWorks/vibecody/releases/download/v0.4.0/vibecli-docker-v0.4.0.tar.gz) | 12.5 MB |

### VibeUI — Desktop Code Editor

| Platform | Download | Size |
|----------|----------|------|
| macOS (Apple Silicon) | [VibeUI_0.4.0_aarch64.dmg](https://github.com/TuringWorks/vibecody/releases/download/v0.4.0/VibeUI_0.4.0_aarch64.dmg) | 8.4 MB |
| macOS (Intel) | [VibeUI_0.4.0_x64.dmg](https://github.com/TuringWorks/vibecody/releases/download/v0.4.0/VibeUI_0.4.0_x64.dmg) | 8.7 MB |
| macOS (Apple Silicon, .app) | [VibeUI-macOS-arm64.app.zip](https://github.com/TuringWorks/vibecody/releases/download/v0.4.0/VibeUI-macOS-arm64.app.zip) | 8.3 MB |
| macOS (Intel, .app) | [VibeUI-macOS-x64.app.zip](https://github.com/TuringWorks/vibecody/releases/download/v0.4.0/VibeUI-macOS-x64.app.zip) | 8.6 MB |
| Linux x64 (.deb) | [VibeUI_0.4.0_amd64.deb](https://github.com/TuringWorks/vibecody/releases/download/v0.4.0/VibeUI_0.4.0_amd64.deb) | 9.7 MB |
| Linux x64 (.AppImage) | [VibeUI_0.4.0_amd64.AppImage](https://github.com/TuringWorks/vibecody/releases/download/v0.4.0/VibeUI_0.4.0_amd64.AppImage) | 83.0 MB |
| Windows x64 (.msi) | [VibeUI_0.4.0_x64_en-US.msi](https://github.com/TuringWorks/vibecody/releases/download/v0.4.0/VibeUI_0.4.0_x64_en-US.msi) | 7.3 MB |
| Windows x64 (.exe) | [VibeUI_0.4.0_x64-setup.exe](https://github.com/TuringWorks/vibecody/releases/download/v0.4.0/VibeUI_0.4.0_x64-setup.exe) | 5.4 MB |

### VibeCLI App — Desktop AI Assistant

| Platform | Download | Size |
|----------|----------|------|
| macOS (Apple Silicon) | [VibeCLI_0.4.0_aarch64.dmg](https://github.com/TuringWorks/vibecody/releases/download/v0.4.0/VibeCLI_0.4.0_aarch64.dmg) | 2.2 MB |
| macOS (Intel) | [VibeCLI_0.4.0_x64.dmg](https://github.com/TuringWorks/vibecody/releases/download/v0.4.0/VibeCLI_0.4.0_x64.dmg) | 2.4 MB |
| Linux x64 (.deb) | [VibeCLI_0.4.0_amd64.deb](https://github.com/TuringWorks/vibecody/releases/download/v0.4.0/VibeCLI_0.4.0_amd64.deb) | 2.3 MB |
| Linux x64 (.AppImage) | [VibeCLI_0.4.0_amd64.AppImage](https://github.com/TuringWorks/vibecody/releases/download/v0.4.0/VibeCLI_0.4.0_amd64.AppImage) | 76.6 MB |
| Windows x64 (.msi) | [VibeCLI_0.4.0_x64_en-US.msi](https://github.com/TuringWorks/vibecody/releases/download/v0.4.0/VibeCLI_0.4.0_x64_en-US.msi) | 2.4 MB |
| Windows x64 (.exe) | [VibeCLI_0.4.0_x64-setup.exe](https://github.com/TuringWorks/vibecody/releases/download/v0.4.0/VibeCLI_0.4.0_x64-setup.exe) | 1.7 MB |

[SHA256SUMS.txt](https://github.com/TuringWorks/vibecody/releases/download/v0.4.0/SHA256SUMS.txt)

---

## v0.3.3

**Released:** March 19, 2026 &middot; [Release notes](https://github.com/TuringWorks/vibecody/releases/tag/v0.3.3) &middot; [Changelog](https://github.com/TuringWorks/vibecody/compare/v0.3.2...v0.3.3)

### VibeCLI — Terminal AI Assistant

| Platform | Download | Size |
|----------|----------|------|
| macOS (Apple Silicon) | [vibecli-aarch64-apple-darwin.tar.gz](https://github.com/TuringWorks/vibecody/releases/download/v0.3.3/vibecli-aarch64-apple-darwin.tar.gz) | 7.0 MB |
| macOS (Intel) | [vibecli-x86_64-apple-darwin.tar.gz](https://github.com/TuringWorks/vibecody/releases/download/v0.3.3/vibecli-x86_64-apple-darwin.tar.gz) | 7.2 MB |
| Linux x86_64 (musl) | [vibecli-x86_64-linux.tar.gz](https://github.com/TuringWorks/vibecody/releases/download/v0.3.3/vibecli-x86_64-linux.tar.gz) | 8.0 MB |
| Linux ARM64 (musl) | [vibecli-aarch64-linux.tar.gz](https://github.com/TuringWorks/vibecody/releases/download/v0.3.3/vibecli-aarch64-linux.tar.gz) | 8.6 MB |
| Windows x64 | [vibecli-x86_64-windows.zip](https://github.com/TuringWorks/vibecody/releases/download/v0.3.3/vibecli-x86_64-windows.zip) | 6.0 MB |
| Docker | [vibecli-docker-v0.3.3.tar.gz](https://github.com/TuringWorks/vibecody/releases/download/v0.3.3/vibecli-docker-v0.3.3.tar.gz) | 12.1 MB |

### VibeUI — Desktop Code Editor

| Platform | Download | Size |
|----------|----------|------|
| macOS (Apple Silicon) | [VibeUI_0.3.3_aarch64.dmg](https://github.com/TuringWorks/vibecody/releases/download/v0.3.3/VibeUI_0.3.3_aarch64.dmg) | 8.3 MB |
| macOS (Intel) | [VibeUI_0.3.3_x64.dmg](https://github.com/TuringWorks/vibecody/releases/download/v0.3.3/VibeUI_0.3.3_x64.dmg) | 8.6 MB |
| macOS (Apple Silicon, .app) | [VibeUI-macOS-arm64.app.zip](https://github.com/TuringWorks/vibecody/releases/download/v0.3.3/VibeUI-macOS-arm64.app.zip) | 8.2 MB |
| macOS (Intel, .app) | [VibeUI-macOS-x64.app.zip](https://github.com/TuringWorks/vibecody/releases/download/v0.3.3/VibeUI-macOS-x64.app.zip) | 8.4 MB |
| Linux x64 (.deb) | [VibeUI_0.3.3_amd64.deb](https://github.com/TuringWorks/vibecody/releases/download/v0.3.3/VibeUI_0.3.3_amd64.deb) | 9.6 MB |
| Linux x64 (.AppImage) | [VibeUI_0.3.3_amd64.AppImage](https://github.com/TuringWorks/vibecody/releases/download/v0.3.3/VibeUI_0.3.3_amd64.AppImage) | 82.9 MB |
| Windows x64 (.msi) | [VibeUI_0.3.3_x64_en-US.msi](https://github.com/TuringWorks/vibecody/releases/download/v0.3.3/VibeUI_0.3.3_x64_en-US.msi) | 7.2 MB |
| Windows x64 (.exe) | [VibeUI_0.3.3_x64-setup.exe](https://github.com/TuringWorks/vibecody/releases/download/v0.3.3/VibeUI_0.3.3_x64-setup.exe) | 5.4 MB |

### VibeCLI App — Desktop AI Assistant

| Platform | Download | Size |
|----------|----------|------|
| macOS (Apple Silicon) | [VibeCLI_0.3.3_aarch64.dmg](https://github.com/TuringWorks/vibecody/releases/download/v0.3.3/VibeCLI_0.3.3_aarch64.dmg) | 2.2 MB |
| macOS (Intel) | [VibeCLI_0.3.3_x64.dmg](https://github.com/TuringWorks/vibecody/releases/download/v0.3.3/VibeCLI_0.3.3_x64.dmg) | 2.4 MB |
| Linux x64 (.deb) | [VibeCLI_0.3.3_amd64.deb](https://github.com/TuringWorks/vibecody/releases/download/v0.3.3/VibeCLI_0.3.3_amd64.deb) | 2.3 MB |
| Linux x64 (.AppImage) | [VibeCLI_0.3.3_amd64.AppImage](https://github.com/TuringWorks/vibecody/releases/download/v0.3.3/VibeCLI_0.3.3_amd64.AppImage) | 76.6 MB |
| Windows x64 (.msi) | [VibeCLI_0.3.3_x64_en-US.msi](https://github.com/TuringWorks/vibecody/releases/download/v0.3.3/VibeCLI_0.3.3_x64_en-US.msi) | 2.4 MB |
| Windows x64 (.exe) | [VibeCLI_0.3.3_x64-setup.exe](https://github.com/TuringWorks/vibecody/releases/download/v0.3.3/VibeCLI_0.3.3_x64-setup.exe) | 1.7 MB |

[SHA256SUMS.txt](https://github.com/TuringWorks/vibecody/releases/download/v0.3.3/SHA256SUMS.txt)

---

## v0.3.2

**Released:** March 17, 2026 &middot; [Release notes](https://github.com/TuringWorks/vibecody/releases/tag/v0.3.2) &middot; [Changelog](https://github.com/TuringWorks/vibecody/compare/v0.3.1...v0.3.2)

### VibeCLI — Terminal AI Assistant

| Platform | Download | Size |
|----------|----------|------|
| macOS (Apple Silicon) | [vibecli-aarch64-apple-darwin.tar.gz](https://github.com/TuringWorks/vibecody/releases/download/v0.3.2/vibecli-aarch64-apple-darwin.tar.gz) | 7.0 MB |
| macOS (Intel) | [vibecli-x86_64-apple-darwin.tar.gz](https://github.com/TuringWorks/vibecody/releases/download/v0.3.2/vibecli-x86_64-apple-darwin.tar.gz) | 7.2 MB |
| Linux x86_64 (musl) | [vibecli-x86_64-linux.tar.gz](https://github.com/TuringWorks/vibecody/releases/download/v0.3.2/vibecli-x86_64-linux.tar.gz) | 8.0 MB |
| Linux ARM64 (musl) | [vibecli-aarch64-linux.tar.gz](https://github.com/TuringWorks/vibecody/releases/download/v0.3.2/vibecli-aarch64-linux.tar.gz) | 8.6 MB |
| Windows x64 | [vibecli-x86_64-windows.zip](https://github.com/TuringWorks/vibecody/releases/download/v0.3.2/vibecli-x86_64-windows.zip) | 6.0 MB |
| Docker | [vibecli-docker-v0.3.2.tar.gz](https://github.com/TuringWorks/vibecody/releases/download/v0.3.2/vibecli-docker-v0.3.2.tar.gz) | 12.1 MB |

### VibeUI — Desktop Code Editor

| Platform | Download | Size |
|----------|----------|------|
| macOS (Apple Silicon) | [VibeUI_0.3.2_aarch64.dmg](https://github.com/TuringWorks/vibecody/releases/download/v0.3.2/VibeUI_0.3.2_aarch64.dmg) | 8.2 MB |
| macOS (Intel) | [VibeUI_0.3.2_x64.dmg](https://github.com/TuringWorks/vibecody/releases/download/v0.3.2/VibeUI_0.3.2_x64.dmg) | 8.4 MB |
| macOS (Apple Silicon, .app) | [VibeUI-macOS-arm64.app.zip](https://github.com/TuringWorks/vibecody/releases/download/v0.3.2/VibeUI-macOS-arm64.app.zip) | 8.0 MB |
| macOS (Intel, .app) | [VibeUI-macOS-x64.app.zip](https://github.com/TuringWorks/vibecody/releases/download/v0.3.2/VibeUI-macOS-x64.app.zip) | 8.2 MB |
| Linux x64 (.deb) | [VibeUI_0.3.2_amd64.deb](https://github.com/TuringWorks/vibecody/releases/download/v0.3.2/VibeUI_0.3.2_amd64.deb) | 9.4 MB |
| Linux x64 (.AppImage) | [VibeUI_0.3.2_amd64.AppImage](https://github.com/TuringWorks/vibecody/releases/download/v0.3.2/VibeUI_0.3.2_amd64.AppImage) | 82.8 MB |
| Windows x64 (.msi) | [VibeUI_0.3.2_x64_en-US.msi](https://github.com/TuringWorks/vibecody/releases/download/v0.3.2/VibeUI_0.3.2_x64_en-US.msi) | 7.0 MB |
| Windows x64 (.exe) | [VibeUI_0.3.2_x64-setup.exe](https://github.com/TuringWorks/vibecody/releases/download/v0.3.2/VibeUI_0.3.2_x64-setup.exe) | 5.2 MB |

### VibeCLI App — Desktop AI Assistant

| Platform | Download | Size |
|----------|----------|------|
| macOS (Apple Silicon) | [VibeCLI_0.3.2_aarch64.dmg](https://github.com/TuringWorks/vibecody/releases/download/v0.3.2/VibeCLI_0.3.2_aarch64.dmg) | 2.2 MB |
| macOS (Intel) | [VibeCLI_0.3.2_x64.dmg](https://github.com/TuringWorks/vibecody/releases/download/v0.3.2/VibeCLI_0.3.2_x64.dmg) | 2.4 MB |
| Linux x64 (.deb) | [VibeCLI_0.3.2_amd64.deb](https://github.com/TuringWorks/vibecody/releases/download/v0.3.2/VibeCLI_0.3.2_amd64.deb) | 2.3 MB |
| Linux x64 (.AppImage) | [VibeCLI_0.3.2_amd64.AppImage](https://github.com/TuringWorks/vibecody/releases/download/v0.3.2/VibeCLI_0.3.2_amd64.AppImage) | 76.6 MB |
| Windows x64 (.msi) | [VibeCLI_0.3.2_x64_en-US.msi](https://github.com/TuringWorks/vibecody/releases/download/v0.3.2/VibeCLI_0.3.2_x64_en-US.msi) | 2.4 MB |
| Windows x64 (.exe) | [VibeCLI_0.3.2_x64-setup.exe](https://github.com/TuringWorks/vibecody/releases/download/v0.3.2/VibeCLI_0.3.2_x64-setup.exe) | 1.7 MB |

[SHA256SUMS.txt](https://github.com/TuringWorks/vibecody/releases/download/v0.3.2/SHA256SUMS.txt)

---

## Verify Downloads

```bash
# Download the checksums file for your version
curl -LO https://github.com/TuringWorks/vibecody/releases/download/v0.5.0/SHA256SUMS.txt

# Verify
sha256sum -c SHA256SUMS.txt
```

## All Releases

See [github.com/TuringWorks/vibecody/releases](https://github.com/TuringWorks/vibecody/releases) for the complete release history.
