---
layout: page
title: Background Jobs
permalink: /background-jobs/
---

# Background Jobs

> Long-running agent tasks that don't block your editor. Submit a job, walk away, come back to a finished result — or watch the live event stream and intervene mid-flight if you want to. Jobs persist across daemon restarts, replay their event history on resume, and surface a heuristic recap the moment they finish.

The Background Jobs panel is VibeCody's queued-work surface. It's wired around three principles:

1. **Durable.** Jobs land in `~/.vibecli/jobs.db` (encrypted, machine-bound) before they start running. A daemon crash never loses a job.
2. **Replayable.** Every agent step (tool call, message, completion) is appended as a `seq`-ordered event. Reconnect later and the daemon replays from your last seen sequence.
3. **Recap-aware.** Every terminal transition (Complete / Failed / Cancelled) auto-writes a heuristic recap so the Resume flow has something to seed from.

---

## Submitting a job

### From the panel

VibeUI → **Background Jobs** tab → enter a task description → choose provider/model → **Submit**. The job lands as Queued, transitions to Running when the worker picks it up, and finishes Complete / Failed / Cancelled.

### From the CLI

```bash
# Submit
curl -X POST http://127.0.0.1:7878/jobs \
     -H 'content-type: application/json' \
     -H "authorization: bearer $VIBECLI_TOKEN" \
     -d '{
       "task": "Refactor the SSRF guard in vibe-net",
       "provider": "claude",
       "model": "claude-sonnet-4-6"
     }'
```

Response: `{ "session_id": "<sid>" }`. That `sid` is the job's stable handle.

### From scripts (Agent SDK)

```typescript
import { submitJob } from "@vibecody/agent-sdk";

const { sessionId } = await submitJob({
  task: "Run cargo test --workspace and fix any failures",
  provider: "claude",
  model: "claude-sonnet-4-6",
});
```

---

## Job lifecycle

```
            ┌─────────┐    pickup     ┌─────────┐
            │ Queued  │──────────────▶│ Running │
            └─────────┘               └────┬────┘
                                           │
            ┌──────────────┬───────────────┴────────────────┐
            ▼              ▼                                ▼
        ┌────────┐    ┌────────┐                       ┌──────────┐
        │Complete│    │ Failed │                       │Cancelled │
        └────┬───┘    └────┬───┘                       └─────┬────┘
             │             │                                 │
             └─────────────┴── auto-recap on terminal ──────┘
                            (J1.2 — heuristic generator)
```

States are one-way; a Complete job never re-runs (resume creates a *fresh* job whose `parent_job_id` links back). Cancellation is graceful: the daemon sends a stop signal, lets the agent commit any pending event, then marks Cancelled.

---

## Streaming events

Jobs emit a `seq`-ordered event stream you can subscribe to live or replay:

```bash
# Subscribe (SSE)
curl -N -H "authorization: bearer $VIBECLI_TOKEN" \
  "http://127.0.0.1:7878/jobs/<sid>/events"

# Replay from a checkpoint (after a reconnect)
curl -H "authorization: bearer $VIBECLI_TOKEN" \
  "http://127.0.0.1:7878/jobs/<sid>/events?since=42"
```

Event payloads are typed (`{ "t": "step", ... }`, `{ "t": "tool_call", ... }`, `{ "t": "complete", ... }`). The panel uses these to render the live timeline; the recap generator uses the full event log to build the headline + bullets.

---

## Cancelling

```bash
curl -X POST -H "authorization: bearer $VIBECLI_TOKEN" \
  "http://127.0.0.1:7878/jobs/<sid>/cancel" \
  -d '{"reason":"user-requested"}'
```

The daemon transitions to `Stopping`, waits for the agent to commit, then `Cancelled`. The cancellation reason is stored on the job and shown in the recap.

---

## Resume

A finished job's recap shows a **Resume from here** button. Clicking it:

1. Calls `POST /v1/resume` with `kind: "job"` and the recap id.
2. Spawns a fresh job whose `parent_job_id` and `resumed_from_recap_id` link back to the source.
3. Inherits the parent's workspace path + approval policy.

You can also resume by `subject_id` directly:

```bash
curl -X POST http://127.0.0.1:7878/v1/resume \
     -H 'content-type: application/json' \
     -d '{"kind":"job","from_subject_id":"<parent-sid>"}'
```

See [Recap & Resume](../recap/) for the full dispatch logic and the `Recap` shape.

---

## Per-bucket quotas

Jobs can be created with a `quota_bucket` so the daemon enforces fair-share limits:

```json
{
  "task": "...",
  "provider": "claude",
  "model": "claude-sonnet-4-6",
  "quota_bucket": "team-frontend"
}
```

The daemon checks the bucket's `Tasks` resource before persisting. When the cap is hit you get:

```json
{ "error": "quota_denied", "resource": "Tasks", "used": 50, "hard_limit": 50 }
```

Configure quotas via `JobManager::set_agent_quotas` or the `[jobs.quotas]` block in `~/.vibecli/config.toml`.

---

## Webhooks

Set `webhook_url` on submit and the daemon POSTs the job's terminal status to that URL:

```json
{
  "session_id": "<sid>",
  "status": "Complete",
  "summary": "Refactored 3 files, all tests pass",
  "started_at": 1714521600000,
  "finished_at": 1714521660000
}
```

Delivery is at-least-once with exponential backoff. Permanent failures land in the dead-letter queue (visible at `/v1/jobs/dead-letters`).

---

## Storage

