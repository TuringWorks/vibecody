---
layout: default
title: Watch Integration (watchOS + Wear OS)
nav_order: 25
---

# Watch Integration — Apple Watch & Wear OS

VibeCody extends its AI coding assistant to wrist-worn devices, giving developers a lightweight session monitor and voice dispatch capability while away from the desktop.

## Architecture Overview

```txt
┌─────────────────────────────────────────────────────────────────┐
│  vibecli daemon  (vibecli --serve --port 7878)                  │
│                                                                 │
│  ┌─────────────────┐  ┌──────────────────┐  ┌───────────────┐   │
│  │  watch_auth.rs  │  │ watch_session_   │  │ watch_bridge  │   │
│  │  HMAC-SHA256    │  │ relay.rs         │  │ .rs           │   │
│  │  JWT lifecycle  │  │ Compact payloads │  │ Axum /watch/* │   │
│  │  Ed25519 reg    │  │ OLED-optimised   │  │ SSE streaming │   │
│  └────────┬────────┘  └────────┬─────────┘  └──────┬────────┘   │
│           │                    │                   │            │
└───────────┼────────────────────┼───────────────────┼─-──────────┘
            │                    │                   │
     ┌──────▼──────┐      ┌──────▼──────┐      ┌─────▼──────┐
     │  LAN / TLS  │      │  LAN / TLS  │      │  SSE feed  │
     └──────┬──────┘      └──────┬──────┘      └─────┬──────┘
            │                    │                   │
   ┌────────▼────────────────────▼───────────────────▼──---────┐
   │          Transport fallback chain                         │
   │  1. Direct LAN (Wi-Fi, same subnet)                       │
   │  2. Tailscale mesh (cross-network)                        │
   │  3. Phone relay (WatchConnectivity on iOS /               │
   │     Wearable Data Layer on Android)                       │
   └──────────┬──────────────────────────────┬─────────────-───┘
              │                              │
   ┌──────────▼──────────┐      ┌────────────▼─────────────-─┐
   │  Apple Watch        │      │  Android Wear OS           │
   │  WatchOS 9+         │      │  Wear OS 3+                │
   │  VibeCody watchApp  │      │  VibeCodyWear app          │
   │  WatchConnectivity  │      │  Wearable Data Layer API   │
   │  Secure Enclave key │      │  Android Keystore P-256    │
   └─────────────────────┘      └──────────────────────────-─┘
```

---

## Rust Modules

### `watch_auth.rs` — Authentication & Device Registration

Provides challenge-response registration and JWT-based session security.

**Key types:**

| Type | Description |
|------|-------------|
| `WatchAuthManager` | Main entry point; holds machine ID and JWT secret |
| `RegistrationChallenge` | Single-use 32-char hex nonce with 5-minute TTL |
| `WatchRegisterRequest` | Device public key + Ed25519 signature over challenge |
| `WatchDevice` | Registered device record (ID, platform, public key, revocation) |
| `WatchClaims` | JWT payload: `sub` (device ID), `machine_id`, `kind`, `exp` |
| `WristActivityEvent` | On-wrist / off-wrist events with Ed25519 signature |
| `NonceRegistry` (relay) | Replay-prevention map with 30-second timestamp window |

**Token lifecycle:**

```text
  [Watch] → GET /watch/challenge
  [Daemon] ← { nonce, machine_id, issued_at, expires_at }

  [Watch] → POST /watch/register { device_id, platform, public_key_b64, nonce, signature_b64 }
  [Daemon] verifies Ed25519(public_key, nonce_bytes) == signature
  [Daemon] ← { access_token (15 min), refresh_token (7 days), device_id }

  [Watch] → GET /watch/sessions  (Authorization: Bearer <access_token>)
  ...
  [Watch] → POST /watch/refresh  (Authorization: Bearer <refresh_token>)
  [Daemon] ← { access_token, refresh_token }
```

**Security properties:**

- JWT signed with HMAC-SHA256 (32-byte secret stored in `ProfileStore`)
- Ed25519 device key pair — private key never leaves the device
  - Apple Watch: Secure Enclave (P-256 bridged via CryptoKit)
  - Wear OS: Android Keystore with StrongBox/TEE backing
- Wrist-suspension lock: suspended sessions block tool execution
- Replay prevention: nonces are single-use within a 5-minute window

---

### `watch_session_relay.rs` — Compact Payloads

Transforms full session models into OLED-optimised representations.

**Key types:**

| Type | Description |
|------|-------------|
| `WatchSessionSummary` | Session ID, status, model, step count, last message preview (≤80 chars), message count |
| `WatchMessage` | Role, content (≤512 chars with `…` truncation), timestamp |
| `WatchAgentEvent` | Streaming SSE event: `kind`, `delta`, `tool`, `step`, `status`, `error` |
| `WatchSandboxStatus` | CPU %, memory MB, disk MB, running flag |
| `NonceRegistry` | Thread-safe replay-prevention map (Arc<Mutex>) |

