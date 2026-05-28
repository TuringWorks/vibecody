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

## v0.5.6 — Latest

**Released:** May 27, 2026 &middot; [Release notes](https://github.com/TuringWorks/vibecody/releases/tag/v0.5.6) &middot; [Changelog](https://github.com/TuringWorks/vibecody/compare/v0.5.5...v0.5.6)

### Highlights

- **Plugin system end-to-end** — Signed MCPB bundles, governance panel, skills in MCP, hook dispatch on daemon + CLI paths, plugin rules in agent context
- **/goal lifecycle (G1–G13)** — Create, tree-view, pin/unpin, keyword search, tags, goal-aware agent preamble, pinned-goal banner, cross-surface ★ parity
- **Sandbox tiers (F0–F8, H0–H6)** — Firecracker + Hyperlight skeletons, rootfs builder + CI, vsock broker, virtio-fs, skill-level policies, sandbox-doctor probe
- **Security posture scanner** — Unifier panel, SonarQube + taint adapters, persistence + audit log; path_guard promoted to vibe-core
- **Phone relay consolidation** — WatchConnectivityBridge + WearDataLayerService moved into VibeMobile; Flutter relay_bridge.dart pushes credentials to native keystores
- **Hook protocol parity** — VS Code + JetBrains implement the same 7-event hook contract as the CLI
- **MCP Apps embedding host** — Generic React host for MCP `app` resources in sandboxed iframes
- **TurboQuant memory index** — 8× smaller on disk; `/memory/stats` telemetry; `vibe-infer` crate with opt-in candle backend
- **Dependency refresh** — Rust (tauri 2.11.2, reqwest 0.13.4, hyper 1.10), npm (106 packages), Flutter (shared_preferences, path_provider)

### VibeCLI — Terminal AI Assistant

| Platform | Download |
|----------|----------|
| macOS (Apple Silicon) | [vibecli-aarch64-apple-darwin.tar.gz](https://github.com/TuringWorks/vibecody/releases/download/v0.5.6/vibecli-aarch64-apple-darwin.tar.gz) |
| macOS (Intel) | [vibecli-x86_64-apple-darwin.tar.gz](https://github.com/TuringWorks/vibecody/releases/download/v0.5.6/vibecli-x86_64-apple-darwin.tar.gz) |
| Linux x86_64 (musl) | [vibecli-x86_64-linux.tar.gz](https://github.com/TuringWorks/vibecody/releases/download/v0.5.6/vibecli-x86_64-linux.tar.gz) |
| Linux ARM64 (musl) | [vibecli-aarch64-linux.tar.gz](https://github.com/TuringWorks/vibecody/releases/download/v0.5.6/vibecli-aarch64-linux.tar.gz) |
| Windows x64 | [vibecli-x86_64-windows.zip](https://github.com/TuringWorks/vibecody/releases/download/v0.5.6/vibecli-x86_64-windows.zip) |
| Docker | [vibecli-docker-v0.5.6.tar.gz](https://github.com/TuringWorks/vibecody/releases/download/v0.5.6/vibecli-docker-v0.5.6.tar.gz) |

### VibeUI — Desktop Code Editor

| Platform | Download |
|----------|----------|
| macOS (Apple Silicon) | [VibeUI_0.5.6_aarch64.dmg](https://github.com/TuringWorks/vibecody/releases/download/v0.5.6/VibeUI_0.5.6_aarch64.dmg) |
| macOS (Intel) | [VibeUI_0.5.6_x64.dmg](https://github.com/TuringWorks/vibecody/releases/download/v0.5.6/VibeUI_0.5.6_x64.dmg) |
| macOS (Apple Silicon, .app) | [VibeUI-macOS-arm64.app.zip](https://github.com/TuringWorks/vibecody/releases/download/v0.5.6/VibeUI-macOS-arm64.app.zip) |
| macOS (Intel, .app) | [VibeUI-macOS-x64.app.zip](https://github.com/TuringWorks/vibecody/releases/download/v0.5.6/VibeUI-macOS-x64.app.zip) |
| Linux x64 (.deb) | [VibeUI_0.5.6_amd64.deb](https://github.com/TuringWorks/vibecody/releases/download/v0.5.6/VibeUI_0.5.6_amd64.deb) |
| Linux x64 (.AppImage) | [VibeUI_0.5.6_amd64.AppImage](https://github.com/TuringWorks/vibecody/releases/download/v0.5.6/VibeUI_0.5.6_amd64.AppImage) |
| Windows x64 (.msi) | [VibeUI_0.5.6_x64_en-US.msi](https://github.com/TuringWorks/vibecody/releases/download/v0.5.6/VibeUI_0.5.6_x64_en-US.msi) |
| Windows x64 (.exe) | [VibeUI_0.5.6_x64-setup.exe](https://github.com/TuringWorks/vibecody/releases/download/v0.5.6/VibeUI_0.5.6_x64-setup.exe) |

### VibeCLI App — Desktop AI Assistant

| Platform | Download |
|----------|----------|
| macOS (Apple Silicon) | [VibeCLI_0.5.6_aarch64.dmg](https://github.com/TuringWorks/vibecody/releases/download/v0.5.6/VibeCLI_0.5.6_aarch64.dmg) |
| macOS (Intel) | [VibeCLI_0.5.6_x64.dmg](https://github.com/TuringWorks/vibecody/releases/download/v0.5.6/VibeCLI_0.5.6_x64.dmg) |
| Linux x64 (.deb) | [VibeCLI_0.5.6_amd64.deb](https://github.com/TuringWorks/vibecody/releases/download/v0.5.6/VibeCLI_0.5.6_amd64.deb) |
| Linux x64 (.AppImage) | [VibeCLI_0.5.6_amd64.AppImage](https://github.com/TuringWorks/vibecody/releases/download/v0.5.6/VibeCLI_0.5.6_amd64.AppImage) |
| Windows x64 (.msi) | [VibeCLI_0.5.6_x64_en-US.msi](https://github.com/TuringWorks/vibecody/releases/download/v0.5.6/VibeCLI_0.5.6_x64_en-US.msi) |
| Windows x64 (.exe) | [VibeCLI_0.5.6_x64-setup.exe](https://github.com/TuringWorks/vibecody/releases/download/v0.5.6/VibeCLI_0.5.6_x64-setup.exe) |

### VibeMobile — Flutter Companion

| Platform | Download |
|----------|----------|
| iOS (unsigned `.ipa` — sideload via AltStore/Sideloadly) | [VibeMobile-iOS.ipa](https://github.com/TuringWorks/vibecody/releases/download/v0.5.6/VibeMobile-iOS.ipa) |
| Android (`.apk`) | [VibeMobile-android.apk](https://github.com/TuringWorks/vibecody/releases/download/v0.5.6/VibeMobile-android.apk) |
| Android (`.aab`) | [VibeMobile-android.aab](https://github.com/TuringWorks/vibecody/releases/download/v0.5.6/VibeMobile-android.aab) |

### VibeWatch — Apple Watch & Wear OS

| Platform | Download |
|----------|----------|
| watchOS 10+ (unsigned `.app.zip` — sideload via Xcode) | [VibeCodyWatch-watchOS.app.zip](https://github.com/TuringWorks/vibecody/releases/download/v0.5.6/VibeCodyWatch-watchOS.app.zip) |
| Wear OS 3+ (`.apk`) | [VibeCodyWear-wearos.apk](https://github.com/TuringWorks/vibecody/releases/download/v0.5.6/VibeCodyWear-wearos.apk) |
| Wear OS 3+ (`.aab`) | [VibeCodyWear-wearos.aab](https://github.com/TuringWorks/vibecody/releases/download/v0.5.6/VibeCodyWear-wearos.aab) |

Install the companion desktop/phone app first — pair the watch from the **Watch Devices** panel in VibeUI (`Governance → Watch Devices`) or the Machine detail screen in VibeMobile. See [Watch Integration](/vibecody/watch-integration/) for the full architecture.

[SHA256SUMS.txt](https://github.com/TuringWorks/vibecody/releases/download/v0.5.6/SHA256SUMS.txt)

---

## Verify Downloads

```bash
# Download the checksums file
curl -LO https://github.com/TuringWorks/vibecody/releases/download/v0.5.6/SHA256SUMS.txt

# Verify
sha256sum -c SHA256SUMS.txt
```

---

## v0.5.5

**Released:** April 17, 2026 &middot; [Release notes](https://github.com/TuringWorks/vibecody/releases/tag/v0.5.5) &middot; [Changelog](https://github.com/TuringWorks/vibecody/compare/v0.5.4...v0.5.5) &middot; [Release notes](https://github.com/TuringWorks/vibecody/releases/tag/v0.5.5) &middot; [Changelog](https://github.com/TuringWorks/vibecody/compare/v0.5.4...v0.5.5)

Apple Watch + Wear OS native clients, Apple-Handoff-style session continuity, zero-config mDNS / Tailscale / ngrok connectivity, Google-Docs-style bidirectional sync with no truncation.

### Highlights

- **Apple Watch** (SwiftUI, watchOS 10+) and **Wear OS** (Kotlin/Compose, Wear OS 3+) native clients sharing a single `/watch/*` backend
- **P-256 ECDSA device pairing** via Apple Secure Enclave and Android Keystore / StrongBox (migrated from Ed25519 for Secure Enclave compatibility)
- **URL-only / Bearer pairing** on every platform — no QR code or JSON copy required; emulator-friendly
- **Google-Docs-style real-time sync** — ID-based message reconciliation with content-window dedup; no more 80/512-char truncation
- **Apple-Handoff-style session continuity** between desktop and phone; VibeUI auto-switches to the Sandbox tab when a watch opens a sandbox session
- **Zero-config connectivity** — mDNS LAN discovery on any IP range, Tailscale Funnel for public HTTPS, ngrok auto-detect + opt-in auto-start; the mobile app races all reachable paths
- **CI release pipeline** now produces watchOS `.app.zip` and Wear OS APK/AAB alongside the existing CLI / VibeUI / VibeCLI App / iOS / Android / Docker artifacts
- **TDD + BDD green** for `watch_auth`, `watch_bridge`, `watch_session_relay`, `mdns_announce`, `tailscale`, `ngrok`, plus a P-256 auth harness

### VibeCLI — Terminal AI Assistant

| Platform | Download |
|----------|----------|
| macOS (Apple Silicon) | [vibecli-aarch64-apple-darwin.tar.gz](https://github.com/TuringWorks/vibecody/releases/download/v0.5.5/vibecli-aarch64-apple-darwin.tar.gz) |
| macOS (Intel) | [vibecli-x86_64-apple-darwin.tar.gz](https://github.com/TuringWorks/vibecody/releases/download/v0.5.5/vibecli-x86_64-apple-darwin.tar.gz) |
| Linux x86_64 (musl) | [vibecli-x86_64-linux.tar.gz](https://github.com/TuringWorks/vibecody/releases/download/v0.5.5/vibecli-x86_64-linux.tar.gz) |
| Linux ARM64 (musl) | [vibecli-aarch64-linux.tar.gz](https://github.com/TuringWorks/vibecody/releases/download/v0.5.5/vibecli-aarch64-linux.tar.gz) |
| Windows x64 | [vibecli-x86_64-windows.zip](https://github.com/TuringWorks/vibecody/releases/download/v0.5.5/vibecli-x86_64-windows.zip) |
| Docker | [vibecli-docker-v0.5.5.tar.gz](https://github.com/TuringWorks/vibecody/releases/download/v0.5.5/vibecli-docker-v0.5.5.tar.gz) |

### VibeUI — Desktop Code Editor

| Platform | Download |
|----------|----------|
| macOS (Apple Silicon) | [VibeUI_0.5.5_aarch64.dmg](https://github.com/TuringWorks/vibecody/releases/download/v0.5.5/VibeUI_0.5.5_aarch64.dmg) |
| macOS (Intel) | [VibeUI_0.5.5_x64.dmg](https://github.com/TuringWorks/vibecody/releases/download/v0.5.5/VibeUI_0.5.5_x64.dmg) |
| macOS (Apple Silicon, .app) | [VibeUI-macOS-arm64.app.zip](https://github.com/TuringWorks/vibecody/releases/download/v0.5.5/VibeUI-macOS-arm64.app.zip) |
| macOS (Intel, .app) | [VibeUI-macOS-x64.app.zip](https://github.com/TuringWorks/vibecody/releases/download/v0.5.5/VibeUI-macOS-x64.app.zip) |
| Linux x64 (.deb) | [VibeUI_0.5.5_amd64.deb](https://github.com/TuringWorks/vibecody/releases/download/v0.5.5/VibeUI_0.5.5_amd64.deb) |
| Linux x64 (.AppImage) | [VibeUI_0.5.5_amd64.AppImage](https://github.com/TuringWorks/vibecody/releases/download/v0.5.5/VibeUI_0.5.5_amd64.AppImage) |
| Windows x64 (.msi) | [VibeUI_0.5.5_x64_en-US.msi](https://github.com/TuringWorks/vibecody/releases/download/v0.5.5/VibeUI_0.5.5_x64_en-US.msi) |
| Windows x64 (.exe) | [VibeUI_0.5.5_x64-setup.exe](https://github.com/TuringWorks/vibecody/releases/download/v0.5.5/VibeUI_0.5.5_x64-setup.exe) |

### VibeCLI App — Desktop AI Assistant

| Platform | Download |
|----------|----------|
| macOS (Apple Silicon) | [VibeCLI_0.5.5_aarch64.dmg](https://github.com/TuringWorks/vibecody/releases/download/v0.5.5/VibeCLI_0.5.5_aarch64.dmg) |
| macOS (Intel) | [VibeCLI_0.5.5_x64.dmg](https://github.com/TuringWorks/vibecody/releases/download/v0.5.5/VibeCLI_0.5.5_x64.dmg) |
| Linux x64 (.deb) | [VibeCLI_0.5.5_amd64.deb](https://github.com/TuringWorks/vibecody/releases/download/v0.5.5/VibeCLI_0.5.5_amd64.deb) |
| Linux x64 (.AppImage) | [VibeCLI_0.5.5_amd64.AppImage](https://github.com/TuringWorks/vibecody/releases/download/v0.5.5/VibeCLI_0.5.5_amd64.AppImage) |
| Windows x64 (.msi) | [VibeCLI_0.5.5_x64_en-US.msi](https://github.com/TuringWorks/vibecody/releases/download/v0.5.5/VibeCLI_0.5.5_x64_en-US.msi) |
| Windows x64 (.exe) | [VibeCLI_0.5.5_x64-setup.exe](https://github.com/TuringWorks/vibecody/releases/download/v0.5.5/VibeCLI_0.5.5_x64-setup.exe) |

### VibeMobile — Flutter Companion

| Platform | Download |
|----------|----------|
| iOS (unsigned `.ipa` — sideload via AltStore/Sideloadly) | [VibeMobile-iOS.ipa](https://github.com/TuringWorks/vibecody/releases/download/v0.5.5/VibeMobile-iOS.ipa) |
| Android (`.apk`) | [VibeMobile-android.apk](https://github.com/TuringWorks/vibecody/releases/download/v0.5.5/VibeMobile-android.apk) |
| Android (`.aab`) | [VibeMobile-android.aab](https://github.com/TuringWorks/vibecody/releases/download/v0.5.5/VibeMobile-android.aab) |

### VibeWatch — Apple Watch & Wear OS *(new in v0.5.5)*

| Platform | Download |
|----------|----------|
| watchOS 10+ (unsigned `.app.zip` — sideload via Xcode) | [VibeCodyWatch-watchOS.app.zip](https://github.com/TuringWorks/vibecody/releases/download/v0.5.5/VibeCodyWatch-watchOS.app.zip) |
| Wear OS 3+ (`.apk`) | [VibeCodyWear-wearos.apk](https://github.com/TuringWorks/vibecody/releases/download/v0.5.5/VibeCodyWear-wearos.apk) |
| Wear OS 3+ (`.aab`) | [VibeCodyWear-wearos.aab](https://github.com/TuringWorks/vibecody/releases/download/v0.5.5/VibeCodyWear-wearos.aab) |

Install the companion desktop/phone app first — pair the watch from the **Watch Devices** panel in VibeUI (`Governance → Watch Devices`) or the Machine detail screen in VibeMobile. See [Watch Integration](/vibecody/watch-integration/) for the full architecture.

[SHA256SUMS.txt](https://github.com/TuringWorks/vibecody/releases/download/v0.5.5/SHA256SUMS.txt)

---

## Verify Downloads

```bash
# Download the checksums file
curl -LO https://github.com/TuringWorks/vibecody/releases/download/v0.5.5/SHA256SUMS.txt

# Verify
sha256sum -c SHA256SUMS.txt
```

---

## Release History

For older releases (v0.5.4 and earlier), see [github.com/TuringWorks/vibecody/releases](https://github.com/TuringWorks/vibecody/releases).
