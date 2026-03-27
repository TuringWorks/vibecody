# Mobile Dispatch Gateway

Remote management of VibeCody CLI/UI sessions from iOS and Android devices. Similar to Claude's dispatch feature and OpenClaw gateway.

## Architecture

```
┌─────────────────┐        ┌──────────────────┐        ┌─────────────────┐
│  iOS/Android App │◄──────►│  Bridge Relay     │◄──────►│ VibeCody Daemon │
│  (Flutter)       │  WSS   │  (cloud/self-     │  WSS   │ (machine agent) │
│                  │        │   hosted)         │        │                 │
└─────────────────┘        └──────────────────┘        └─────────────────┘
```

## Machine Registration

Machines running `vibecli --serve` register themselves with the Mobile Gateway:

```bash
# Start daemon and register for mobile access
vibecli --serve --host 0.0.0.0 --port 7878
# The daemon prints an API token and pairing QR code

# In REPL mode
/dispatch register 7878        # Register this machine
/dispatch machines             # List all registered machines
/dispatch pair <machine_id>    # Generate pairing QR/PIN for mobile
```

## Device Pairing

Three pairing methods:
1. **QR Code** — Scan from terminal using mobile camera
2. **6-Digit PIN** — Enter manually on mobile
3. **Tailscale** — Auto-pair via tailnet mesh VPN

## Dispatch Types

From the mobile app, dispatch tasks to any paired machine:

| Type | Description | Example |
|------|-------------|---------|
| `chat` | Chat with the AI agent | "What's the status of the auth refactor?" |
| `agent_task` | Start autonomous coding task | "Fix the failing test in payment_test.rs" |
| `command` | Run shell command | "cargo build --release" |
| `repl_command` | Run REPL slash-command | "/status" |
| `file_op` | File operations | "list:/src" |
| `git_op` | Git operations | "status" |
| `cancel` | Cancel a running task | Session ID |

## REST API Endpoints

All under `/mobile/*` prefix, require Bearer token auth:

```
POST   /mobile/machines             — Register a machine
GET    /mobile/machines             — List all machines
GET    /mobile/machines/:id         — Get machine details
DELETE /mobile/machines/:id         — Unregister
POST   /mobile/machines/:id/heartbeat — Send heartbeat + metrics

POST   /mobile/pairing              — Create pairing request (QR/PIN)
POST   /mobile/pairing/:id/accept   — Accept pairing from mobile
POST   /mobile/pairing/:id/verify   — Verify 6-digit PIN
POST   /mobile/pairing/:id/reject   — Reject pairing

GET    /mobile/devices              — List paired devices
POST   /mobile/devices/:id/push-token — Update push token

POST   /mobile/dispatch             — Dispatch task to machine
GET    /mobile/dispatch/:id         — Get dispatch status
POST   /mobile/dispatch/:id/cancel  — Cancel dispatch
POST   /mobile/dispatch/:id/update  — Update dispatch status

GET    /mobile/stats                — Gateway statistics
GET    /mobile/notifications/:id    — Get pending notifications
```

## Push Notifications

The gateway queues push notifications for:
- Task completion or failure
- Approval required (agent needs permission)
- Machine goes offline/online
- Security alerts

Supports APNs (iOS), FCM (Android), and WebPush.

## Mobile App (Flutter)

The `vibemobile/` directory contains a Flutter app for iOS and Android:

### Screens
- **Onboarding** — QR scan, PIN entry, or manual connection
- **Machines** — List all paired machines with status
- **Machine Detail** — System info, sessions, dispatches, quick actions
- **Chat** — Real-time chat with streaming responses
- **Sessions** — View all agent sessions across machines
- **Settings** — Manage connections, push tokens, device info

### Key Features
- Secure credential storage (Keychain/Keystore)
- QR code scanning for instant pairing
- SSE streaming for real-time agent responses
- Multi-machine management from one device
- Offline queueing with sync on reconnect
- Dark theme matching VibeCody desktop

## REPL Commands

```
/dispatch register [port]      — Register this machine (default 7878)
/dispatch unregister <id>      — Unregister a machine
/dispatch machines             — List registered machines
/dispatch pair <machine_id>    — Create pairing QR/PIN for mobile
/dispatch unpair <dev> <mac>   — Unpair device from machine
/dispatch devices              — List paired mobile devices
/dispatch send <id> <msg>      — Send a dispatch to machine
/dispatch cancel <task_id>     — Cancel a dispatch
/dispatch status               — Health check
/dispatch stats                — Show gateway statistics
/dispatch heartbeat <id>       — Trigger heartbeat for machine
```

## Security

- Bearer token authentication on all endpoints
- AES-256-GCM encrypted pairing tokens
- 10-minute pairing request TTL
- Per-device rate limiting
- Command blocklist for dangerous operations
- Tailscale integration for zero-trust networking
