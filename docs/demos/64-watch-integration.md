---
layout: page
title: "Demo 64: Apple Watch & Wear OS Integration"
permalink: /demos/64-watch-integration/
nav_order: 64
parent: Demos
---

## Overview

Control VibeCody agent sessions from your wrist. The watch bridge provides secure, low-latency session relay between Apple Watch (watchOS) and Wear OS companions and the VibeCLI daemon running on your Mac or Linux machine.

---

## Quick Start

```bash
# Start the daemon with watch bridge enabled (default port 7860)
vibecli --serve

# Pair your Apple Watch (watchOS companion app)
vibecli --watch pair --platform apple

# Pair a Wear OS device
vibecli --watch pair --platform wearos

# List paired devices
vibecli --watch devices

# Check wrist-suspension status
vibecli --watch status
```

---

## Apple Watch Companion

The watchOS companion (VibeCody Watch) communicates over WatchConnectivity / HTTP/2:

```
┌────────────────────┐        HMAC-SHA256 JWT        ┌──────────────────────┐
│  Apple Watch       │ ─────────────────────────────► │  VibeCLI Daemon      │
│  watchOS companion │ ◄───────────────────────────── │  /watch/* endpoints  │
│  (SwiftUI)         │        SSE event stream        │  port 7860           │
└────────────────────┘                                └──────────────────────┘
```

**Wrist capabilities:**
- Start / stop / pause agent sessions
- Receive streaming output digest (last N tokens)
- Approve or reject tool-use confirmations
- View cost ticker (tokens used, estimated $)
- Wrist-suspension lock: raising the watch resumes; lowering suspends

---

## Wear OS Companion

The Kotlin companion (`VibeCodyWear`) uses the [Wearable Data Layer API](https://developer.android.com/training/wearables/data/data-layer):

```bash
# Android companion auto-pairs via WearDataLayerService
# Manual trigger from the Wear OS device settings:
adb -s <wear-device-id> shell am start \
  -n com.vibecody.wear/.MainActivity
```

**Wear OS features:**
- Tile showing active session name and token count
- Complication with cost-per-minute display
- Haptic feedback on tool-use events

---

## Security Model

Authentication uses HMAC-SHA256 JWTs with per-device Ed25519 keys:

```bash
# Inspect the device key registry
cat ~/.vibecli/watch-devices.json

# Rotate a device key (invalidates existing tokens)
vibecli --watch rotate --device <device-id>

# Revoke a device entirely
vibecli --watch revoke --device <device-id>
```

Token lifetimes:
- **Access token**: 15 minutes (auto-refreshed by companion)
- **Refresh token**: 7 days (requires re-pair after expiry)
- **Nonce registry**: replay protection (64-entry LRU window)

---

## Wrist Suspension

When the watch is taken off (wrist sensor reports `on_wrist = false`), active sessions are automatically suspended:

```
[watch] Wrist event received: on_wrist=false, device=ABC123
[watch] Session "my-task" → SUSPENDED (wrist-lock)
[watch] Session resumes automatically when watch is back on wrist
```

Override via:
```bash
vibecli --watch unlock --session my-task
```

---

## HTTP API (daemon)

The watch bridge exposes REST endpoints under `/watch/`:

| Method | Path | Description |
|--------|------|-------------|
| `POST` | `/watch/auth/pair` | Register new device + issue JWT |
| `POST` | `/watch/auth/refresh` | Refresh access token |
| `GET`  | `/watch/session/stream` | SSE stream of session events |
| `POST` | `/watch/session/control` | Start / stop / pause |
| `POST` | `/watch/wrist-event` | Report wrist-on/off state |
| `GET`  | `/watch/devices` | List registered devices |
| `DELETE` | `/watch/devices/{id}` | Revoke a device |

---

## Rust Modules

| Module | Purpose |
|--------|---------|
| `watch_auth` | JWT generation/validation, Ed25519 device keys, nonce registry |
| `watch_session_relay` | SSE broadcast, session control, event buffering |
| `watch_bridge` | HTTP route handlers, daemon integration |

---

## BDD Coverage

```gherkin
Feature: Watch Auth
  Scenario: Pair new Apple Watch device
  Scenario: Reject expired JWT
  Scenario: Replay attack blocked by nonce registry
  Scenario: Wrist suspension locks active session
  ...10 scenarios total

Feature: Watch Session Relay
  Scenario: Stream session events to paired watch
  Scenario: Control message pauses running agent
  Scenario: Resume on wrist-on event
  ...15 scenarios total

Feature: Watch Bridge
  Scenario: Full pair → stream → control flow
  Scenario: Multi-device concurrent streams
  ...10 scenarios total
```

Run the BDD suites:

```bash
cargo test --test watch_auth_bdd
cargo test --test watch_session_relay_bdd
cargo test --test watch_bridge_bdd
```

---

## Related Demos

- [55 — Voice, Pairing & Tailscale](../55-voice-pairing-tailscale/) — QR pairing and Tailscale Funnel
- [56 — Browser Web Client](../56-web-client/) — Zero-install SPA
- [65 — Zero-Config Connectivity](../65-connectivity/) — mDNS + ngrok
