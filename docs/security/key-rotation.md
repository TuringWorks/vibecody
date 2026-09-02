---
layout: page
title: "Daemon Token Rotation"
permalink: /security/key-rotation/
---

# Daemon Bearer Token Rotation

> Companion to [`threat-model.md`]({{ site.baseurl }}/security/threat-model/) §7 item #20. Procedure for invalidating a leaked or stale `vibecli serve` bearer token.

## Current behavior — implicit rotation on every restart

The daemon mints a **fresh 128-bit bearer token on every `vibecli serve` start** (`serve.rs::serve` body — `let api_token = format!("{:032x}", rand::rng().random::<u128>())`). The token is:

- Returned by `/health.api_token.minted_at_unix` so clients can detect a restart.
- Identified by `/health.api_token.fingerprint` — the first 16 hex characters of its SHA-256 — so a client can tell whether the token it holds is the one this daemon accepts, **without the token ever appearing in the body**. Publishing it on an unauthenticated route is safe: the token is 128 bits of CSPRNG output, so there is no dictionary to search and 64 bits of digest narrows nothing.
- Printed (masked) to stderr in the startup banner, alongside the exact files written.
- Written to `~/.vibecli/daemon-<port>.token` (mode 0600 on Unix), plus `~/.vibecli/daemon.token` **only** when this daemon is on the port clients resolve by default.

No bearer token survives a daemon restart. If the daemon is running, the token has been live since `/health.api_token.minted_at_unix`.

### Why the file is named after the port

`daemon.token` names no port, and the daemon runs on any port — so every daemon on the machine wrote the same file and the last writer won regardless of which one was still alive. Observed: a daemon bound port 7979, wrote `daemon.token`, and exited two minutes later; the daemon on 7878 stayed healthy and **every client on the machine received 401 for two and a half days**, with nothing anywhere able to say why. `vibe_daemon_token::files_owned_by` is now the only thing that decides which files a daemon writes, and a daemon on a non-default port never touches the shared one.

## When to rotate

| Trigger | What to do |
|---|---|
| You think the token was logged, screenshotted, or pasted somewhere it shouldn't be | **Rotate now** (procedure below) |
| You're handing off the machine, taking a break, or shutting the laptop for travel | Stop the daemon (`pkill vibecli`) — the next start mints a new token |
| Routine hygiene | Restart at least every 30 days; check `/health.api_token.age_seconds` to know the current age |
| `--host` is not `127.0.0.1` (LAN-exposed mode) and you're done using a remote device | Rotate after disconnecting |

## How to rotate (single-machine)

```bash
# 1. Stop the running daemon. It's safe to lose in-flight requests for any
#    user-driven workflow; agent jobs that need to survive a restart are
#    persisted in ~/.vibecli/jobs.db and resumed on next start.
pkill -f 'vibecli serve' 2>/dev/null || true

# 2. (Optional) clear the old token files so anyone who stashed a copy
#    doesn't have a recovery path on disk.
rm -f ~/.vibecli/daemon.token ~/.vibecli/daemon-*.token

# 3. Start the daemon again. The new token is written to
#    ~/.vibecli/daemon-<port>.token (and daemon.token on the default port),
#    and printed (masked) on stderr along with the exact paths.
vibecli serve
```

After step 3, clients that were authenticated with the old token (mobile, watch, VibeCoder tabs) will start receiving `401 Unauthorized`. They need the new token:

- **Desktop shells**: read the token at IPC time on the same host and re-read on a 401 — no user action required. If a stale file survives (a daemon killed with `SIGKILL` never cleans up), the shells compare their token's fingerprint against `/health.api_token.fingerprint` and say *"restart the daemon"* rather than *"is the daemon running?"*.
- **Mobile / Watch**: re-pair through the daemon's `/pair` endpoint. Device keys (P-256 ECDSA per [AGENTS.md](https://github.com/TuringWorks/vibecody/blob/main/AGENTS.md)) survive the rotation; only the *bearer* changes.
- **Manual API consumers** (scripts, `curl`, etc.): read the new value from `~/.vibecli/daemon-<port>.token` or copy from the startup banner.

## Verifying rotation succeeded

```bash
# /health does not require auth and surfaces the token freshness.
curl -s http://127.0.0.1:7878/health | jq '.api_token'
```

Expected response (token itself is *never* in the body):

```json
{
  "minted_at_unix": 1715600000,
  "age_seconds": 3,
  "fingerprint": "9f2c41ab7d0e5836",
  "rotation_doc": "docs/security/key-rotation.md"
}
```

`age_seconds` should be in the single-digit range immediately after rotation.

To confirm the token *you* hold is the one this daemon accepts:

```bash
# Compare the first 16 hex of the file's SHA-256 against the fingerprint above.
shasum -a 256 ~/.vibecli/daemon-7878.token | cut -c1-16
```

A mismatch means a stale file, not a stopped daemon — restart the daemon on that port and it rewrites its own token.

## Out of scope (today)

The following are *not* supported and are not planned for the current release:

| Want | Why not today |
|---|---|
| Rotate without dropping in-flight requests | Single-user system; daemon restart is sub-second. Hot rotation would add a grace-window mechanism for marginal benefit. |
| Per-client bearer tokens | Device-bound credentials already exist for mobile/watch via P-256 ECDSA pairing. The "single bearer for the host" model is appropriate for `127.0.0.1`-bound usage. |
| Bearer revocation list | Same — one token, one daemon, restart-to-rotate. |
| OS keychain integration for the bearer | The bearer is per-session and persisted only to `~/.vibecli/daemon-<port>.token` (mode 0600). Keychain integration would survive restarts, which is the wrong model. The encrypted ProfileStore handles long-lived LLM API keys (those *should* survive restarts). |

If you have a use case that requires any of the above, file an issue with the scenario — the trade-off math is different for multi-user deployments and we'll reconsider.

## Related

- [`threat-model.md`]({{ site.baseurl }}/security/threat-model/) §7 items #6 (constant-time bearer compare), #8 (per-IP rate limit), #20 (this document)
- [`AGENTS.md`](https://github.com/TuringWorks/vibecody/blob/main/AGENTS.md) — pairing, device keys, encrypted stores
- `serve.rs::require_auth` and `auth_util::bearer_matches` — the enforcement points