| Item | Path |
|---|---|
| Job records | `~/.vibecli/jobs.db` (encrypted, table `jobs`) |
| Event log | `~/.vibecli/jobs.db` (table `events`, one row per `seq`) |
| Job recaps | `~/.vibecli/jobs.db` (table `recaps`) |
| Dead-letter webhooks | `~/.vibecli/jobs.db` (table `webhook_deliveries`, status=dead) |

Single-file SQLite, encrypted with the same machine-bound key as ProfileStore. Backup the file directly — it round-trips cleanly.

---

## Health and metrics

`/health.features.background_jobs` declares the feature; live counts come from `/v1/metrics/jobs`:

```bash
curl http://127.0.0.1:7878/health | jq '.features.background_jobs'
# { "available": true, "transport": "daemon-http", "routes_prefix": "/jobs",
#   "metrics_route": "/v1/metrics/jobs", "store_path": "~/.vibecli/jobs.db" }

curl http://127.0.0.1:7878/v1/metrics/jobs | jq
# {
#   "jobs_created": 42,
#   "jobs_completed": 38,
#   "jobs_failed": 3,
#   "jobs_cancelled": 1,
#   "queued": 2,
#   "running": 1,
#   "events_published": 1284,
#   "webhooks_delivered": 38,
#   "webhooks_dead_lettered": 0
# }
```

Dashboards should pull from `/v1/metrics/jobs` (cheap — atomic counters); `/health` only declares the feature exists.

---

## Configuration

```toml
# ~/.vibecli/config.toml

[jobs]
# Max concurrent running jobs. 0 = unlimited.
max_concurrent = 4

# How long a Running job can sit without emitting events before the
# daemon force-marks it Failed. 0 = no timeout.
heartbeat_timeout = "10m"

# Webhook delivery — exponential-backoff retries.
webhook_retry_max = 8
webhook_retry_base = "5s"

[jobs.quotas]
# Per-bucket Tasks-resource caps. Set on JobManager startup.
"team-frontend" = { tasks_per_hour = 30, tasks_total = 200 }
"team-backend"  = { tasks_per_hour = 60, tasks_total = 500 }
```

---

## Troubleshooting

### "Daemon not running. Start it with: vibecli --serve --port 7878"

The panel's offline indicator is correct — the daemon is unreachable. Start it:

```bash
vibecli --serve --port 7878
```

The panel auto-reconnects when `/health` becomes reachable. No refresh needed.

### Jobs stuck in Queued

Either:
- Concurrency cap hit. Check `/v1/metrics/jobs.running` against `[jobs] max_concurrent`.
- The pickup loop is wedged. Check the daemon log for `vibecody::jobs` warnings.

### Event stream goes silent mid-job

The agent may have hung. Check `[jobs] heartbeat_timeout` — when set, the daemon will force-mark the job Failed after that interval. If it's 0, the job sits forever; restart the daemon to recover.

### Webhooks never arrive

- Check `/v1/metrics/jobs.webhooks_dead_lettered` — non-zero means delivery is failing permanently.
- Inspect dead letters at `/v1/jobs/dead-letters`. Common causes: 4xx from the receiver, TLS handshake failure, DNS.
- Re-run from the dead-letter queue: `POST /v1/jobs/<sid>/webhook/redeliver`.

### "Quota denied"

The bucket is at its hard limit. Either:
- Wait for the per-hour rolling window to drain.
- Reset via `POST /v1/jobs/quota/reset` with the bucket name (admin-only).

### Cancellation hangs in Stopping

The agent isn't responding to the stop signal. After 30 seconds the daemon force-cancels.

---

## Observability

Every job lifecycle transition emits structured `tracing` events under the `vibecody::jobs` target:

```bash
RUST_LOG=vibecody::jobs=info vibecli serve
```

Examples:

```
INFO vibecody::jobs: job.create: queued
  sid=ses_a1b2c3 provider=claude priority=0 tag_count=2

INFO vibecody::jobs: job.mark_running sid=ses_a1b2c3

INFO vibecody::jobs: job.mark_terminal
  sid=ses_a1b2c3 status=complete has_summary=true has_reason=false
```

Job *content* (the task text, agent messages, tool outputs) is never logged at any level — only stable IDs, statuses, and counts.

---

## Cross-client consistency

| Client | Background-jobs surface |
|---|---|
| **VibeUI (desktop)** | Full panel: submit, list, stream, cancel, recap on completion |
| **VibeCLI** | `vibecli jobs submit / list / cancel` (REPL + script) |
| **VibeMobile** | Full read + cancel (M2 — read jobs, watch live events, tap to cancel; M3 will add submit) |
| **VibeWatch** | Read-only glances of running + recently-finished jobs |
| **IDE plugins** | Submit + list via `/jobs` HTTP routes; per-IDE UX varies |
| **Agent SDK** | First-class `submitJob`, `streamEvents`, `cancelJob` helpers |

The daemon is the source of truth. Every client reads from `/jobs` or writes to `POST /jobs`; nobody bypasses the JobManager.

---

## Related

- **Design doc:** [`docs/design/recap-resume/02-job.md`](https://github.com/TuringWorks/vibecody/blob/main/docs/design/recap-resume/02-job.md) — Job recap shape + resume semantics (J1.x slices).
- **Recap & Resume:** [`/recap/`](../recap/) — How job recaps fit into the broader recap system.
- **Source:** `vibecli/vibecli-cli/src/job_manager.rs` (backend) · `vibeui/src/components/BackgroundJobsPanel.tsx` (UI).
