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

## v0.5.8 — Latest

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
| macOS (Apple Silicon) | [vibecli-aarch64-apple-darwin.tar.gz](https://github.com/TuringWorks/vibecody/releases/download/v0.5.8/vibecli-aarch64-apple-darwin.tar.gz) |
| macOS (Intel) | [vibecli-x86_64-apple-darwin.tar.gz](https://github.com/TuringWorks/vibecody/releases/download/v0.5.8/vibecli-x86_64-apple-darwin.tar.gz) |
| Linux x86_64 (musl) | [vibecli-x86_64-linux.tar.gz](https://github.com/TuringWorks/vibecody/releases/download/v0.5.8/vibecli-x86_64-linux.tar.gz) |
| Linux ARM64 (musl) | [vibecli-aarch64-linux.tar.gz](https://github.com/TuringWorks/vibecody/releases/download/v0.5.8/vibecli-aarch64-linux.tar.gz) |
| Windows x64 | [vibecli-x86_64-windows.zip](https://github.com/TuringWorks/vibecody/releases/download/v0.5.8/vibecli-x86_64-windows.zip) |
| Docker | [vibecli-docker-v0.5.8.tar.gz](https://github.com/TuringWorks/vibecody/releases/download/v0.5.8/vibecli-docker-v0.5.8.tar.gz) |

### VibeCoder — Desktop Code Editor

| Platform | Download |
|----------|----------|
| macOS (Apple Silicon) | [VibeCoder_0.5.8_aarch64.dmg](https://github.com/TuringWorks/vibecody/releases/download/v0.5.8/VibeCoder_0.5.8_aarch64.dmg) |
| macOS (Intel) | [VibeCoder_0.5.8_x64.dmg](https://github.com/TuringWorks/vibecody/releases/download/v0.5.8/VibeCoder_0.5.8_x64.dmg) |
| macOS (Apple Silicon, .app) | [VibeCoder-macOS-arm64.app.zip](https://github.com/TuringWorks/vibecody/releases/download/v0.5.8/VibeCoder-macOS-arm64.app.zip) |
| macOS (Intel, .app) | [VibeCoder-macOS-x64.app.zip](https://github.com/TuringWorks/vibecody/releases/download/v0.5.8/VibeCoder-macOS-x64.app.zip) |
| Linux x64 (.deb) | [VibeCoder_0.5.8_amd64.deb](https://github.com/TuringWorks/vibecody/releases/download/v0.5.8/VibeCoder_0.5.8_amd64.deb) |
| Linux arm64 (.deb) | [VibeCoder_0.5.8_arm64.deb](https://github.com/TuringWorks/vibecody/releases/download/v0.5.8/VibeCoder_0.5.8_arm64.deb) |
| Linux x64 (.AppImage) | [VibeCoder_0.5.8_amd64.AppImage](https://github.com/TuringWorks/vibecody/releases/download/v0.5.8/VibeCoder_0.5.8_amd64.AppImage) |
| Linux arm64 (.AppImage) | [VibeCoder_0.5.8_aarch64.AppImage](https://github.com/TuringWorks/vibecody/releases/download/v0.5.8/VibeCoder_0.5.8_aarch64.AppImage) |
| Windows x64 (.msi) | [VibeCoder_0.5.8_x64_en-US.msi](https://github.com/TuringWorks/vibecody/releases/download/v0.5.8/VibeCoder_0.5.8_x64_en-US.msi) |
| Windows x64 (.exe) | [VibeCoder_0.5.8_x64-setup.exe](https://github.com/TuringWorks/vibecody/releases/download/v0.5.8/VibeCoder_0.5.8_x64-setup.exe) |

### VibeAIChat — Desktop AI Assistant

Tauri bundles ship as `VibeAIChat_*` (productName "VibeAIChat").

| Platform | Download |
|----------|----------|
| macOS (Apple Silicon) | [VibeAIChat_0.5.8_aarch64.dmg](https://github.com/TuringWorks/vibecody/releases/download/v0.5.8/VibeAIChat_0.5.8_aarch64.dmg) |
| macOS (Intel) | [VibeAIChat_0.5.8_x64.dmg](https://github.com/TuringWorks/vibecody/releases/download/v0.5.8/VibeAIChat_0.5.8_x64.dmg) |
| Linux x64 (.deb) | [VibeAIChat_0.5.8_amd64.deb](https://github.com/TuringWorks/vibecody/releases/download/v0.5.8/VibeAIChat_0.5.8_amd64.deb) |
| Linux arm64 (.deb) | [VibeAIChat_0.5.8_arm64.deb](https://github.com/TuringWorks/vibecody/releases/download/v0.5.8/VibeAIChat_0.5.8_arm64.deb) |
| Linux x64 (.AppImage) | [VibeAIChat_0.5.8_amd64.AppImage](https://github.com/TuringWorks/vibecody/releases/download/v0.5.8/VibeAIChat_0.5.8_amd64.AppImage) |
| Linux arm64 (.AppImage) | [VibeAIChat_0.5.8_aarch64.AppImage](https://github.com/TuringWorks/vibecody/releases/download/v0.5.8/VibeAIChat_0.5.8_aarch64.AppImage) |
| Windows x64 (.msi) | [VibeAIChat_0.5.8_x64_en-US.msi](https://github.com/TuringWorks/vibecody/releases/download/v0.5.8/VibeAIChat_0.5.8_x64_en-US.msi) |
| Windows x64 (.exe) | [VibeAIChat_0.5.8_x64-setup.exe](https://github.com/TuringWorks/vibecody/releases/download/v0.5.8/VibeAIChat_0.5.8_x64-setup.exe) |

### VibeDesk — Desktop Task Shell

New in this release.

| Platform | Download |
|----------|----------|
| macOS (Apple Silicon) | [VibeDesk_0.5.8_aarch64.dmg](https://github.com/TuringWorks/vibecody/releases/download/v0.5.8/VibeDesk_0.5.8_aarch64.dmg) |
| macOS (Intel) | [VibeDesk_0.5.8_x64.dmg](https://github.com/TuringWorks/vibecody/releases/download/v0.5.8/VibeDesk_0.5.8_x64.dmg) |
| Linux x64 (.deb) | [VibeDesk_0.5.8_amd64.deb](https://github.com/TuringWorks/vibecody/releases/download/v0.5.8/VibeDesk_0.5.8_amd64.deb) |
| Linux arm64 (.deb) | [VibeDesk_0.5.8_arm64.deb](https://github.com/TuringWorks/vibecody/releases/download/v0.5.8/VibeDesk_0.5.8_arm64.deb) |
| Linux x64 (.AppImage) | [VibeDesk_0.5.8_amd64.AppImage](https://github.com/TuringWorks/vibecody/releases/download/v0.5.8/VibeDesk_0.5.8_amd64.AppImage) |
| Linux arm64 (.AppImage) | [VibeDesk_0.5.8_aarch64.AppImage](https://github.com/TuringWorks/vibecody/releases/download/v0.5.8/VibeDesk_0.5.8_aarch64.AppImage) |
| Windows x64 (.msi) | [VibeDesk_0.5.8_x64_en-US.msi](https://github.com/TuringWorks/vibecody/releases/download/v0.5.8/VibeDesk_0.5.8_x64_en-US.msi) |
| Windows x64 (.exe) | [VibeDesk_0.5.8_x64-setup.exe](https://github.com/TuringWorks/vibecody/releases/download/v0.5.8/VibeDesk_0.5.8_x64-setup.exe) |

### VibeMobile — Flutter Companion

| Platform | Download |
|----------|----------|
| iOS (unsigned `.ipa` — sideload via AltStore/Sideloadly) | [VibeCody-Mobile-v0.5.8-ios.ipa](https://github.com/TuringWorks/vibecody/releases/download/v0.5.8/VibeCody-Mobile-v0.5.8-ios.ipa) |
| Android (`.apk`) | [VibeCody-Mobile-v0.5.8-android.apk](https://github.com/TuringWorks/vibecody/releases/download/v0.5.8/VibeCody-Mobile-v0.5.8-android.apk) |
| Android (`.aab`) | [VibeCody-Mobile-v0.5.8-android.aab](https://github.com/TuringWorks/vibecody/releases/download/v0.5.8/VibeCody-Mobile-v0.5.8-android.aab) |

### VibeWatch — Apple Watch & Wear OS

| Platform | Download |
|----------|----------|
| watchOS 10+ (unsigned `.app.zip` — sideload via Xcode) | [VibeCody-WatchOS-v0.5.8.app.zip](https://github.com/TuringWorks/vibecody/releases/download/v0.5.8/VibeCody-WatchOS-v0.5.8.app.zip) |
| Wear OS 3+ (`.apk`) | [VibeCody-Wear-v0.5.8.apk](https://github.com/TuringWorks/vibecody/releases/download/v0.5.8/VibeCody-Wear-v0.5.8.apk) |
| Wear OS 3+ (`.aab`) | [VibeCody-Wear-v0.5.8.aab](https://github.com/TuringWorks/vibecody/releases/download/v0.5.8/VibeCody-Wear-v0.5.8.aab) |

Install the companion desktop/phone app first — pair the watch from the **Watch Devices** panel in VibeCoder (`Governance → Watch Devices`) or the Machine detail screen in VibeMobile.

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

## Installing on macOS

Applies to every release; not specific to the version above.

### macOS install: first-launch warning

From v0.5.8 the macOS desktop artifacts *can be* **Developer ID signed** (v0.5.7 and earlier were always ad-hoc) — see [Code signing](#code-signing) to check yours. Signing alone does not remove the first-launch prompt — that needs notarization as well, so an **"unidentified developer"** dialog on a signed-but-not-notarized build is expected. See [Code signing](#code-signing) to check which you have.

Two options:

1. **Right-click → Open** (one-time): in Finder, right-click the app icon, choose **Open**, then click **Open** again in the dialog. The app launches and is whitelisted from then on.
2. **Strip the quarantine xattr** from the terminal (one-time):
   ```bash
   xattr -dr com.apple.quarantine /Applications/VibeCoder.app
   xattr -dr com.apple.quarantine "/Applications/VibeAIChat.app"
   xattr -dr com.apple.quarantine /Applications/VibeDesk.app
   ```

If you see *"is damaged and can't be opened"* (not "from an unidentified developer"), the DMG download was corrupted — re-download and verify against [SHA256SUMS.txt](https://github.com/TuringWorks/vibecody/releases/download/v0.5.8/SHA256SUMS.txt).

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

For older releases (v0.5.7 and earlier), see [github.com/TuringWorks/vibecody/releases](https://github.com/TuringWorks/vibecody/releases).
