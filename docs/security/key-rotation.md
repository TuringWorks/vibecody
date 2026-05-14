# Daemon Bearer Token Rotation

> Companion to [`threat-model.md`](./threat-model.md) §7 item #20. Procedure for invalidating a leaked or stale `vibecli serve` bearer token.

## Current behavior — implicit rotation on every restart

The daemon mints a **fresh 128-bit bearer token on every `vibecli serve` start** (`serve.rs::serve` body — `let api_token = format!("{:032x}", rand::rng().random::<u128>())`). The token is:

- Returned by `/health.api_token.minted_at_unix` so clients can detect a restart.
- Printed (masked) to stderr in the startup banner.
- Written to `~/.vibecli/daemon.token` (mode 0600 on Unix) so other local tools can find it.

No bearer token survives a daemon restart. If the daemon is running, the token has been live since `/health.api_token.minted_at_unix`.

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

# 2. (Optional) clear the old token file so anyone who stashed a copy
#    doesn't have a recovery path on disk.
rm -f ~/.vibecli/daemon.token

# 3. Start the daemon again. New token gets written to ~/.vibecli/daemon.token
#    and printed (masked) on stderr.
vibecli serve
```

After step 3, clients that were authenticated with the old token (mobile, watch, VibeUI tabs) will start receiving `401 Unauthorized`. They need the new token:

- **VibeUI**: reads `~/.vibecli/daemon.token` at IPC time on the same host — no user action required.
- **Mobile / Watch**: re-pair through the daemon's `/pair` endpoint. Device keys (P-256 ECDSA per [AGENTS.md](../../AGENTS.md)) survive the rotation; only the *bearer* changes.
- **Manual API consumers** (scripts, `curl`, etc.): read the new value from `~/.vibecli/daemon.token` or copy from the startup banner.

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
  "rotation_doc": "docs/security/key-rotation.md"
}
```

`age_seconds` should be in the single-digit range immediately after rotation.

## Out of scope (today)

The following are *not* supported and are not planned for the current release:

| Want | Why not today |
|---|---|
| Rotate without dropping in-flight requests | Single-user system; daemon restart is sub-second. Hot rotation would add a grace-window mechanism for marginal benefit. |
| Per-client bearer tokens | Device-bound credentials already exist for mobile/watch via P-256 ECDSA pairing. The "single bearer for the host" model is appropriate for `127.0.0.1`-bound usage. |
| Bearer revocation list | Same — one token, one daemon, restart-to-rotate. |
| OS keychain integration for the bearer | The bearer is per-session and persisted only to `~/.vibecli/daemon.token` (mode 0600). Keychain integration would survive restarts, which is the wrong model. The encrypted ProfileStore handles long-lived LLM API keys (those *should* survive restarts). |

If you have a use case that requires any of the above, file an issue with the scenario — the trade-off math is different for multi-user deployments and we'll reconsider.

## Related

- [`threat-model.md`](./threat-model.md) §7 items #6 (constant-time bearer compare), #8 (per-IP rate limit), #20 (this document)
- [`AGENTS.md`](../../AGENTS.md) — pairing, device keys, encrypted stores
- `serve.rs::require_auth` and `auth_util::bearer_matches` — the enforcement points
