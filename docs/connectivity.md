---
layout: page
title: Mobile Connectivity — Handoff & Remote Access
permalink: /connectivity/
---

The VibeMobile app connects to a running `vibecli --serve` daemon. Three complementary network paths are supported, tried in priority order from fastest to most universally reachable. No single path is required — the app races all available paths and uses whichever responds first.

---

## Overview

| Path | Range | Setup required | How it works |
|------|-------|----------------|--------------|
| **mDNS/DNS-SD** | Same LAN | None — zero config | Daemon announces `_vibecli._tcp.local.` every 60 s; app queries on startup |
| **Tailscale** | Tailnet (any internet) | Install Tailscale on both devices | Daemon IP included in beacon; Funnel opt-in for public HTTPS URL |
| **ngrok** | Public internet | ngrok installed or opt-in auto-start | Daemon detects running tunnel automatically; opt-in auto-start with auth token |

---

## Path 1 — mDNS (zero config, LAN only)

The daemon broadcasts its presence over multicast DNS (RFC 6762 / DNS-SD) the moment `--serve` starts. No flags, no config file changes.

### How it works

1. `vibecli --serve` binds its TCP port (default 7878) and immediately starts the mDNS announcer.
2. The announcer sends a DNS response to `224.0.0.251:5353` containing:
   - **PTR** `_vibecli._tcp.local.` → service instance name
   - **SRV** → host, port
   - **TXT** → `machine_id=<id>` `version=<ver>`
   - **A** → LAN IPv4 address(es)
3. Announcements repeat every 60 seconds. A listener also answers active PTR queries from the mobile app (< 1 s discovery time).
4. The app queries `_vibecli._tcp.local.` on startup and every 90 seconds, resolves the SRV + A records, and adds the discovered IP:port to its URL race pool.

### Works on any IP range

mDNS uses link-local multicast — it works on 10.0.0.0/8, 192.168.0.0/16, 172.16.0.0/12, and any other private range without any routing changes.

### What you need

- Both devices on the same Wi-Fi (or wired) network.
- No firewall blocking UDP port 5353.

---

## Path 2 — Tailscale

