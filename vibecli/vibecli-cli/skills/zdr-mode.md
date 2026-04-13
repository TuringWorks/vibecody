# ZDR Mode — Zero Data Retention

## What is ZDR Mode?

Zero Data Retention (ZDR) mode makes every AI interaction **stateless and ephemeral**:

- **No session logging** — messages are never written to disk.
- **Full conversation history** — the entire chat is re-sent with each request
  so the provider reconstructs context without storing it server-side.
- **PII scrubbing** — email addresses, IP addresses, and JWT tokens are
  replaced with `[REDACTED]` before leaving the client.
- **API key scrubbing** — `sk-`, `ghp_`, and `xoxb-` tokens are redacted.

ZDR matches the enterprise guarantees of OpenAI Codex ZDR and Claude Code's
Zero Data Retention capability.

---

## When to Use ZDR Mode

Enable ZDR mode whenever you are working with:

- Proprietary source code that must not be retained by the AI provider.
- Personal health, financial, or legal data (HIPAA, GDPR, SOC 2 environments).
- API keys or credentials that might appear in code snippets or error messages.
- Air-gapped or high-security development environments.

---

## Enabling ZDR Mode

### CLI flag

```bash
vibecli --zdr
```

Equivalent to setting:

```toml
[zdr]
enabled = true
log_to_disk = false
retain_session = false
include_full_history = true
scrub_pii = true
scrub_api_keys = true
```

### Config file (`~/.vibecli/config.toml`)

```toml
[zdr]
enabled = true
scrub_pii = true
scrub_api_keys = true
```

---

## What Gets Scrubbed

| Pattern | Example | Replaced with |
|---------|---------|---------------|
| Email addresses | `alice@example.com` | `[REDACTED]` |
| IPv4 addresses | `192.168.1.100` | `[REDACTED]` |
| JWT tokens | `eyJhbGci…` | `[REDACTED]` |
| OpenAI API keys | `sk-abcdef…` | `[REDACTED]` |
| Anthropic keys | `sk-ant-api03-…` | `[REDACTED]` |
| GitHub tokens | `ghp_AbCdEf…` | `[REDACTED]` |
| Slack bot tokens | `xoxb-12345-…` | `[REDACTED]` |

Scrubbing is applied **before** the message is sent to any provider.

---

## Compliance Validation

A policy is **ZDR compliant** when all three conditions hold:

| Field | Required value |
|-------|---------------|
| `log_to_disk` | `false` |
| `retain_session` | `false` |
| `include_full_history` | `true` |

Any deviation is reported as a `ZdrViolation` with a human-readable reason.

---

## REPL Commands

| Command | Description |
|---------|-------------|
| `/zdr status` | Show current ZDR policy and compliance state. |
| `/zdr check` | Run compliance check and list any violations. |
| `/zdr enable` | Switch to the strict ZDR policy for this session. |
| `/zdr disable` | Switch to the permissive policy (logging re-enabled). |

### Examples

```
> /zdr status
ZDR mode: ENABLED
  log_to_disk       : false  ✓
  retain_session    : false  ✓
  include_full_history: true ✓
  scrub_pii         : true
  scrub_api_keys    : true
Compliance: PASS (0 violations)

> /zdr check
Compliance check — 0 violations. Policy is ZDR compliant.

> /zdr disable
ZDR mode disabled. Session logging is now active.

> /zdr enable
ZDR mode enabled. All messages will be scrubbed and sessions will not be retained.
```

---

## Architecture Notes

- **`ZdrPolicy`** — serialisable struct with the five control flags.
- **`ZdrSession`** — in-memory message accumulator; `build_request()` always
  packages the full history; `clear()` forgets everything.
- **`ZdrCompliance`** — validator that produces a list of `ZdrViolation`s.
- **`scrub_pii` / `scrub_api_keys` / `apply_scrubbing`** — pure free functions,
  no regex crate required; implemented with manual byte scanning.

All scrubbing happens on the client before any network call.  The provider
never sees raw PII or secret tokens.