**Helper functions:**

```rust
pub fn truncate(s: &str, max_chars: usize) -> String
pub fn to_watch_event_json(payload: &serde_json::Value) -> WatchAgentEvent
pub fn to_watch_message(row: &MessageRowView<'_>) -> WatchMessage
pub fn to_watch_summary(session: &SessionRowView<'_>, messages: &[MessageRowView<'_>]) -> WatchSessionSummary
```

**SSE event mapping:**

| SSE `type` field | `WatchAgentEvent.kind` | Extra fields |
|------------------|----------------------|--------------|
| `token_delta` | `delta` | `delta` = text |
| `tool_start` | `tool_start` | `tool` = name, `step` = step number |
| `tool_end` | `tool_end` | `tool` = name, `status` = `"ok"` / `"err"` |
| `done` | `done` | `status` = status string |
| `error` | `error` | `error` = message (≤200 chars) |
| _(anything else)_ | `info` | — |

---

### `watch_bridge.rs` — Axum Router

Standalone Axum state and 11 HTTP routes under `/watch/*`.

**State:**

```rust
pub struct WatchBridgeState {
    pub streams: WatchEventStreams,           // session_id → BroadcastSender
    pub api_token: Option<String>,           // bearer token for auth
    pub auth_manager: Arc<Mutex<WatchAuthManager>>,
    pub nonce_registry: NonceRegistry,
}
pub type WatchEventStreams = Arc<Mutex<HashMap<String, broadcast::Sender<serde_json::Value>>>>;
```

**Routes:**

| Method | Path | Description |
|--------|------|-------------|
| `GET` | `/watch/health` | Unauthenticated health check |
| `GET` | `/watch/challenge` | Issue registration challenge nonce |
| `POST` | `/watch/register` | Register device (Ed25519 key exchange) |
| `POST` | `/watch/refresh` | Refresh expired access token |
| `GET` | `/watch/sessions` | List active sessions (auth required) |
| `GET` | `/watch/sessions/:id` | Session detail |
| `GET` | `/watch/sessions/:id/messages` | Message history |
| `POST` | `/watch/dispatch` | Send message to active session |
| `GET` | `/watch/stream/:session_id` | SSE stream of agent events |
| `POST` | `/watch/wrist-event` | Wrist on/off event (session lock) |
| `POST` | `/watch/sandbox/:id/control` | Pause / resume / stop sandbox |

---

## Platform Clients

### Apple Watch (`vibewatch/VibeCodyWatch/`)

- **Language**: Swift / SwiftUI
- **Auth**: CryptoKit P-256 (Secure Enclave), stored in Keychain
- **Transport**: WatchConnectivity framework for phone relay; direct HTTP over LAN
- **Screens**: Session list → Conversation → Voice input → Sandbox status → Settings

### Wear OS (`vibewatch/VibeCodyWear/`)

- **Language**: Kotlin / Jetpack Compose for Wear
- **Auth**: Android Keystore P-256 with StrongBox check; EncryptedSharedPreferences for token storage
- **Transport**: OkHttp SSE client for direct connections; Wearable Data Layer API for offline relay via companion phone
- **Voice**: `SpeechRecognizer` with `EXTRA_PREFER_OFFLINE=true` (audio never leaves device)
- **Key files**:
  - `WearAuthManager.kt` — Keystore key pair, signature building, token refresh
  - `WearNetworkManager.kt` — SSE streaming, token validation, phone relay fallback
  - `WearDataLayerClient.kt` — Wearable Data Layer message sending
  - `WearDataLayerService.kt` (companion app) — `WearableListenerService` bridging watch to daemon

---

## VibeUI Panel

The **Watch Devices** panel (`vibeui/src/components/WatchManagementPanel.tsx`) provides:

- Platform badge: "watchOS" (accent) or "Wear OS" (green) based on device model heuristic
- Per-device: status dot, platform badge, device ID, last-seen timestamp
- QR code pairing modal (generates `/watch/challenge` → deep-link URL)
- Security info: Secure Enclave (iOS) / StrongBox TEE (Android)
- Transport info: Phone relay via WatchConnectivity (iOS) / Data Layer (Android)

Navigation: **Governance → Watch Devices**

---

## TDD Coverage

### `watch_auth.rs` (inline `#[cfg(test)]`)