[Tailscale](https://tailscale.com) is a mesh VPN. Once installed on both your development machine and phone, they share a private `100.x.x.x` address space regardless of where either device is physically located.

### Automatic detection

The daemon calls `tailscale status --json` on startup and includes the result in the `/mobile/beacon` response:

```json
{
  "tailscale_ip": "100.64.1.2",
  ...
}
```

The mobile app adds `http://100.64.1.2:7878` to the URL race automatically — no configuration needed beyond having Tailscale running.

### Tailscale Funnel (public HTTPS URL, opt-in)

[Tailscale Funnel](https://tailscale.com/kb/1223/tailscale-funnel) exposes the daemon as a public `https://<machine>.ts.net` endpoint accessible from any internet connection, even without Tailscale installed on the client.

Enable it in `~/.vibecli/config.toml`:

```toml
[tunnel]
tailscale_funnel = true
```

On `--serve`, the daemon:
1. Runs `tailscale funnel 7878` in the background.
2. Polls `tailscale status --json` until `Self.FunnelPorts` includes `443` and `Self.DNSName` is populated.
3. Stores the resulting `https://<machine>.<tailnet>.ts.net` URL in the beacon's `public_url` field.

The mobile app picks it up automatically — no pairing changes needed.

### Setup

1. [Install Tailscale](https://tailscale.com/download) on your Mac/Linux machine.
2. [Install Tailscale](https://tailscale.com/download) on your iPhone/Android.
3. Sign in to the same account on both.
4. Done. The 100.x.x.x address is detected automatically.

For Funnel: your Tailscale plan must support Funnel (personal plans included).

---

## Path 3 — ngrok

[ngrok](https://ngrok.com) creates a public HTTPS tunnel to your local daemon. Useful when you're not on Tailscale and need to reach the daemon from a phone on a different network.

### Auto-detection (zero config)

If ngrok is already running a tunnel to the daemon port, the daemon detects it automatically:

```bash
# Start ngrok separately (one-time setup or via ngrok config)
ngrok http 7878
```

On `--serve`, the daemon probes `localhost:4040/api/tunnels` and includes the public URL in the beacon if a matching tunnel exists. No config file changes needed.

### Auto-start (opt-in)

To have `vibecli --serve` start ngrok automatically:

```toml
# ~/.vibecli/config.toml
[tunnel]
ngrok_auto_start = true
ngrok_auth_token = "your-ngrok-auth-token"   # or set NGROK_AUTHTOKEN env var
```

The daemon spawns `ngrok http 7878` in the background and polls for the tunnel URL (up to 15 seconds). Once detected, it appears in the beacon's `public_url` field.

### Setup

1. [Install ngrok](https://ngrok.com/download) and add it to your PATH.
2. Sign up for a free ngrok account and copy your auth token from the dashboard.
3. Either set `NGROK_AUTHTOKEN=<token>` in your shell, or add it to `config.toml` as shown above.

Free ngrok accounts get one tunnel per session with a random URL. Paid plans offer stable custom domains.

---

## URL race — how the app picks the fastest path

The `HandoffService` in the mobile app never commits to a single path. On every probe cycle (startup + every 60 s) it:

1. Builds a candidate set from all available sources:
   - Stored `baseUrl` from the pairing QR code
   - LAN IPs from the latest beacon (`lan_ips`)
   - Tailscale IP from beacon (`tailscale_ip`)
   - ngrok / Tailscale Funnel URL from beacon (`public_url`)
   - mDNS-discovered IPs (queried independently every 90 s)

2. Races all candidates in parallel with a 3-second timeout each.

3. The first URL to respond with HTTP 200 on `/health` wins and is cached for that machine until the next probe.

This means if you start at home on Wi-Fi (mDNS wins), commute (ngrok or Tailscale wins), and arrive at the office on a different network (LAN mDNS wins again) — the app adapts silently without any user action.

---

## Beacon response reference

`GET /mobile/beacon` (no auth required) returns:

```json
{
  "machine_id": "a3f1c8e2b4d90571",
  "hostname": "my-mac",
  "daemon_version": "0.5.4",
  "port": 7878,
  "lan_ips": ["10.0.1.42"],
  "tailscale_ip": "100.64.1.2",
  "public_url": "https://my-mac.tailnet-abc.ts.net",
  "uptime_secs": 3612,
  "active_session": {
    "session_id": "sess_abc123",
    "task": "Refactor authentication module",
    "provider": "claude",
    "status": "running",
    "started_at": 1713100800,
    "message_count": 14,
    "summary": null
  }
}
```

| Field | Source | Notes |
|-------|--------|-------|
| `lan_ips` | UDP connect trick + `ip addr`/`ifconfig` | Primary outbound interface + all non-loopback IPs |
| `tailscale_ip` | `tailscale status --json` → `Self.TailscaleIPs[0]` | `null` if Tailscale not running |
| `public_url` | ngrok `localhost:4040` API or Tailscale Funnel DNS name | `null` if no tunnel active |
| `active_session` | Most recent running job, or finished job within last 15 min | Powers the Handoff banner |

---

## iOS sideloading (no App Store)

See the [VibeMobile setup guide](/vibemobile/#ios-sideloading) for building and installing the IPA with AltStore or Sideloadly using a free Apple ID.

---

## Troubleshooting

**App says "No machines found" on the same Wi-Fi**

- Check that UDP port 5353 is not blocked by your router's AP isolation setting. Many guest networks block mDNS between clients — use the home/regular network instead.
- The daemon logs `[vibecli serve] mDNS announcing _vibecli._tcp.local. on port 7878` on startup. If you don't see this, check that port 5353 is not blocked by a local firewall.

**Tailscale IP shows in beacon but app can't connect**

- Verify both devices are on the same Tailscale account: `tailscale status` on the machine should list your phone.
- Check that the daemon's port (7878) is not blocked by the machine's firewall for the Tailscale interface (`utun` on macOS, `tailscale0` on Linux).

**ngrok tunnel detected but URL times out**

- Free ngrok URLs expire when the ngrok process exits. Restart `ngrok http 7878` or add `ngrok_auto_start = true` to config.
- If `ngrok_auto_start = true` and startup logs show `ngrok start failed`, verify `ngrok` is on PATH and the auth token is valid: `ngrok config check`.
