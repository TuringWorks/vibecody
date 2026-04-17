---
layout: page
title: VibeMobile — Flutter Companion
permalink: /vibemobile/
---

**VibeMobile** is the Flutter companion for VibeCody — a full phone/tablet/desktop client that pairs with a running VibeCLI or VibeUI instance, streams AI sessions in real time, and hands the session back and forth between devices.

As of **v0.5.5** VibeMobile gained URL-only pairing, zero-config mDNS / Tailscale / ngrok discovery, Apple-Handoff-style continuity with desktop and watch clients, and Google-Docs-style full-content sync (no more 80/512-char truncation).

> **Platforms:** iOS, Android, macOS, Linux, Windows, Web — one Flutter codebase.

---

## What's new in 0.5.5

| Area | Improvement |
|------|-------------|
| Pairing | **URL-only / URL + Bearer** pairing — no QR code or JSON copy required; works on emulators |
| Auth | **P-256 ECDSA** (Keystore / StrongBox / Secure Enclave) replaces Ed25519 |
| Discovery | **mDNS LAN** (`_vibecli._tcp.local.`), **Tailscale Funnel**, **ngrok** auto-detect — client races all reachable paths |
| Continuity | **Handoff banner** auto-appears when desktop or watch opens a session |
| Sync | **Google-Docs-style** full-content reconciliation with ID-based dedup — no truncation |
| Session tree | Sandbox tab auto-surfaces when a paired watch starts a sandbox session |

---

## Platform requirements

| Target | Minimum |
|--------|---------|
| iOS | **13.0+** (deployment target raised from 12.0 in v0.5.5) |
| Android | 7.0 / API 24+ |
| macOS | 12.0+ |
| Linux / Windows | GTK 3 / Edge WebView2 |
| Build toolchain | Flutter 3.29.3 (CI-pinned, floor ≥ 3.2.0), Xcode 26 (App Store submissions after 2026-04-28) |

---

## Install

### From release artifacts (fastest)

See the [Releases page](/vibecody/release/) for the current build. For v0.5.5:

