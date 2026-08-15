# VibeCody v0.5.9 Release

**64 commits since v0.5.8.**

---

## What's in v0.5.9

An evaluation harness that refuses to report a result it did not measure, a
plugin marketplace where a workspace can actually install something, connectors
as first-class MCP integrations with encrypted credentials, and a run of
agent-reliability work that came out of watching real runs stall and then claim
success.

Much of what follows is not a new feature going in but an old one that never
worked: panels calling commands nobody had registered, a connector list that
reported "connected" without a credential, a transform that scanned the app's
working directory instead of your project. Each is now covered by a test that
fails when it comes back.

### Headline features

- **`vibecli eval` — an evaluation harness (`crates/vibe-eval`).** Coding,
  agentic tool use, knowledge work, safety, and per-surface transport
  conformance across all fourteen clients. Four verdicts are kept strictly
  apart — `pass`, `fail`, `error` (the harness could not decide) and `skipped`
  (did not apply) — with errors and skips outside the pass-rate denominator,
  because "the agent regressed" and "python3 isn't installed" are different
  sentences. A grader with no assertions is an error, not a pass, rejected at
  load time. `make eval-check` validates the suites with no provider and no
  agent; `vibecli --eval gate latest --baseline <run-id>` exits 1 on regression.

- **A plugin marketplace, and connectors.** The Plugins panel used to read "No
  plugin components are enabled for this workspace" above a pointer to a CLI
  command — true, and useless, because the only way to get a plugin was to
  author, sign and pack an MCPB bundle by hand. Eleven core plugins and
  seventeen connectors now ship inside the binary, installable in one click,
  offline. Connector credentials are encrypted in the workspace store and are
  never returned by any route.

- **Bundles.** A plugin can install other plugins and set up the connectors a
  job assumes — Engineering, On-call, Security review, Data work, Research. A
  bundle adds every connector that needs no credential and reports the rest as
  `needs_credentials` with the field names. It never reports one as configured
  that you have not supplied a token for.

- **Connector health is measured, never inferred.** Nothing says a connector
  works until **Test** has launched the server and listed its tools. No probe
  result is persisted: a stored "ok" is a claim about the past presented as a
  claim about now.

- **`/goal <what you want>`** states a goal and works on it, and a VibeCoder
  panel shows what a goal is doing with failures kept on screen.

### Notable fixes

- **The agent could report work it had not done.** `--exec` now double-checks
  with the project's own build and test before accepting `task_complete`,
  bounded so a check that can never pass cannot spin the run to death. A check
  that fails to spawn is reported as unverified rather than mapped onto pass —
  the verification step had contained the exact fallback it existed to catch.

- **A run could not be bounded from inside its own loops.** Every guard was
  checked between turns or between chunks, so each depended on some inner loop
  coming back round; four separate stalls slipped past. Runs are now bounded
  from outside, with elapsed-time walls for "nothing has changed on disk" and
  "no tool has run at all".

- **Secrets could leave in the agent's own words.** Asked to summarise a
  `.env`, it reproduced a database password verbatim — and paraphrased it once
  output redaction was added. Credential files are redacted on the way *in*, so
  the model never receives the value.

- **An autonomous run cannot remove an authorization guard.** Told to make a
  failing test pass against a test asserting anonymous access should succeed,
  the agent deleted the auth check in 2 of 3 sampled runs. Guard-bearing files
  are restored unless the task explicitly asked for that file.

- **27 Tauri commands VibeCoder panels were already calling did not exist**, so
  those buttons did nothing. **Eight panels ignored the toolbar's provider.**
  **Code Transforms scanned three different roots**, one of them the app's own
  working directory. **VibeCoder's Connectors panel was a facade.** All four are
  now covered by contract tests over the joins that `tsc` and `rustc` cannot
  see.

- Python MCP connectors were all broken against the current SDK and the reason
  was thrown away — `McpClient` opened the server's stderr as `/dev/null`, so a
  traceback surfaced as "EOF while parsing". Both fixed.


## Downloads

### VibeCLI — Terminal AI Assistant

| Platform | File |
|----------|------|
| macOS (Apple Silicon) | `vibecli-aarch64-apple-darwin.tar.gz` |
| macOS (Intel) | `vibecli-x86_64-apple-darwin.tar.gz` |
| Linux x86_64 (static musl) | `vibecli-x86_64-linux.tar.gz` |
| Linux ARM64 (static musl) | `vibecli-aarch64-linux.tar.gz` |
| Windows x64 | `vibecli-x86_64-windows.zip` |
| Docker | `vibecli-docker-v0.5.9.tar.gz` |

