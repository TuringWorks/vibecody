---
layout: page
title: VibeWatch — Apple Watch (watchOS)
permalink: /watchos/
---

**VibeCodyWatch** is the native Apple Watch client for VibeCody. Written in SwiftUI for watchOS 10+, it pairs with a running VibeCLI or VibeUI instance and gives you a glanceable session monitor + dictated reply from your wrist.

Introduced in **v0.5.5**. Shares the `/watch/*` backend with the Wear OS client.

---

## What you can do on the watch

- **See live AI sessions** streaming from your desktop in real time (no truncation).
- **Dictate a reply** with the watchOS speech recognizer or tap a quick-reply template.
- **Approve or cancel agent actions** when the desktop is in `suggest` mode.
- **Switch sessions** between chat, agent, review, and sandbox tasks.
- **Receive session notifications** on the wrist via Apple's Watch Connectivity framework.
- **Pair via URL only** — no JSON, no QR required. Works against the watchOS simulator.

---

## Requirements

| Component | Minimum |
|-----------|---------|
| watch | Apple Watch Series 6 or later (SE 2nd gen OK) |
| watchOS | 10.0+ (runtime) — latest tested: watchOS 26 |
| iPhone | iOS 17+ with VibeMobile installed and paired |
| Desktop | VibeCLI or VibeUI ≥ 0.5.5 running `--serve` |
| Xcode (to sideload) | 15+ |
| Xcode (to submit to App Store / TestFlight) | **26+** — Apple requires watchOS 26 / iOS 26 SDKs for App Store Connect submissions after **2026-04-28**. CI pins `xcode-version: ^26.0` accordingly. |

> VibeWatch requires the companion iPhone app. Pair the desktop once from VibeMobile, and the watch inherits the pairing.

---

## Install

### Option 1 — sideload the release build

Download the unsigned `.app.zip` from the [Releases page](/vibecody/release/):

```bash
curl -LO https://github.com/TuringWorks/vibecody/releases/download/v0.5.5/VibeCodyWatch-watchOS.app.zip
unzip VibeCodyWatch-watchOS.app.zip
```

In Xcode: **Window → Devices and Simulators → Apple Watch → +** → select the extracted `.app`. Trust the developer profile on the watch (**Settings → General → VPN & Device Management**).

### Option 2 — build from source

```bash
# From repo root, on macOS with Xcode 15+ (Xcode 26+ to submit to App Store):
make watch-ios          # Release build for the simulator
make watch-ios-archive  # Archive for a real device (requires signing)
```

For TestFlight / Ad Hoc distribution from CI, populate the `APPLE_TEAM_ID`, `APPLE_CERT_P12_BASE64`, `APPLE_PROVISIONING_PROFILE_BASE64` (+ optional `APPLE_ASC_*`) repo secrets — the `Watch · watchOS (signed)` release-workflow job is otherwise a no-op.

The Xcode project lives at `vibewatch/VibeCodyWatch.xcodeproj`. Open it directly if you want to run in the simulator with Cmd-R.

---

## Pair in 3 taps

Pairing is done *once, from the iPhone*. The watch inherits the JWT.