| Test | What it verifies |
|------|-----------------|
| `ttl_matches_constant` | Access/refresh TTL constants match JWT `exp` |
| `machine_id_bound_to_manager` | `for_testing()` sets correct machine ID |
| `wrong_secret_rejected` | Different secret fails JWT decode |
| `challenge_window_is_nonce_ttl` | `expires_at - issued_at == NONCE_TTL_SECS` |
| `register_request_serde_roundtrip` | JSON round-trip preserves all fields |
| `wrist_suspended_serialises` | `wrist_suspended` field present in JSON |
| `revoked_at_is_none_by_default` | Newly registered device has no revocation |
| `ed25519_wrong_length_rejected` | Invalid sig bytes return error |
| `stale_wrist_event_rejected` | Timestamp > 30s old fails |
| `max_watch_devices_is_positive` | `MAX_WATCH_DEVICES > 0` |
| `watch_claims_serde_roundtrip` | `WatchClaims` JSON round-trip |

### `watch_session_relay.rs` (inline)

| Test | What it verifies |
|------|-----------------|
| `truncate_short_string_unchanged` | Short strings pass through unchanged |
| `truncate_adds_ellipsis` | Long strings get `…` suffix |
| `truncate_exact_length_unchanged` | Exact-length strings are not truncated |
| `tool_start_event` | `tool_start` maps name and step |
| `tool_end_success_event` | `tool_end` with success maps to `"ok"` |
| `tool_end_failure_event` | `tool_end` without success maps to `"err"` |
| `done_event` | `done` maps status field |
| `error_event_truncated` | Error messages capped at 200 chars with `…` |
| `unknown_event_type_defaults_to_info` | Unrecognised types map to `"info"` |
| `kind_field_in_event` | All events have a `kind` field |
| `session_summary_message_count` | Summary counts all messages |
| `session_summary_last_activity` | Summary uses latest message timestamp |
| `session_summary_empty_messages` | Summary handles empty message list |
| `nonce_registry_distinct_nonces_accepted` | Multiple distinct nonces all accepted |
| `watch_sandbox_status_serde` | `WatchSandboxStatus` JSON round-trip |
| `watch_agent_event_serde` | `WatchAgentEvent` JSON round-trip |

### `watch_bridge.rs` (inline)

| Test | What it verifies |
|------|-----------------|
| `watch_dispatch_response_streaming_url_contains_session_id` | URL has session ID |
| `watch_event_streams_new_is_empty` | Fresh map has no entries |
| `watch_event_streams_accepts_broadcaster` | Sender insertion works |
| `watch_sandbox_control_request_serde` | `WatchSandboxControlRequest` round-trip |
| `watch_bridge_state_size_of_does_not_panic` | State type is sized |
| `nonce_replay_rejected_in_bridge_context` | Replay prevention in bridge |
| `watch_dispatch_request_without_session_id` | `None` session_id preserved |
| `watch_dispatch_request_with_session_id` | `Some` session_id preserved |

---

## BDD Coverage

### `tests/features/watch_auth.feature` — 10 scenarios / 38 steps

- Challenge nonce is 32-char hex
- Challenge nonce is consumed on use (single-use)
- Access token embeds device ID and machine ID
- Expired token is rejected
- Tampered signature is rejected
- Refresh token has correct kind field
- Token signed with wrong secret is rejected
- Wrist event with stale timestamp is rejected
- Ed25519 signature with wrong length is rejected
- WatchDevice serialises round-trip through JSON

### `tests/features/watch_session_relay.feature` — 15 scenarios / 57 steps

- Short/long/exact-length truncation
- SSE delta, tool_start, tool_end (success/failure), done, error, unknown events
- Nonce replay rejection, multiple nonces, stale timestamp
- Session summary preview and message count
- Message content capping at 512 characters

### `tests/features/watch_bridge.feature` — 10 scenarios / 35 steps

- Streaming URL format validation
- WatchEventStreams empty on init / accepts senders
- Nonce replay rejection, distinct nonces accepted
- SandboxControlRequest serialisation
- Dispatch request null / existing session_id
- WatchBridgeState sizing
- WatchDispatchResponse field serialisation

**Total BDD: 35 scenarios, 130 steps — all green**

---

## Security Considerations

| Threat | Mitigation |
|--------|-----------|
| Stolen bearer token | Short 15-min access TTL; refresh token stored in Keychain/EncryptedSharedPreferences |
| Token replay | Per-session nonce registry with 30-second window |
| Man-in-the-middle | TLS required for all non-LAN transports; Tailscale adds mutual auth |
| Rogue device pairing | Challenge nonce expires in 5 minutes; requires Ed25519 signature over nonce |
| Wrist lift = session access | `wrist_suspended` flag blocks tool execution when watch is off wrist |
| Voice audio exfiltration | `EXTRA_PREFER_OFFLINE=true` on Wear OS; voice processed on-device |
| Session token in Data Layer | Companion phone uses its own bearer token; never sends watch token to daemon |

---

## Configuration

Add to `~/.vibecli/config.toml`:

```toml
[watch]
enabled = true           # default: true when --serve is active
port = 7878              # shared with HTTP daemon
require_tls = false      # set true in production
max_devices = 10         # per machine_id
session_lock_on_suspend = true   # block tool calls when off wrist
```
