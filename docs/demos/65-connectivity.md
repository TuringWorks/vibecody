---
layout: page
title: "Demo 65: Zero-Config Connectivity"
permalink: /demos/65-connectivity/
nav_order: 65
parent: Demos
---

## Overview

VibeCody discovers peers and exposes its daemon without manual port-forwarding or config files. Two mechanisms work in tandem:

- **mDNS LAN discovery** — broadcast `_vibecli._tcp` on the local network so other VibeCLI instances and the VibeUI desktop app find each other instantly
- **ngrok auto-detection** — if `ngrok` is running, VibeCLI reads the tunnel URL from the ngrok API and announces it so remote collaborators can connect

---

## mDNS LAN Discovery

```bash
# Start daemon with mDNS announcement (enabled by default)
vibecli --serve --port 7860

# Output:
# [vibecli] A2A protocol enabled on port 7860
# [mdns] Announced _vibecli._tcp → vibecli-macbook.local:7860
# [vibecli serve] Listening on http://127.0.0.1:7860

# Discover peers on your LAN (from any machine)
vibecli --discover

# Output:
# Peers on local network:
#   alice-mbp.local       → http://alice-mbp.local:7860
#   build-server.local    → http://build-server.local:7860
```

### How it works

VibeCLI registers an mDNS service record via the `mdns_announce` module:

```
Service type : _vibecli._tcp.local.
Instance     : vibecli-<hostname>
Port         : 7860 (configurable)
TXT records  : version=<semver>, agent=<agent-name>
```

The VibeUI desktop app listens for these announcements and populates the "Local Peers" panel automatically.

---

## ngrok Tunnel Auto-Detection

```bash
# 1. Start ngrok in another terminal (or as a background service)
ngrok http 7860

# 2. Start VibeCLI — it detects the active tunnel automatically
vibecli --serve

# Output:
# [ngrok] Detected tunnel: https://abc123.ngrok.io → 127.0.0.1:7860
# [ngrok] Public URL: https://abc123.ngrok.io
# [vibecli serve] Listening on http://127.0.0.1:7860

# Share the ngrok URL with a remote collaborator
vibecli --tunnel-url
# → https://abc123.ngrok.io
```

### Detection mechanism

The `ngrok` module polls `http://127.0.0.1:4040/api/tunnels` (ngrok's local API) every 5 seconds. When a `https` tunnel is found pointing to the configured daemon port, the URL is stored and broadcast via mDNS TXT record `tunnel=<url>`.

```bash
# Manually set a tunnel URL (useful with other tunnel providers)
vibecli --config set network.tunnel_url https://my-tunnel.example.com

# Disable auto-detection
vibecli --config set network.ngrok_autodetect false
```

---

## Tailscale Integration

For persistent, authenticated connectivity use [Tailscale Funnel](https://tailscale.com/kb/1223/funnel):

```bash
# See Demo 55 for the full Tailscale walkthrough
vibecli --serve --tailscale
# [tailscale] Funnel URL: https://myhost.tail12345.ts.net/vibecli
```

The `tailscale` module integrates with `mdns_announce` — the Funnel URL is added as a TXT record alongside the ngrok URL so peers have both options.

---

## VibeUI Auto-Connect

When the VibeUI desktop app starts, it:

1. Listens for `_vibecli._tcp` mDNS announcements
2. Checks the ngrok TXT record on discovered peers
3. Connects to the first healthy peer (ping < 100ms)
4. Falls back to `http://127.0.0.1:7860` if no peers found

This means opening VibeUI on a machine where VibeCLI is already running connects them automatically — no configuration needed.

---

## Configuration

```toml
# ~/.vibecli/config.toml
[network]
mdns_enabled       = true
mdns_service_name  = "vibecli"          # → _vibecli._tcp
mdns_port          = 7860
ngrok_autodetect   = true
ngrok_api_port     = 4040               # default ngrok local API port
tunnel_url         = ""                 # override: set manually if needed
```

---

## Rust Modules

| Module | Purpose |
|--------|---------|
| `mdns_announce` | mDNS/DNS-SD service registration and peer discovery |
| `ngrok` | ngrok local API polling, tunnel URL extraction |
| `tailscale` | Tailscale daemon integration, Funnel URL detection |

---

## Related Demos

- [55 — Voice, Pairing & Tailscale](../55-voice-pairing-tailscale/) — QR code pairing, Tailscale Funnel
- [56 — Browser Web Client](../56-web-client/) — Web SPA that connects to the daemon
- [64 — Apple Watch & Wear OS](../64-watch-integration/) — Wrist companion uses the daemon API
- [37 — A2A Protocol](../37-a2a-protocol/) — Agent-to-Agent communication over the same daemon
