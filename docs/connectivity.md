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

## Security: which bind address to pick

The `--host` flag controls *which interface* the daemon listens on. It does **not** control authentication — every state-mutating route still requires a bearer token regardless of bind address. But the choice of bind address determines who can *reach* the daemon and try to brute-force that bearer.

Threat-model reference: [`docs/security/threat-model.md`](./security/threat-model.md) §7 items #7 and #18.

| `--host` value | Reachable from | When to use | Risk |
|---|---|---|---|
| **default (no flag)** / `127.0.0.1` / `localhost` | This machine only | Single-device editing | None — loopback is unreachable off-box. |
| `100.x.x.x` (your Tailscale IP) | Your tailnet | Paired phone / watch / second laptop on Tailscale | Low — Tailscale ACLs gate who can reach the IP. |
| `192.168.x.x` / `10.x.x.x` / `172.16.x.x` (your LAN IP) | Anyone on the LAN | Phone on the same Wi-Fi without Tailscale | **Medium** — every device on the LAN can probe `/health` and attempt bearer brute-force. Coffee-shop, conference, hotel Wi-Fi are all hostile LANs. Pair with a host firewall or a strong (≥128-bit) bearer. |
| `0.0.0.0` / `::` (wildcard) | Anyone reachable via any interface | Demos with no Tailscale option | **High** — equivalent to "all of the above". If the LAN is publicly routed (some hotel networks), this is reachable from the internet. |

The daemon **prints a stderr warning** on any non-loopback bind ([`serve.rs::emit_public_bind_warning`](https://github.com/TuringWorks/vibecody/blob/main/vibecli/vibecli-cli/src/serve.rs)). The warning is informational — we don't hard-fail because `--host 0.0.0.0` is a legitimate mobile-LAN flow — but it's a deliberate cue to add a firewall rule.

### Mental model

mDNS, Tailscale, and ngrok are **transports** — they all reach the same daemon. They do not change the bind address. If your daemon is bound to `127.0.0.1`:

- mDNS announcements still go out, but mobile clients can't connect (the daemon refuses).
- Tailscale routes packets to your machine, but the daemon ignores them.
- ngrok forwards public traffic to localhost, and the daemon serves it. **This is the safest "public" path** because ngrok itself is the trust boundary, not the LAN.

If you need phone/watch access without Tailscale, the typical pairing is:

```bash
vibecli serve --host 192.168.1.42   # bind your LAN interface explicitly, not 0.0.0.0
```

…with a host firewall rule that allows port 7878 only from the LAN subnet.

### Bearer-token rotation

Every `vibecli serve` start mints a fresh 128-bit bearer token. Restarting the daemon is the rotation procedure. See [`docs/security/key-rotation.md`](./security/key-rotation.md) for the full procedure (what survives rotation, what doesn't, and how to verify via `/health.api_token.minted_at_unix`).

### Verifying your bind is safe { #verifying-bind }

After `vibecli serve` starts, confirm the daemon is only reachable where you intended. The stderr warning fires on any non-loopback bind, but it's informational — these commands turn it into a yes/no check.

**1. What is the daemon actually listening on?**

```bash
# macOS / Linux
lsof -nP -iTCP:7878 -sTCP:LISTEN
# or
ss -ltnp 'sport = :7878'        # Linux
```

```powershell
# Windows
netstat -ano -p TCP | findstr :7878
```

You want to see `127.0.0.1:7878` (loopback-only) or a specific interface IP (`192.168.x.x`, `100.x.x.x`), **not** `*.7878` / `0.0.0.0:7878` / `[::]:7878` unless you intentionally chose `--host 0.0.0.0` for a documented mobile-LAN flow.

**2. Can another machine on the LAN reach you?**

From a second device on the same Wi-Fi:

```bash
# Quick probe — should connection-refuse if you're loopback-bound
curl -m 3 http://<your-lan-ip>:7878/health
# or
nc -zv <your-lan-ip> 7878
```

- Connection refused / timeout ⇒ safe (firewalled or loopback-bound).
- HTTP 401 / 200 ⇒ the daemon is reachable; review §"Security: which bind address to pick" above.

**3. Is the port reachable from the public internet?**

If you're on a residential ISP with double-NAT, this is almost certainly *no* — but coffee-shop / hotel / conference networks sometimes route public IPs directly to clients. Worst-case verification:

```bash
# From any phone on cellular (off-Wi-Fi):
curl -m 5 http://<your-public-ip-from-whatismyip>:7878/health
```

If that returns anything other than connection refused/timeout, your `--host 0.0.0.0` bind is internet-reachable. Mitigations, in order of preference:

1. **Switch to `--host 127.0.0.1` + ngrok or Tailscale Funnel** — moves the trust boundary to the tunnel provider and keeps the daemon socket loopback-only.
2. **Add a host firewall rule** that allows port 7878 only from your LAN subnet (e.g. `pf` on macOS, `ufw allow from 192.168.0.0/16 to any port 7878` on Linux).
3. **Bind the LAN interface explicitly** (`--host 192.168.1.42` instead of `0.0.0.0`) so you don't accidentally listen on a future-added interface.

### Pre-bind checklist { #pre-bind-checklist }

Before running `vibecli serve --host 0.0.0.0` (or any non-loopback host):

- [ ] Are you on a trusted LAN (home, office)? Coffee-shop / hotel / conference Wi-Fi count as **hostile** even though they look benign.
- [ ] Does your host firewall block port 7878 from anything other than your LAN?
- [ ] Could the LAN itself bridge to the public internet without NAT? (See verification step 3.)
- [ ] Have you considered ngrok / Tailscale / SSH-tunnel? They're typically the safer answer when "phone on the same Wi-Fi" is the actual requirement.

If any answer is "no" or "I'm not sure", default to loopback + a tunnel.

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
