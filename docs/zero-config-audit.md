---
layout: page
title: Zero-Config Audit
permalink: /zero-config-audit/
---

# Zero-Config Audit (2026-05-02)

> First pass of the codebase against [AGENTS.md → Zero-Config First](https://github.com/anthropics/claude-code/blob/main/AGENTS.md#zero-config-first--the-user-experience-contract). The policy: features ship working out-of-the-box; required values live in the encrypted ProfileStore; env vars are accepted as a *fallback* read path only, never the *only* way.
>
> **TL;DR**: 137 unique env-var reads. **35 are AI provider keys with ProfileStore paths (compliant).** **18 are developer-only knobs (compliant per the policy carve-out).** **44 are system context (`HOME`, `PATH` etc., not knobs).** **40+ are user-facing integration tokens, of which a documented subset already routes through ProfileStore via `config.rs`, but five files are env-only and need migration.**

## Methodology

```sh
# Find every env::var read in production crates
grep -rnE 'std::env::var\("[A-Z_]+' vibecli/vibecli-cli/src/ vibeui/crates/ \
  --include="*.rs" | grep -v "tests::"

# Bucket by category, then sample-check whether each file also touches ProfileStore
```

Each env var was placed into one of four buckets:

1. **Compliant** — has a ProfileStore read path; env is fallback-only.
2. **Developer-only** — affects internal behavior during local dev/debug. The policy carve-out applies (`RUST_BACKTRACE`, `VIBE_INFER_KV_CACHE`, etc.).
3. **System context** — not a knob; the daemon reads it to discover its own environment (`$HOME`, `$PATH`, `$EDITOR`).
4. **Violation** — user-facing setting that has no ProfileStore path. Must migrate.

## Counts

| Category | Count | Verdict |
|---|---|---|
| AI provider keys (Anthropic, OpenAI, Gemini, Grok, …) | 35 reads | ✅ Compliant — `Config::load() → overlay_from_store()` consults ProfileStore first |
| VibeCody-internal knobs (`VIBECLI_*`, `VIBE_INFER_*`) | 18 reads | ⚠️ Mostly developer-only; two user-facing (`VIBECLI_DAEMON_TOKEN`, `VIBECLI_DAEMON_URL`) — see below |
| Shell / system context (`HOME`, `PATH`, `EDITOR`, …) | 44 reads | ✅ Not knobs |
| Integration tokens (Slack, GitHub, Linear, Jira, etc.) | ~71 reads across 10 files | 🔧 Mixed — see violation list |

## Violations to migrate

These five files read integration tokens directly from env without consulting ProfileStore. Each one needs a `vibecli set-key <integration> ...` flow plus a daemon-side read fallback.

| File | Env vars read | Severity | Notes |
|---|---|---|---|
| `vibecli/vibecli-cli/src/bugbot.rs` | `GH_TOKEN`, `GITHUB_TOKEN` | Med | Already-paired GitHub flow exists in `github_app.rs`; bugbot should reuse it via `ProfileStore.get_api_key("default", "github")` |
| `vibecli/vibecli-cli/src/github_app.rs` | `GITHUB_APP_*`, `GH_TOKEN` | Med | Webhook signing secret + app token; webhook secret in particular *must* never live in env |
| `vibecli/vibecli-cli/src/productivity.rs` | `LINEAR_API_KEY`, `NOTION_API_KEY`, `JIRA_*`, `TODOIST_API_KEY` | High | Five separate integration tokens, none in ProfileStore. User-visible feature surface (`/v1/productivity/*`) — most likely env-only path users hit |
| `vibecli/vibecli-cli/src/vulnerability_db.rs` | `GH_TOKEN` | Low | Internal scanner; could fall back to `bugbot.rs`'s GitHub creds path once that's compliant |
| `vibeui/crates/vibe-ai/src/providers/copilot.rs` | `COPILOT_TOKEN` | Med | Unlike other AI providers, this one bypasses `overlay_from_store`; the Tauri `commands.rs` `build_temp_provider` match arm needs to consult ProfileStore |
| `vibeui/crates/vibe-ai/src/providers/native_connectors.rs` | several integration vars | Med | Sampled earlier; same shape as `productivity.rs` |

### Discoverability gap

Of the violations above, **none currently surface in the daemon startup banner or `/health`**. A user whose `LINEAR_API_KEY` isn't set sees a generic "auth failed" error from the upstream API on first call, not a "configure with `vibecli set-key linear ...`" hint. Per the policy's third rule (every config knob is documented and discoverable), each violation should add:

1. A startup banner line if mistralrs-style "this feature won't work without configuration" is the right framing, OR
2. A `/health` field per integration: `{ "linear_configured": false, "github_configured": true, ... }`, OR (preferred — less banner noise)
3. A single `/health.integrations.unconfigured: ["linear", "notion"]` array.

Recommend option 3 — single line in `/health`, no banner spam, easy for the UI to consume.

## VibeCody-internal knobs — triage

The `VIBECLI_*` and `VIBE_INFER_*` vars are project-internal. Triage:

| Var | Category | Action |
|---|---|---|
| `VIBE_INFER_KV_CACHE` | Developer-only — switches FP16 vs TurboQuant | ✅ Keep as env (user picks via UI dropdown that prepends the env at launch) |
| `VIBE_INFER_KV_CACHE_SEED` / `_QJL_DIM` | Developer-only — codec parameter tuning | ✅ Keep as env |
| `VIBE_INFER_TURBOQUANT_BACKEND` | Developer-only — debug native vs Candle codec | ✅ Keep as env |
| `VIBE_INFER_MODEL` | Used by examples (`generate.rs`, `kv_cache_compare.rs`) — example-only | ✅ Keep as env (examples, not user-facing) |
| `VIBECLI_BACKEND_PINS` / `VIBECLI_DEFAULT_BACKEND` | Developer/test override of inference router | ✅ Keep as env |
| `VIBECLI_WORKER_MODE` | Internal worker spawn flag | ✅ Keep as env |
| `VIBECLI_MACHINE_ID` | Daemon machine identity, used during pairing | ⚠️ Should derive from `ProfileStore.machine_id()` (already has one) and only fall back to env for test harnesses |
| `VIBECLI_A2A_HOST` / `VIBECLI_A2A_PORT` | Agent-to-agent network endpoint | 🔧 User-facing — should be `vibecli set-key a2a.host` / `a2a.port` (or stored under daemon settings table) |
| `VIBECLI_DAEMON_TOKEN` / `VIBECLI_DAEMON_URL` | Client-side: where to find the daemon, what bearer to send | 🔧 User-facing on the client (CLI / UI) — should be discovered via mDNS or read from `ProfileStore` after pairing, never set by hand |

## What's already compliant — credit where due

Six integrations already do this right (env is a fallback; ProfileStore is canonical):

| Integration | File | ProfileStore key |
|---|---|---|
| Slack | `config.rs` | `integration.slack.bot_token` |
| Jira | `config.rs` | `integration.jira.api_token` |
| Home Assistant | `config.rs` + `home_assistant.rs` | `integration.home_assistant.token` |
| Gmail | `config.rs` + `email_client.rs` | `integration.email.gmail_access_token` |
| Outlook | `config.rs` + `email_client.rs` | `integration.email.outlook_access_token` |
| Google Calendar | `config.rs` + `calendar_client.rs` | `integration.calendar.google_access_token` |
| Linear (partial) | `linear.rs` reads from ProfileStore but `native_connectors.rs` does not | (cross-file inconsistency) |

The pattern those six follow is the model for the violations above:

```rust
// 0. ProfileStore (encrypted SQLite) — highest priority
if let Ok(store) = crate::profile_store::ProfileStore::new() {
    if let Ok(Some(token)) = store.get_api_key("default", "integration.slack.bot_token") {
        return Some(token);
    }
}
// 1. Config TOML (legacy, being phased out)
// 2. Env var (compatibility fallback)
if let Ok(env) = std::env::var("SLACK_BOT_TOKEN") {
    return Some(env);
}
None
```

## Next steps

In rough priority order:

1. **`productivity.rs` migration** — five integration tokens, highest user-visible blast radius. ~80 lines per token to mirror the `home_assistant.rs` pattern. Add `vibecli set-key linear|notion|jira|jira_email|jira_url|todoist <value>`.
2. **`copilot.rs`** — bring the Copilot AI provider under the `overlay_from_store` chain so it resolves like every other AI provider. ~20 lines.
3. **`/health.integrations.unconfigured`** — single field listing integrations that have no ProfileStore entry. Lets the UI show a "needs configuration" badge per surface without users hunting through docs.
4. **`bugbot.rs` + `vulnerability_db.rs`** — both want a single `github` token; share the `github_app.rs` ProfileStore key once that one is compliant.
5. **`VIBECLI_MACHINE_ID`** — switch primary path to `ProfileStore.machine_id()`; env stays for test harnesses only.
6. **Cross-file consistency**: pick one of `linear.rs` (compliant) or `native_connectors.rs` (env-only) as the sole Linear path. Two places fighting over the same key is its own bug.

Each item is small. Sequencing by blast radius rather than effort: ship #1 next, #2-#3 together, then sweep the rest.

## See also

- [AGENTS.md → Zero-Config First](https://github.com/anthropics/claude-code/blob/main/AGENTS.md#zero-config-first--the-user-experience-contract) — the policy this audit checks against.
- [Configuration](/configuration/) — user-facing setup reference (where users will look for missing settings).
- [Model Comparison](/model-comparison/) — the recent `vibecli set-key huggingface` precedent.
