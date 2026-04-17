---
layout: page
title: VibeWatch — Wear OS
permalink: /wearos/
---

**VibeCodyWear** is the native Wear OS client for VibeCody. Built with Kotlin and Jetpack Compose for Wear OS 3+, it delivers the same feature set as the Apple Watch client against the same `/watch/*` backend — so you can mix & match.

Introduced in **v0.5.5**.

---

## What you can do on the watch

- **Stream live AI sessions** from your paired desktop.
- **Reply by voice** using the built-in Wear OS recognizer, or tap a template.
- **Approve / reject** pending agent actions (shell commands, file writes).
- **Switch between sessions** from chat, agent, review, and sandbox.
- **Pair with URL + Bearer** — no QR or JSON copy required; works on the Android Wear emulator.

---

## Requirements

| Component | Minimum |
|-----------|---------|
| Watch | Wear OS 3 device (Pixel Watch, Galaxy Watch 4+, TicWatch Pro 5, …) |
| Wear OS | 3.5+ (API 30+) runtime — latest tested: **Wear OS 6** on Android 16 |
| Phone | Android 10+ with VibeMobile installed and paired |
| Desktop | VibeCLI or VibeUI ≥ 0.5.5 running `--serve` |
| ADB (for sideload) | latest |

> **Build SDK** — `compileSdk = 36` and `targetSdk = 36` (Android 16 / Wear OS 6) as of v0.5.5; `minSdk = 30` preserves Wear OS 3 compatibility. AGP 8.7.3, Kotlin 2.1.0, JDK 17.

---

## Install

### Option 1 — sideload the release APK

Download from the [Releases page](/vibecody/release/):

```bash
curl -LO https://github.com/TuringWorks/vibecody/releases/download/v0.5.5/VibeCodyWear-wearos.apk
adb connect <watch-ip>:5555          # enable ADB debugging first
adb -s <watch-ip>:5555 install VibeCodyWear-wearos.apk
```

Enable ADB debugging on the watch:
**Settings → Developer options → ADB debugging** + **Debug over Wi-Fi**.

### Option 2 — Play Store AAB

Upload `VibeCodyWear-wearos.aab` to the Play Console's internal-testing track. Once approved, install directly from the watch's Play Store.

### Option 3 — build from source

```bash
# Linux, macOS, or Windows:
make watch-wear            # Release APK
make watch-wear-bundle     # Release AAB

# Or drive Gradle directly:
cd vibewatch/VibeCodyWear
./gradlew :app:assembleRelease
```

Output: `vibewatch/VibeCodyWear/app/build/outputs/apk/release/app-release.apk`

---

## Pair in 3 taps

Pairing is brokered through VibeMobile on your Android phone — the watch inherits the session.