| Platform | Artifact |
|----------|----------|
| iOS | [`VibeMobile-iOS.ipa`](https://github.com/TuringWorks/vibecody/releases/download/v0.5.5/VibeMobile-iOS.ipa) (unsigned — sideload via AltStore / Sideloadly) |
| Android APK | [`VibeMobile-android.apk`](https://github.com/TuringWorks/vibecody/releases/download/v0.5.5/VibeMobile-android.apk) |
| Android AAB | [`VibeMobile-android.aab`](https://github.com/TuringWorks/vibecody/releases/download/v0.5.5/VibeMobile-android.aab) |

### From source

```bash
# Prerequisites: Flutter ≥ 3.2.0, Xcode (iOS/macOS), Android Studio + SDK (Android)
cd vibemobile
flutter pub get
flutter run            # launches on the currently-selected device

# Release builds
make -C vibemobile ios-ipa         # signed .ipa (needs APPLE_TEAM_ID)
flutter build apk --release        # Android APK
flutter build appbundle --release  # Android AAB for Play Store
flutter build macos --release      # macOS .app
flutter build linux --release      # Linux bundle
flutter build windows --release    # Windows .exe
flutter build web --release        # static web bundle
```

---

## Pair with VibeCody in 30 seconds

Pairing produces a short-lived challenge, the phone signs it with its Keystore/Secure-Enclave P-256 key, and the daemon returns a JWT.

### Path A — QR code (desktop nearby)

1. **Desktop:** start the daemon.
   ```bash
   vibecli --serve --port 7879
   ```
2. **Desktop:** in the REPL or VibeUI `Governance → Watch Devices`, click **Pair Device**.
3. **Phone:** open VibeMobile → **Pair** → scan the QR code. Done.

### Path B — URL only (new in 0.5.5, works on emulators)

1. **Desktop:** run `/pair --url-only` in VibeCLI (or click **Show URL** in VibeUI).
2. **Phone:** VibeMobile → **Pair** → **Paste URL** → confirm.

The URL encodes `host:port` + a one-time bearer token. No clipboard JSON is needed.

### Path C — URL + Bearer (manual, air-gapped)

1. **Desktop:** `/pair --show-bearer` prints two fields.
2. **Phone:** VibeMobile → **Pair** → **Manual** → paste URL and bearer.

All three paths produce the same JWT. Sessions authenticate with `Authorization: Bearer …` afterwards.

---

## Home tabs

| Tab | Purpose |
|-----|---------|
| **Machines** | All paired desktops/laptops. Health, CPU/memory, active provider/model, session count |
| **Sessions** | Live agent + chat sessions across all machines; tap to stream |
| **Sandbox** | Auto-focused when a paired watch or desktop opens a sandbox session |
| **Chat** | Direct remote chat with any provider configured on the paired host |
| **Settings** | Pairing, notifications, mDNS toggle, Tailscale / ngrok preferences |

The **Handoff banner** appears at the top of any tab when another paired device is active in the same session — tap to pull the session onto the phone.

---

## Connectivity paths

VibeMobile races all reachable paths on every request and picks the fastest one:

1. **mDNS LAN** — discovers `_vibecli._tcp.local.` on any IP range; no setup.
2. **Tailscale** — if the phone is on your tailnet, the 100.x address is used; if the host has **Funnel** enabled, a public HTTPS URL is shared.
3. **ngrok** — auto-detected if already running on the host; opt-in auto-start requires an auth token in VibeCLI settings.
4. **Manual host:port** — always available as a fallback under **Settings → Manual Connection**.

See the full [Connectivity guide](/vibecody/connectivity/) for firewall / NAT / corporate-network notes.

---

## Chat & sessions

- **Streaming.** Every message streams token-by-token over Server-Sent Events. Reconnects automatically on flaky networks.
- **No truncation.** The 0.5.5 sync model reconciles by message ID and keeps the full transcript — even long code blocks survive the round trip.
- **Markdown.** Fenced code blocks render with syntax highlighting; copy button per block.
- **Voice.** Tap the microphone on the chat input to dictate (uses on-device speech → Groq Whisper if the host has it configured).
- **Provider picker.** Switch providers or models mid-conversation from the session header.

---

## Handoff continuity

When you're actively using a session on another paired device (desktop, watch, or another phone), VibeMobile surfaces a Handoff chip at the top of the current screen:

```
↻ ravi's iMac — Session #1287 · "refactor axum routes"
   [ Pull session ]  [ Mirror ]  [ Dismiss ]
```

- **Pull** — transfers stream ownership. The other device goes read-only.
- **Mirror** — keeps both devices writing; the Google-Docs-style sync reconciles edits.
- **Dismiss** — ignores, but the chip reappears if the other device is still active.

---

## Secure storage

Credentials never touch plaintext on disk:

| Platform | Backing store |
|----------|---------------|
| iOS | Keychain Services (kSecClassGenericPassword, access group per-app) |
| Android | EncryptedSharedPreferences, key material in Android Keystore / StrongBox where available |
| macOS | Keychain |
| Linux | Secret Service (`libsecret`) |
| Windows | DPAPI via `flutter_secure_storage` |
| Web | IndexedDB + WebCrypto; clears on cache wipe |

---

## Notifications

| Trigger | Default | How to change |
|---------|---------|---------------|
| Agent task complete | On | Settings → Notifications |
| Security alert (policy violation) | On | — |
| Machine goes offline | On | — |
| New message from another device (Handoff) | Off | Toggle on if you want push-style Handoff |

iOS notifications require granting permission on first launch. Android 13+ requires POST_NOTIFICATIONS runtime permission.

---

## Troubleshooting

### Pairing QR code won't scan

- Dim your display, then try again — QR codes need contrast.
- Switch to **Path B (URL only)** — paste the URL instead.
- Check the phone can reach `host:port` (try a browser).

### mDNS discovery doesn't find the host

- Some corporate networks block multicast. Fall back to manual host:port.
- Verify `vibecli --serve` is running and bound to `0.0.0.0` (not `127.0.0.1`).

### "Session desynced" banner

This is the reconciliation catch-up signaling a missed event. Tap **Reload** — the full transcript is fetched and merged.

### Wear OS / watchOS continuity not working

The watch must be paired to the *same host* as the phone. Check **Governance → Watch Devices** on the desktop.

---

## API endpoints

All calls require `Authorization: Bearer <jwt>` after pairing. See [VibeCLI Server Mode](/vibecody/vibecli/#server-mode-vibecli-serve) for the full reference.

| Endpoint | Method | Purpose |
|----------|--------|---------|
| `/health` | GET | Liveness |
| `/status` | GET | Active provider/model |
| `/pair/challenge` | POST | Issue pairing nonce |
| `/pair/confirm` | POST | Submit signed attestation, receive JWT |
| `/mobile/beacon` | POST | Heartbeat + presence for Handoff routing |
| `/sessions` | GET | List sessions across all machines |
| `/sessions/{id}/stream` | GET (SSE) | Stream messages |
| `/sessions/{id}/reply` | POST | Append a user reply |
| `/chat` | POST | One-shot chat |

---

## Related reading

- [VibeCLI reference](/vibecody/vibecli/) — full REPL + server mode
- [Connectivity guide](/vibecody/connectivity/) — mDNS, Tailscale, ngrok
- [Watch Integration](/vibecody/watch-integration/) — how VibeMobile relays watch sessions
- [Apple Watch guide](/vibecody/watchos/) · [Wear OS guide](/vibecody/wearos/)
