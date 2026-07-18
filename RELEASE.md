# VibeCody v0.5.7 Release

**Release-engineering patch — restores the full v0.5.6 download matrix.**

---

## What's in v0.5.7

v0.5.6 shipped successfully across the critical surfaces (CLI binaries, VibeCoder, VibeCLI App, Android, Firecracker rootfs) but five optional release-workflow jobs failed before publish, so the iOS `.ipa`, watchOS `.app.zip`, Wear OS APK/AAB, Docker tarball, and CycloneDX SBOMs never landed under the v0.5.6 tag.

v0.5.7 carries no application-level changes — same surface as v0.5.6 — and fixes the five broken jobs so the full artifact set is back.

### Fixed release jobs

- **CycloneDX SBOM** (`a6d670bf`, closes [#28](https://github.com/TuringWorks/vibecody/issues/28)) — `cyclonedx-py requirements` takes the requirements path positionally, not as `-i FILE`. Drop the rejected flag so `vibe-rl-py.cdx.json` is produced.
- **Mobile · iOS** (`b8d95e0f`, closes [#29](https://github.com/TuringWorks/vibecody/issues/29)) — `AppDelegate.swift` referenced `FlutterImplicitEngineDelegate` / `FlutterImplicitEngineBridge`, both introduced in Flutter 3.38; CI is pinned to 3.29.3. Rewrite to the 3.29-compatible `GeneratedPluginRegistrant.register(with: self)` plugin-registry pattern; register the relay-credentials method channel synchronously in `didFinishLaunchingWithOptions`.
- **Watch · watchOS** (`014f5cce`, closes [#30](https://github.com/TuringWorks/vibecody/issues/30)) — `GoalsView.swift`, `JobPickerView.swift`, `RecapView.swift`, and `TaintedConfirmationView.swift` existed on disk but were never registered in `VibeCodyWatch.xcodeproj`'s Sources build phase. Add them as `PBXFileReference` + `PBXBuildFile` entries.
- **Watch · Wear OS** (`6193920a`, closes [#31](https://github.com/TuringWorks/vibecody/issues/31)) — `JobRecapTileService` / `GoalsTileService` import `androidx.concurrent.futures.CallbackToFutureAdapter` + Guava `Futures` / `ListenableFuture`, and `RecapScreen` uses `@Preview`. Declare `guava` (33.4.0-android), `androidx.concurrent:concurrent-futures` (1.2.0), and `androidx.compose.ui:ui-tooling-preview` (1.7.6).
- **Docker image** (`99d8adfe` + `f922536b`, closes [#32](https://github.com/TuringWorks/vibecody/issues/32)) — Dockerfile fell behind the workspace; seven members added since March (`vibecli/crates/vibe-sandbox{,-native,-firecracker,-hyperlight}`, `vibecli/crates/vibe-broker`, `vibecoder/crates/vibe-infer`, `vibe-memory`) lacked manifest COPY + stub-creation, and `vibe-memory/src/` was never copied over the stub in the second-stage source phase. Add the missing COPY + stub lines.

### Docs fixes

- **docs/release.md, docs/vibemobile.md, docs/watchos.md, docs/wearos.md** (`41f189eb`) — reconcile asset names with the actual workflow output (`Vibe.App_*` not `VibeCLI_*`; `VibeCody-Mobile-vX.Y.Z-{ios,android}.*` not `VibeMobile-*`; `VibeCody-WatchOS-vX.Y.Z.app.zip` and `VibeCody-Wear-vX.Y.Z.*` not the old `-watchOS` / `-wearos` names). Surface the new `aarch64.AppImage` + `arm64.deb` artifacts that landed in v0.5.6.

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
| Docker | `vibecli-docker-v0.5.7.tar.gz` |

### VibeCoder — Desktop Code Editor

| Platform | File |
|----------|------|
| macOS (Apple Silicon) | `VibeCoder_0.5.7_aarch64.dmg` |
| macOS (Intel) | `VibeCoder_0.5.7_x64.dmg` |
| Linux x64 | `.deb` / `.AppImage` |
| Linux arm64 | `.deb` / `.AppImage` |
| Windows x64 | `.msi` / `.exe` |

### VibeCLI App — Desktop AI Assistant

Tauri bundles ship as `Vibe.App_0.5.7_*` (productName "Vibe App") on every platform above.

### VibeCody Mobile

| Platform | File |
|----------|------|
| Android (APK) | `VibeCody-Mobile-v0.5.7-android.apk` |
| Android (AAB) | `VibeCody-Mobile-v0.5.7-android.aab` |
| iOS (unsigned IPA) | `VibeCody-Mobile-v0.5.7-ios.ipa` |

### VibeCody Watch

| Platform | File |
|----------|------|
| watchOS (Simulator `.app.zip`, unsigned) | `VibeCody-WatchOS-v0.5.7.app.zip` |
| Wear OS (APK) | `VibeCody-Wear-v0.5.7.apk` |
| Wear OS (AAB) | `VibeCody-Wear-v0.5.7.aab` |

---

## Quick Install

```bash
# One-liner (Linux/macOS)
curl -fsSL https://raw.githubusercontent.com/TuringWorks/vibecody/main/install.sh | sh

# Docker (air-gapped / on-prem)
docker load < vibecli-docker-v0.5.7.tar.gz
docker run -p 7878:7878 vibecli:v0.5.7

# Verify
vibecli --version   # Should print: vibecli 0.5.7
```

---

## Upgrade Guide

### From v0.5.6

No code-level breaking changes. Drop-in replace the binary and restart:

```bash
curl -fsSL https://raw.githubusercontent.com/TuringWorks/vibecody/main/install.sh | sh
```

If you were stuck on v0.5.5 waiting for the missing v0.5.6 mobile/watch/Docker artifacts, v0.5.7 is the upgrade target — it carries every v0.5.6 feature (plugin system end-to-end, `/goal` lifecycle G1–G13, sandbox tiers, security posture, phone-relay consolidation, hook-protocol parity, MCP Apps host, TurboQuant memory index, Linux arm64 Tauri builds) and adds back the missing platforms.

### From v0.5.5

See the [v0.5.6 release notes](https://github.com/TuringWorks/vibecody/releases/tag/v0.5.6) for the v0.5.5 → v0.5.6 feature delta — every entry there applies.

---

## Full Changelog

See [docs/CHANGELOG.md](docs/CHANGELOG.md) for the complete history.
See [compare view](../../compare/v0.5.6...v0.5.7) for the v0.5.7 diff.