### VibeCoder — Desktop Code Editor

| Platform | File |
|----------|------|
| macOS (Apple Silicon) | `VibeCoder_0.5.9_aarch64.dmg` |
| macOS (Intel) | `VibeCoder_0.5.9_x64.dmg` |
| macOS (`.app`) | `VibeCoder-macOS-{arm64,x64}.app.zip` |
| Linux x64 / arm64 (`.deb`) | `VibeCoder_0.5.9_{amd64,arm64}.deb` |
| Linux x64 / arm64 (`.AppImage`) | `VibeCoder_0.5.9_{amd64,aarch64}.AppImage` |
| Windows x64 | `VibeCoder_0.5.9_x64_en-US.msi` · `VibeCoder_0.5.9_x64-setup.exe` |

### VibeAIChat — Desktop AI Assistant

| Platform | File |
|----------|------|
| macOS (Apple Silicon) | `VibeAIChat_0.5.9_aarch64.dmg` |
| macOS (Intel) | `VibeAIChat_0.5.9_x64.dmg` |
| Linux x64 / arm64 (`.deb`) | `VibeAIChat_0.5.9_{amd64,arm64}.deb` |
| Linux x64 / arm64 (`.AppImage`) | `VibeAIChat_0.5.9_{amd64,aarch64}.AppImage` |
| Windows x64 | `VibeAIChat_0.5.9_x64_en-US.msi` · `VibeAIChat_0.5.9_x64-setup.exe` |

### VibeDesk — Desktop Task Shell

| Platform | File |
|----------|------|
| macOS (Apple Silicon) | `VibeDesk_0.5.9_aarch64.dmg` |
| macOS (Intel) | `VibeDesk_0.5.9_x64.dmg` |
| Linux x64 / arm64 (`.deb`) | `VibeDesk_0.5.9_{amd64,arm64}.deb` |
| Linux x64 / arm64 (`.AppImage`) | `VibeDesk_0.5.9_{amd64,aarch64}.AppImage` |
| Windows x64 | `VibeDesk_0.5.9_x64_en-US.msi` · `VibeDesk_0.5.9_x64-setup.exe` |

### VibeCody Mobile

| Platform | File |
|----------|------|
| iOS (unsigned `.ipa`) | `VibeCody-Mobile-v0.5.9-ios.ipa` |
| Android | `VibeCody-Mobile-v0.5.9-android.apk` · `.aab` |

### VibeCody Watch

| Platform | File |
|----------|------|
| watchOS 10+ (unsigned `.app.zip`) | `VibeCody-WatchOS-v0.5.9.app.zip` |
| Wear OS 3+ | `VibeCody-Wear-v0.5.9.apk` · `.aab` |

---

## Quick Install

```bash
# One-liner (Linux/macOS)
curl -fsSL https://raw.githubusercontent.com/TuringWorks/vibecody/main/install.sh | sh

# Docker (air-gapped / on-prem)
docker load < vibecli-docker-v0.5.9.tar.gz
docker run -p 7878:7878 vibecli:v0.5.9

# Verify
vibecli --version   # Should print: vibecli 0.5.9
```

---

## macOS code signing

Developer ID signing has covered the whole macOS surface since v0.5.8, the
`vibecli` binary included; before that it shipped with **no signature at all**
and the three app bundles were ad-hoc. Nothing about signing changed in v0.5.9.

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

### From v0.5.8

No breaking API changes. Drop-in replace the binary and restart:

```bash
curl -fsSL https://raw.githubusercontent.com/TuringWorks/vibecody/main/install.sh | sh
```

Two things to know:

1. **The daemon must be restarted for the plugin and connector routes to
   exist.** The desktop shells autostart it, but an already-running daemon from
   a previous install keeps serving the old route set — the Plugins panel then
   reports that the daemon is an older build than the app, with the command to
   fix it.
2. **Connector definitions are per workspace.** They live in
   `<workspace>/.vibecli/workspace.db` with credentials encrypted; nothing is
   read from or written to a plaintext config file, and removing a connector
   deletes its stored credentials with it.

### From v0.5.7 or earlier

See the [v0.5.8 release notes](https://github.com/TuringWorks/vibecody/releases/tag/v0.5.8)
for the intermediate delta — every entry there applies, including the semantic
index migration that discards the plaintext API key the old format stored.

---

## Full Changelog

See [docs/CHANGELOG.md](docs/CHANGELOG.md) for the complete history.
See [compare view](../../compare/v0.5.8...v0.5.9) for the v0.5.9 diff.