1. **Phone (VibeMobile):** pair with the desktop ([VibeMobile pairing](/vibecody/vibemobile/#pair-with-vibecody-in-30-seconds)).
2. **Phone:** on the paired-machine row → **⋯ → Add Wear OS Watch**.
3. **Watch:** open **VibeCody** → tap **Accept**.

The watch generates a **P-256 ECDSA** keypair in the Android Keystore (StrongBox-backed on devices that support it, e.g. Pixel Watch 2+), signs the challenge, and receives a 30-day JWT.

### Emulator / dev-watch path (URL + Bearer)

```bash
vibecli> /pair --show-bearer --for-watch
```

On the watch: **Settings → Manual Pair → Paste URL + Bearer**. This bypasses the phone entirely and is how the Android Wear emulator pairs.

---

## The screens

### 1. Sessions list (rotary-scroll)

Turn the rotary bezel / side button to scroll. Each row:

```
 ● refactor axum routes          08:42
 ○ review PR #42                 08:30
 ● agent: make test              08:11
```

Tap → opens the transcript. Long-press → session menu (Pull to phone, Close, Cancel step).

### 2. Transcript view

Full message content — **no 80/512-char truncation** as of 0.5.5. Text reflows to the screen shape (round or square); code blocks get a subtle mono font and horizontal-scroll affordance.

### 3. Reply sheet

- **Voice** — tap the mic. Uses Google's on-device recognition (offline on Pixel Watch / Galaxy Watch Ultra).
- **Templates** — "Yes, proceed", "Show diff", "Cancel", + 3 custom slots.
- **Keyboard** — the standard Wear OS IME; slow but works.

### 4. Approvals

A haptic nudge + a full-screen modal when the desktop agent needs approval:

```
 ⚠ Run: cargo check
 Session: refactor axum
 [ Approve ]  [ Reject ]
```

---

## Sync model — no truncation, no drift

The pre-0.5.5 watch protocol used a fixed-size ring buffer (80 chars for labels, 512 for message bodies). 0.5.5 replaces this with a **Google-Docs-style ID-based reconciliation**: each message has a stable ID; the watch subscribes to a stream of `{id, seq, delta}` events and resolves its local buffer against the canonical transcript. Misses are caught up automatically on reconnect.

The practical effect: your watch, your phone, and your desktop show identical transcripts, regardless of who's typing.

---

## Data Layer relay

On Android, the watch can reach the desktop in two ways:

1. **Direct** — when the watch has Wi-Fi and can see the host (mDNS / Tailscale / ngrok), it talks HTTP/2 directly.
2. **Phone relay** — when the watch has no route (e.g., LTE-only watch out of Wi-Fi range), traffic is tunneled through the phone via the Wearable Data Layer.

VibeCodyWear detects the best path automatically. You can force one from **Settings → Connection → Prefer direct / phone-relayed**.

---

## Battery tips

- The SSE stream is cheap; the typical drain is **1–2 % per 10 minutes of active streaming**.
- Keep the transcript screen open only while actively watching — VibeCodyWear falls back to push-only mode when backgrounded.
- Voice dictation is the biggest drain; batch replies instead of one-offs.

---

## Troubleshooting

### ADB install fails with `INSTALL_FAILED_NO_MATCHING_ABIS`

Wear OS is ARM64. Make sure you grabbed the `wearos.apk` (not the phone APK) and that your watch is Wear OS 3+.

### Watch says "No paired machine"

- On the phone, open VibeMobile → **Machines** and verify the target is listed.
- On the desktop, `/watch devices` in VibeCLI. Revoke and re-pair if missing.

### Voice capture returns empty

- Grant **Microphone** permission to VibeCodyWear (first-launch prompt).
- Confirm **Google Assistant** is the default assistant (required by the recognizer).

### Pairing hangs at "Signing challenge"

- StrongBox / Keystore lockout — reboot the watch and retry.
- If you're on the emulator, use the **URL + Bearer** flow instead.

### Stream disconnects at the 2-minute mark

Some carrier NATs drop idle connections. VibeCodyWear auto-reconnects; the Google-Docs-style sync will catch up without gaps. If it happens constantly, enable Tailscale on the watch and host.

---

## Security notes

- **P-256 private key** lives in Android Keystore. On devices with StrongBox (Pixel Watch 2+, Galaxy Watch 6+), it's stored in dedicated tamper-resistant hardware.
- **JWT** is kept in `EncryptedSharedPreferences`, backed by the Keystore.
- **Revocation** — `/watch revoke <device-id>` on the desktop, or **Governance → Watch Devices** in VibeUI. Takes effect on the next request.

See [Watch Integration](/vibecody/watch-integration/) for the full architecture and protocol reference.

---

## Related

- [Apple Watch guide](/vibecody/watchos/) — same client, watchOS
- [VibeMobile](/vibecody/vibemobile/) — the Android phone app that brokers pairing
- [Connectivity](/vibecody/connectivity/) — mDNS / Tailscale / ngrok paths
- [Releases](/vibecody/release/) — download artifacts