1. **iPhone (VibeMobile):** pair with the desktop (see [VibeMobile pairing](/vibecody/vibemobile/#pair-with-vibecody-in-30-seconds)).
2. **iPhone:** on the paired machine row, tap **⋯ → Add Apple Watch**.
3. **Watch:** open **VibeCody** → tap **Accept** on the pairing prompt.

Under the hood the watch generates a **P-256 ECDSA** keypair inside the Secure Enclave, signs a freshly issued nonce (`SHA-256(nonce ‖ device_id ‖ issued_at_be)`), and receives a 30-day JWT.

### Emulator / simulator path

The watchOS simulator has no Secure Enclave, so it falls back to a software-backed P-256 key — pairing still succeeds using the **URL + Bearer** method. From VibeCLI:

```bash
vibecli> /pair --show-bearer --for-watch
```

Paste both fields into the watch's **Settings → Manual Pair** screen.

---

## The screens

### 1. Sessions list

```
┌──────────────────┐
│  ● refactor axu… │   ← green dot = streaming
│  ○ review PR#42  │
│  ● agent: tests  │
│                  │
│     [ Scroll ]   │
└──────────────────┘
```

Crown-scroll through sessions from all machines the desktop advertises. The dot shows live/streaming status.

### 2. Transcript view

Tap a session to see the live transcript. Text is auto-sized to the bezel, word-wrapped, and never truncated (0.5.5 fixed the old 80/512-char cap). Turn the Digital Crown to scroll.

Force-touch (or long-press on Series 7+) opens the session context menu:

- **Reply** — dictate a response
- **Pull to phone** — Handoff to iPhone
- **Cancel step** — cancels the current agent action (if in `suggest` mode)
- **Close** — closes the session on the desktop

### 3. Reply sheet

Three reply paths, pick whichever matches the context:

- **Dictate** — tap the mic; uses Apple's on-device speech recognizer. Hit **Send** or let the auto-detect silence trigger.
- **Templates** — "Yes, proceed", "Show me the diff", "Cancel", plus 3 customizable ones you edit from VibeMobile.
- **Keyboard** — QWERTY if you like tiny taps (or use Scribble on compatible watches).

### 4. Approvals (suggest-mode agents)

When the desktop agent runs in the default `suggest` policy and proposes a shell command or file write, the watch gets a haptic tap and a modal:

```
 ⚠ Run:  cargo check
 Session: refactor axum
 [ Approve ]  [ Reject ]
```

Approve / reject answers stream back to the desktop in under 200ms.

---

## Wrist-suspend and battery

- **Auto-sleep.** When the wrist drops, the SSE stream is paused and replaced with a low-power heartbeat. Raising the wrist instantly resumes.
- **Active use.** Expect ~1% battery for a 10-minute active session. Streaming transcripts are cheap; dictation is the main drain.
- **Complication.** Add the VibeCody complication to a watch face for 1-tap launch into the last active session.

---

## Sandbox sessions and VibeUI auto-focus

If you start a **sandbox** session from the watch (agent runs in a container), VibeUI on the desktop automatically switches to the **Sandbox** tab so you can watch the container output while the watch drives. Turn this off in `vibeui/Settings → Handoff → Auto-focus sandbox tab`.

---

## Troubleshooting

### Watch says "No paired machine"

- Open VibeMobile and verify the machine is listed under **Machines**.
- On the desktop, run `/watch devices` in VibeCLI to see accepted devices. Revoke and re-pair if the watch isn't there.

### Dictation returns empty text

- Check **Settings → General → Dictation** on the watch is enabled.
- Offline dictation requires watchOS 10+ and an A-series chip (Series 6+); older watches need WiFi / phone nearby.

### Stream pauses after 30 seconds

- Raise the wrist — the SSE stream resumes. If it doesn't, toggle airplane mode briefly or tap **Reload** in the session context menu.

### "Certificate trust" error on first launch

Sideloaded `.app`s require trust from **Settings → General → VPN & Device Management** on the *phone*, then **Watch app → General → Profiles** for the watch.

---

## Security notes

- **P-256 private key** lives in the Secure Enclave and never leaves the watch.
- **JWT** is stored in the watch keychain (access group per-app, `kSecAttrAccessibleWhenUnlockedThisDeviceOnly`).
- **Revocation** is instant: `/watch revoke <device-id>` from VibeCLI, or the **Watch Devices** panel in VibeUI — the next request gets a 401 and the watch falls back to re-pair.

See [Watch Integration](/vibecody/watch-integration/) for the full pairing / signature / relay architecture.

---

## Related

- [Wear OS guide](/vibecody/wearos/) — identical feature set for Android
- [VibeMobile](/vibecody/vibemobile/) — the iPhone app that brokers pairing
- [Connectivity](/vibecody/connectivity/) — mDNS / Tailscale / ngrok
- [Releases](/vibecody/release/) — download artifacts
