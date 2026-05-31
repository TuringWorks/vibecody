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

## v0.5.7 — Latest

**Released:** May 29, 2026 &middot; [Release notes](https://github.com/TuringWorks/vibecody/releases/tag/v0.5.7) &middot; [Changelog](https://github.com/TuringWorks/vibecody/compare/v0.5.6...v0.5.7)

Release-engineering patch — restores the iOS `.ipa`, watchOS `.app.zip`, Wear OS APK/AAB, Docker tarball, and CycloneDX SBOM artifacts that didn't build for v0.5.6. No application-level feature changes — same surface as v0.5.6 with the full release matrix back.

### Bug fixes

- **CycloneDX SBOM** — `cyclonedx-py requirements` takes a positional file path, not `-i FILE`; fix the workflow invocation so `vibe-rl-py.cdx.json` is produced again ([#28](https://github.com/TuringWorks/vibecody/issues/28))
- **Mobile · iOS** — `AppDelegate.swift` referenced `FlutterImplicitEngineDelegate` / `FlutterImplicitEngineBridge` (Flutter 3.38+ UIScene APIs) while the CI Flutter SDK is pinned to 3.29.3; rewrite to the 3.29-compatible `GeneratedPluginRegistrant.register(with: self)` pattern so the unsigned `.ipa` builds again ([#29](https://github.com/TuringWorks/vibecody/issues/29))
- **Watch · watchOS** — `GoalsView.swift`, `JobPickerView.swift`, `RecapView.swift`, and `TaintedConfirmationView.swift` existed on disk but were never registered in `VibeCodyWatch.xcodeproj`'s Sources build phase; the watchOS simulator app build failed with four `cannot find … in scope` errors. Add them ([#30](https://github.com/TuringWorks/vibecody/issues/30))
- **Watch · Wear OS** — `JobRecapTileService` / `GoalsTileService` import `CallbackToFutureAdapter` + Guava `Futures`, and `RecapScreen` uses `@Preview`; declare `guava` (33.4.0-android), `androidx.concurrent:concurrent-futures` (1.2.0), and `androidx.compose.ui:ui-tooling-preview` (1.7.6) so `:app:compileReleaseKotlin` succeeds ([#31](https://github.com/TuringWorks/vibecody/issues/31))
- **Docker** — Dockerfile's two-phase cargo cache fell behind the workspace; add COPY + stub-creation for 7 workspace members added since March (`vibecli/crates/vibe-sandbox{,-native,-firecracker,-hyperlight}`, `vibecli/crates/vibe-broker`, `vibeui/crates/vibe-infer`, `vibe-memory`), and copy the real `vibe-memory/src/` over the stub during the source phase ([#32](https://github.com/TuringWorks/vibecody/issues/32))
- **docs/release.md, vibemobile.md, watchos.md, wearos.md** — fix asset names so the download links resolve; surface the new `aarch64` AppImage and `arm64` deb artifacts that landed in v0.5.6

### VibeCLI — Terminal AI Assistant

| Platform | Download |
|----------|----------|
| macOS (Apple Silicon) | [vibecli-aarch64-apple-darwin.tar.gz](https://github.com/TuringWorks/vibecody/releases/download/v0.5.7/vibecli-aarch64-apple-darwin.tar.gz) |
| macOS (Intel) | [vibecli-x86_64-apple-darwin.tar.gz](https://github.com/TuringWorks/vibecody/releases/download/v0.5.7/vibecli-x86_64-apple-darwin.tar.gz) |
| Linux x86_64 (musl) | [vibecli-x86_64-linux.tar.gz](https://github.com/TuringWorks/vibecody/releases/download/v0.5.7/vibecli-x86_64-linux.tar.gz) |
| Linux ARM64 (musl) | [vibecli-aarch64-linux.tar.gz](https://github.com/TuringWorks/vibecody/releases/download/v0.5.7/vibecli-aarch64-linux.tar.gz) |
| Windows x64 | [vibecli-x86_64-windows.zip](https://github.com/TuringWorks/vibecody/releases/download/v0.5.7/vibecli-x86_64-windows.zip) |
| Docker | [vibecli-docker-v0.5.7.tar.gz](https://github.com/TuringWorks/vibecody/releases/download/v0.5.7/vibecli-docker-v0.5.7.tar.gz) |

### VibeUI — Desktop Code Editor

| Platform | Download |
|----------|----------|
| macOS (Apple Silicon) | [VibeUI_0.5.7_aarch64.dmg](https://github.com/TuringWorks/vibecody/releases/download/v0.5.7/VibeUI_0.5.7_aarch64.dmg) |
| macOS (Intel) | [VibeUI_0.5.7_x64.dmg](https://github.com/TuringWorks/vibecody/releases/download/v0.5.7/VibeUI_0.5.7_x64.dmg) |
| macOS (Apple Silicon, .app) | [VibeUI-macOS-arm64.app.zip](https://github.com/TuringWorks/vibecody/releases/download/v0.5.7/VibeUI-macOS-arm64.app.zip) |
| macOS (Intel, .app) | [VibeUI-macOS-x64.app.zip](https://github.com/TuringWorks/vibecody/releases/download/v0.5.7/VibeUI-macOS-x64.app.zip) |
| Linux x64 (.deb) | [VibeUI_0.5.7_amd64.deb](https://github.com/TuringWorks/vibecody/releases/download/v0.5.7/VibeUI_0.5.7_amd64.deb) |
| Linux arm64 (.deb) | [VibeUI_0.5.7_arm64.deb](https://github.com/TuringWorks/vibecody/releases/download/v0.5.7/VibeUI_0.5.7_arm64.deb) |
| Linux x64 (.AppImage) | [VibeUI_0.5.7_amd64.AppImage](https://github.com/TuringWorks/vibecody/releases/download/v0.5.7/VibeUI_0.5.7_amd64.AppImage) |
| Linux arm64 (.AppImage) | [VibeUI_0.5.7_aarch64.AppImage](https://github.com/TuringWorks/vibecody/releases/download/v0.5.7/VibeUI_0.5.7_aarch64.AppImage) |
| Windows x64 (.msi) | [VibeUI_0.5.7_x64_en-US.msi](https://github.com/TuringWorks/vibecody/releases/download/v0.5.7/VibeUI_0.5.7_x64_en-US.msi) |
| Windows x64 (.exe) | [VibeUI_0.5.7_x64-setup.exe](https://github.com/TuringWorks/vibecody/releases/download/v0.5.7/VibeUI_0.5.7_x64-setup.exe) |

### VibeCLI App — Desktop AI Assistant

Tauri bundles ship as `Vibe.App_*` (productName "Vibe App").

| Platform | Download |
|----------|----------|
| macOS (Apple Silicon) | [Vibe.App_0.5.7_aarch64.dmg](https://github.com/TuringWorks/vibecody/releases/download/v0.5.7/Vibe.App_0.5.7_aarch64.dmg) |
| macOS (Intel) | [Vibe.App_0.5.7_x64.dmg](https://github.com/TuringWorks/vibecody/releases/download/v0.5.7/Vibe.App_0.5.7_x64.dmg) |
| Linux x64 (.deb) | [Vibe.App_0.5.7_amd64.deb](https://github.com/TuringWorks/vibecody/releases/download/v0.5.7/Vibe.App_0.5.7_amd64.deb) |
| Linux arm64 (.deb) | [Vibe.App_0.5.7_arm64.deb](https://github.com/TuringWorks/vibecody/releases/download/v0.5.7/Vibe.App_0.5.7_arm64.deb) |
| Linux x64 (.AppImage) | [Vibe.App_0.5.7_amd64.AppImage](https://github.com/TuringWorks/vibecody/releases/download/v0.5.7/Vibe.App_0.5.7_amd64.AppImage) |
| Linux arm64 (.AppImage) | [Vibe.App_0.5.7_aarch64.AppImage](https://github.com/TuringWorks/vibecody/releases/download/v0.5.7/Vibe.App_0.5.7_aarch64.AppImage) |
| Windows x64 (.msi) | [Vibe.App_0.5.7_x64_en-US.msi](https://github.com/TuringWorks/vibecody/releases/download/v0.5.7/Vibe.App_0.5.7_x64_en-US.msi) |
| Windows x64 (.exe) | [Vibe.App_0.5.7_x64-setup.exe](https://github.com/TuringWorks/vibecody/releases/download/v0.5.7/Vibe.App_0.5.7_x64-setup.exe) |

### VibeMobile — Flutter Companion

| Platform | Download |
|----------|----------|
| iOS (unsigned `.ipa` — sideload via AltStore/Sideloadly) | [VibeCody-Mobile-v0.5.7-ios.ipa](https://github.com/TuringWorks/vibecody/releases/download/v0.5.7/VibeCody-Mobile-v0.5.7-ios.ipa) |
| Android (`.apk`) | [VibeCody-Mobile-v0.5.7-android.apk](https://github.com/TuringWorks/vibecody/releases/download/v0.5.7/VibeCody-Mobile-v0.5.7-android.apk) |
| Android (`.aab`) | [VibeCody-Mobile-v0.5.7-android.aab](https://github.com/TuringWorks/vibecody/releases/download/v0.5.7/VibeCody-Mobile-v0.5.7-android.aab) |

### VibeWatch — Apple Watch & Wear OS

| Platform | Download |
|----------|----------|
| watchOS 10+ (unsigned `.app.zip` — sideload via Xcode) | [VibeCody-WatchOS-v0.5.7.app.zip](https://github.com/TuringWorks/vibecody/releases/download/v0.5.7/VibeCody-WatchOS-v0.5.7.app.zip) |
| Wear OS 3+ (`.apk`) | [VibeCody-Wear-v0.5.7.apk](https://github.com/TuringWorks/vibecody/releases/download/v0.5.7/VibeCody-Wear-v0.5.7.apk) |
| Wear OS 3+ (`.aab`) | [VibeCody-Wear-v0.5.7.aab](https://github.com/TuringWorks/vibecody/releases/download/v0.5.7/VibeCody-Wear-v0.5.7.aab) |

Install the companion desktop/phone app first — pair the watch from the **Watch Devices** panel in VibeUI (`Governance → Watch Devices`) or the Machine detail screen in VibeMobile.

[SHA256SUMS.txt](https://github.com/TuringWorks/vibecody/releases/download/v0.5.7/SHA256SUMS.txt)

### macOS install: first-launch warning

VibeUI and Vibe App for macOS ship **ad-hoc signed by default** (until Apple Developer credentials are added to CI — see [macOS code signing setup](#macos-code-signing-setup-for-maintainers) below). Ad-hoc signing is enough to avoid the "is damaged and can't be opened" Gatekeeper error, but the first launch still shows an **"unidentified developer"** dialog.

Two options:

1. **Right-click → Open** (one-time): in Finder, right-click the app icon, choose **Open**, then click **Open** again in the dialog. The app launches and is whitelisted from then on.
2. **Strip the quarantine xattr** from the terminal (one-time):
   ```bash
   xattr -dr com.apple.quarantine /Applications/VibeUI.app
   xattr -dr com.apple.quarantine "/Applications/Vibe App.app"
   ```

If you see *"is damaged and can't be opened"* (not "from an unidentified developer"), the DMG download was corrupted — re-download and verify against [SHA256SUMS.txt](https://github.com/TuringWorks/vibecody/releases/download/v0.5.7/SHA256SUMS.txt).

### macOS code signing setup (for maintainers)

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

The `build-vibeui` and `build-vibeapp` jobs auto-detect these secrets — when `APPLE_CERT_P12_BASE64` is unset, the build emits a workflow `::notice::` and falls back to ad-hoc signing. The watchOS-signed track uses a parallel set of secrets (`APPLE_PROVISIONING_PROFILE_BASE64` + App Store Connect API key) — see `.github/workflows/release.yml` `build-watchos-signed` for that path.

---

## v0.5.6

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

> **Note:** the iOS `.ipa`, watchOS `.app.zip`, Wear OS `.apk`/`.aab`, Docker tarball, and CycloneDX SBOMs did not build for v0.5.6 — use [v0.5.7](#v057--latest) where they're restored. Tracking issues (closed in v0.5.7): [#28](https://github.com/TuringWorks/vibecody/issues/28), [#29](https://github.com/TuringWorks/vibecody/issues/29), [#30](https://github.com/TuringWorks/vibecody/issues/30), [#31](https://github.com/TuringWorks/vibecody/issues/31), [#32](https://github.com/TuringWorks/vibecody/issues/32).

### VibeCLI — Terminal AI Assistant

| Platform | Download |
|----------|----------|
| macOS (Apple Silicon) | [vibecli-aarch64-apple-darwin.tar.gz](https://github.com/TuringWorks/vibecody/releases/download/v0.5.6/vibecli-aarch64-apple-darwin.tar.gz) |
| macOS (Intel) | [vibecli-x86_64-apple-darwin.tar.gz](https://github.com/TuringWorks/vibecody/releases/download/v0.5.6/vibecli-x86_64-apple-darwin.tar.gz) |
| Linux x86_64 (musl) | [vibecli-x86_64-linux.tar.gz](https://github.com/TuringWorks/vibecody/releases/download/v0.5.6/vibecli-x86_64-linux.tar.gz) |
| Linux ARM64 (musl) | [vibecli-aarch64-linux.tar.gz](https://github.com/TuringWorks/vibecody/releases/download/v0.5.6/vibecli-aarch64-linux.tar.gz) |
| Windows x64 | [vibecli-x86_64-windows.zip](https://github.com/TuringWorks/vibecody/releases/download/v0.5.6/vibecli-x86_64-windows.zip) |

### VibeUI — Desktop Code Editor

| Platform | Download |
|----------|----------|
| macOS (Apple Silicon) | [VibeUI_0.5.6_aarch64.dmg](https://github.com/TuringWorks/vibecody/releases/download/v0.5.6/VibeUI_0.5.6_aarch64.dmg) |
| macOS (Intel) | [VibeUI_0.5.6_x64.dmg](https://github.com/TuringWorks/vibecody/releases/download/v0.5.6/VibeUI_0.5.6_x64.dmg) |
| macOS (Apple Silicon, .app) | [VibeUI-macOS-arm64.app.zip](https://github.com/TuringWorks/vibecody/releases/download/v0.5.6/VibeUI-macOS-arm64.app.zip) |
| macOS (Intel, .app) | [VibeUI-macOS-x64.app.zip](https://github.com/TuringWorks/vibecody/releases/download/v0.5.6/VibeUI-macOS-x64.app.zip) |
| Linux x64 (.deb) | [VibeUI_0.5.6_amd64.deb](https://github.com/TuringWorks/vibecody/releases/download/v0.5.6/VibeUI_0.5.6_amd64.deb) |
| Linux arm64 (.deb) | [VibeUI_0.5.6_arm64.deb](https://github.com/TuringWorks/vibecody/releases/download/v0.5.6/VibeUI_0.5.6_arm64.deb) |
| Linux x64 (.AppImage) | [VibeUI_0.5.6_amd64.AppImage](https://github.com/TuringWorks/vibecody/releases/download/v0.5.6/VibeUI_0.5.6_amd64.AppImage) |
| Linux arm64 (.AppImage) | [VibeUI_0.5.6_aarch64.AppImage](https://github.com/TuringWorks/vibecody/releases/download/v0.5.6/VibeUI_0.5.6_aarch64.AppImage) |
| Windows x64 (.msi) | [VibeUI_0.5.6_x64_en-US.msi](https://github.com/TuringWorks/vibecody/releases/download/v0.5.6/VibeUI_0.5.6_x64_en-US.msi) |
| Windows x64 (.exe) | [VibeUI_0.5.6_x64-setup.exe](https://github.com/TuringWorks/vibecody/releases/download/v0.5.6/VibeUI_0.5.6_x64-setup.exe) |

### VibeCLI App — Desktop AI Assistant

Tauri bundles ship as `Vibe.App_*` (productName "Vibe App").

| Platform | Download |
|----------|----------|
| macOS (Apple Silicon) | [Vibe.App_0.5.6_aarch64.dmg](https://github.com/TuringWorks/vibecody/releases/download/v0.5.6/Vibe.App_0.5.6_aarch64.dmg) |
| macOS (Intel) | [Vibe.App_0.5.6_x64.dmg](https://github.com/TuringWorks/vibecody/releases/download/v0.5.6/Vibe.App_0.5.6_x64.dmg) |
| Linux x64 (.deb) | [Vibe.App_0.5.6_amd64.deb](https://github.com/TuringWorks/vibecody/releases/download/v0.5.6/Vibe.App_0.5.6_amd64.deb) |
| Linux arm64 (.deb) | [Vibe.App_0.5.6_arm64.deb](https://github.com/TuringWorks/vibecody/releases/download/v0.5.6/Vibe.App_0.5.6_arm64.deb) |
| Linux x64 (.AppImage) | [Vibe.App_0.5.6_amd64.AppImage](https://github.com/TuringWorks/vibecody/releases/download/v0.5.6/Vibe.App_0.5.6_amd64.AppImage) |
| Linux arm64 (.AppImage) | [Vibe.App_0.5.6_aarch64.AppImage](https://github.com/TuringWorks/vibecody/releases/download/v0.5.6/Vibe.App_0.5.6_aarch64.AppImage) |
| Windows x64 (.msi) | [Vibe.App_0.5.6_x64_en-US.msi](https://github.com/TuringWorks/vibecody/releases/download/v0.5.6/Vibe.App_0.5.6_x64_en-US.msi) |
| Windows x64 (.exe) | [Vibe.App_0.5.6_x64-setup.exe](https://github.com/TuringWorks/vibecody/releases/download/v0.5.6/Vibe.App_0.5.6_x64-setup.exe) |

### VibeMobile — Flutter Companion

| Platform | Download |
|----------|----------|
| Android (`.apk`) | [VibeCody-Mobile-v0.5.6-android.apk](https://github.com/TuringWorks/vibecody/releases/download/v0.5.6/VibeCody-Mobile-v0.5.6-android.apk) |
| Android (`.aab`) | [VibeCody-Mobile-v0.5.6-android.aab](https://github.com/TuringWorks/vibecody/releases/download/v0.5.6/VibeCody-Mobile-v0.5.6-android.aab) |

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

Tauri bundles ship as `Vibe.App_*` (productName "Vibe App").

| Platform | Download |
|----------|----------|
| macOS (Apple Silicon) | [Vibe.App_0.5.5_aarch64.dmg](https://github.com/TuringWorks/vibecody/releases/download/v0.5.5/Vibe.App_0.5.5_aarch64.dmg) |
| macOS (Intel) | [Vibe.App_0.5.5_x64.dmg](https://github.com/TuringWorks/vibecody/releases/download/v0.5.5/Vibe.App_0.5.5_x64.dmg) |
| Linux x64 (.deb) | [Vibe.App_0.5.5_amd64.deb](https://github.com/TuringWorks/vibecody/releases/download/v0.5.5/Vibe.App_0.5.5_amd64.deb) |
| Linux x64 (.AppImage) | [Vibe.App_0.5.5_amd64.AppImage](https://github.com/TuringWorks/vibecody/releases/download/v0.5.5/Vibe.App_0.5.5_amd64.AppImage) |
| Windows x64 (.msi) | [Vibe.App_0.5.5_x64_en-US.msi](https://github.com/TuringWorks/vibecody/releases/download/v0.5.5/Vibe.App_0.5.5_x64_en-US.msi) |
| Windows x64 (.exe) | [Vibe.App_0.5.5_x64-setup.exe](https://github.com/TuringWorks/vibecody/releases/download/v0.5.5/Vibe.App_0.5.5_x64-setup.exe) |

### VibeMobile — Flutter Companion

| Platform | Download |
|----------|----------|
| iOS (unsigned `.ipa` — sideload via AltStore/Sideloadly) | [VibeCody-Mobile-v0.5.5-ios.ipa](https://github.com/TuringWorks/vibecody/releases/download/v0.5.5/VibeCody-Mobile-v0.5.5-ios.ipa) |
| Android (`.apk`) | [VibeCody-Mobile-v0.5.5-android.apk](https://github.com/TuringWorks/vibecody/releases/download/v0.5.5/VibeCody-Mobile-v0.5.5-android.apk) |
| Android (`.aab`) | [VibeCody-Mobile-v0.5.5-android.aab](https://github.com/TuringWorks/vibecody/releases/download/v0.5.5/VibeCody-Mobile-v0.5.5-android.aab) |

### VibeWatch — Apple Watch & Wear OS *(new in v0.5.5)*

| Platform | Download |
|----------|----------|
| watchOS 10+ (unsigned `.app.zip` — sideload via Xcode) | [VibeCody-WatchOS-v0.5.5.app.zip](https://github.com/TuringWorks/vibecody/releases/download/v0.5.5/VibeCody-WatchOS-v0.5.5.app.zip) |
| Wear OS 3+ (`.apk`) | [VibeCody-Wear-v0.5.5.apk](https://github.com/TuringWorks/vibecody/releases/download/v0.5.5/VibeCody-Wear-v0.5.5.apk) |
| Wear OS 3+ (`.aab`) | [VibeCody-Wear-v0.5.5.aab](https://github.com/TuringWorks/vibecody/releases/download/v0.5.5/VibeCody-Wear-v0.5.5.aab) |

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
