# 02 — Job Recap & Resume

**Scope:** background agent runs, Counsel deliberations, Arena comparisons
**Parent:** [`README.md`](./README.md)
**Status:** Draft · 2026-04-26

---

## What's there today

- **`JobManager`** — `vibecli/vibecli-cli/src/job_manager.rs` (~1600 lines). Durable async job queue. SQLite at `~/.vibecli/jobs.db` with **ChaCha20-Poly1305 encryption on BLOBs**. Tables: `jobs`, `job_events`, `webhook_deliveries`, `scratchpad`.
- **`JobRecord`** has `status`, `summary`, `started_at`, `finished_at`, `provider`, `webhook_url`, `priority`, `tags`, `cancellation_reason`, `steps_completed`, `tokens_used`, `cost_cents`. The `summary` field already exists — recap *replaces* it with structured content (and the existing flat `summary` becomes the recap's `headline`).
- **Live event stream:** `tokio::broadcast` channels per job, exposed via `GET /stream/:session_id` (SSE). Not durable across daemon restart — the durable copy lives in `job_events`.
- **Routes today:** `GET /jobs`, `GET /jobs/:id`, `POST /jobs/:id/cancel`, `GET /v1/metrics/jobs`. `POST /agent` creates a new job. **No** `/jobs/:id/recap`, `/jobs/:id/resume`.
- **vibeui `BackgroundJobsPanel`** polls `GET /jobs` every 10s, opens an `EventSource` per active job for live token stream. Currently shows a "Done" badge on terminal status; no structured handoff.
- **`CounselSession`** (`counsel.rs`) and Arena state are **in-memory only** — no durable store. Out of scope for the first cut; design covers the path that exists today (`JobRecord`-backed) and notes Counsel/Arena as follow-on.

## Goals

1. When a job reaches a terminal status (`complete`, `failed`, `cancelled`), generate a recap automatically and stop polling that job's event stream.
2. Replace the BackgroundJobs "Done" badge with a recap row showing headline + duration + tokens + cost + 3 bullets.
3. Make the recap re-runnable: "Resume" on a job spawns a *new* job whose seed task is the prior recap's `next_actions`, with full lineage tracked via `parent_job_id`.
4. Surface job recaps on mobile (a notification + a "Recap" detail view) and watch (a tap-able "Job done" complication / tile).
5. Keep `JobRecord.summary` as the *flat* one-liner (= recap headline) for backwards compatibility with the existing `/jobs` JSON shape.

## Non-goals

- Re-running with a different provider/model from a recap (that's a "fork-job" feature; out of scope here).
- Persisting Counsel rounds or Arena votes (those need their own scopes; recap design here applies once they're durable).
- Webhook delivery of recap JSON to the existing `webhook_url` field (could be an opt-in later; not in v1 to avoid surprise PII fan-out).

## Triggers

| Trigger | Default |
|---|---|
| Job transitions to `complete` / `failed` / `cancelled` | Auto-recap (heuristic) |
| User invokes `POST /v1/recap` with `kind: job` | On-demand (heuristic or LLM) |
| Job is `cancelled` with `cancellation_reason` set | Auto-recap; reason is included as a bullet |

Auto-recap fires from the existing terminal-state hook in `JobManager` (the same place that today writes `summary`). The recap call is awaited but timeboxed (1.5s for heuristic, 10s for LLM) so a slow recap can't block the job-state transition; on timeout, the recap is queued for a follow-up async pass.

## Data model

Inherits the cross-cutting `Recap` shape. Job-specific `ResumeHint`:

```rust
pub struct ResumeHint {
    pub target: ResumeTarget::Job(JobId),
    pub from_step: Option<u32>,            // currently always None — re-run from start
    pub seed_instruction: Option<String>,  // = next_actions[0], pre-fills the new task
    pub inherit_provider: bool,            // default true
    pub inherit_tags: bool,                // default true
}
```

Resume of a job spawns a **new** job (new `session_id`), linked to the parent via a new column:

```sql
ALTER TABLE jobs ADD COLUMN parent_job_id TEXT;       -- the job this was resumed from
ALTER TABLE jobs ADD COLUMN resumed_from_recap_id TEXT;  -- the recap that triggered the resume
CREATE INDEX IF NOT EXISTS idx_jobs_parent ON jobs(parent_job_id);
```

Migration via the existing idempotent `ALTER TABLE` pattern. The `sessions.db` `parent_session_id` column (already present) is the analogous primitive — same idea, different store.

## Storage

New `recaps` table on `jobs.db` with the same columns as `01-session.md`, but **encrypted body**:

```sql
CREATE TABLE IF NOT EXISTS recaps (
    id TEXT PRIMARY KEY,                    -- ULID
    kind TEXT NOT NULL DEFAULT 'job',
    subject_id TEXT NOT NULL,               -- job_id
    last_event_seq INTEGER,                 -- job_events.seq for idempotency
    workspace TEXT,
    generated_at TEXT NOT NULL,
    generator_kind TEXT NOT NULL,
    generator_provider TEXT,
    generator_model TEXT,
    headline_enc BLOB NOT NULL,             -- ChaCha20-Poly1305(headline)
    body_enc BLOB NOT NULL,                 -- ChaCha20-Poly1305(json bullets+next_actions+artifacts+resume_hint)
    token_input INTEGER,
    token_output INTEGER,
    cost_cents INTEGER,
    schema_version INTEGER NOT NULL DEFAULT 1,
    FOREIGN KEY (subject_id) REFERENCES jobs(session_id) ON DELETE CASCADE
);

CREATE INDEX IF NOT EXISTS idx_jobrecaps_subject ON recaps(subject_id);
CREATE UNIQUE INDEX IF NOT EXISTS uq_jobrecaps_subject_seq ON recaps(subject_id, last_event_seq);
```

Encryption key: existing `jobs.db` ChaCha20-Poly1305 setup (machine-bound: `SHA-256("vibecli-jobs-store-v1:" + HOME + ":" + USER)`). No new key derivation.

The `headline` is encrypted alongside the body — even the one-liner can leak intent. (Contrast with `sessions.db` recaps, which are unencrypted because the surrounding store is unencrypted.)

## RPC contract

The shared routes from `01-session.md` apply, with `kind: "job"`. Plus job-specific:

### `POST /v1/recap` (kind=job)

```jsonc
{
  "kind": "job",
  "subject_id": "01HJOB...",
  "force": false,
  "generator": "auto",      // job recaps default to "auto" — heuristic if <30 events, else LLM
  "include_event_excerpts": false  // job-only: include 5 representative events in body.artifacts
}
```

### `POST /v1/resume` (kind=job)

```jsonc
{
  "from_recap_id": "01HMRECAP...",
  "kind": "job",
  "seed_instruction": null,           // overrides recap.resume_hint.seed_instruction
  "inherit_provider": true,
  "inherit_tags": true,
  "client": "vibeui"
}

// response 200
{
  "handle": "01HRESUME...",
  "resumed_session_id": "01HJOB_NEW...",   // a NEW job ID; parent linked via parent_job_id
  "ready": true                             // jobs are queued, not warmed; ready means "queued"
}
```

The new job appears in `GET /jobs` with `parent_job_id` populated and `resumed_from_recap_id` set. Existing job filters/queries continue to work.

### Webhook posture

The existing `webhook_url` field on `JobRecord` delivers events at `complete` / `failed` today. **New optional field:** `webhook_recap` (bool, default `false`). When `true`, the webhook payload includes the recap JSON inline. Off by default to avoid silent PII fan-out.

## Per-surface UX

### vibeui — BackgroundJobsPanel

Today (per the survey at `BackgroundJobsPanel.tsx`): polls `/jobs` every 10s, opens an `EventSource` per active job for tokens.

Changes:

- **On job terminal:** the panel stops the EventSource (it does already), and instead of showing the existing "Done" badge, renders a **recap row** with:
  - Headline (1 line, truncated with tooltip)
  - Duration, tokens, cost (existing fields, unchanged)
  - 3 bullets (collapsible)
  - "Resume" button → calls `/v1/resume`, optimistically adds the new job to the list with a "Queued" badge
  - "View transcript" → existing behavior
- **Recap row uses the existing card pattern** (no new design tokens). Bullets render as a `ul.text-secondary` list.
- **Filter pill** "Show recaps only" hides in-flight rows for users who only want history.
- **Settings → Background Jobs:**
  - "Auto-recap on completion" — default on
  - "Recap generator" — auto (default) | heuristic | LLM
  - "Show cost & tokens in recap row" — default on

### vibecli REPL / TUI

- **`/jobs` REPL command** (already exists, lists jobs) gains recap headline as the second line per row.
- **`/recap job <id>`** — print the job recap.
- **`/resume job <id>`** — spawn the resume job; print the new job ID + a follow link `/stream <new_id>` to tail it.
- **End-of-async-job notification:** when a backgrounded job (started with `--agent --background`) completes, the next REPL prompt shows a one-liner: `[job 01HJOB… complete: <headline>] (/recap, /resume, /view)`. Suppressible with `VIBECLI_NO_JOB_NOTICE=1`.

### vibemobile

- **NotificationService** (already exists, in-memory ring buffer) gains a new category: `job_recap`. When the app foregrounds and the daemon reports a recently completed job (since last seen), a notification card surfaces with the headline + "Open" / "Resume" actions.
- **Sessions tab** (currently shows handoff list via `/mobile/sessions`) gains a "Jobs" subtab listing recent job recaps. Tap → `JobRecapView` with the full bullets + a "Resume" button.
- **Background-fetch path is not present today** — design adds it as a follow-on (`M1.2`), not a v1 blocker. v1 surfaces recaps when the user opens the app.

### vibewatch (watchOS + Wear)

- **Complication / Tile (watchOS) and Tile (Wear):** when a job from the user's paired daemon completes, the latest recap headline rotates into the complication slot. Tap → opens a slim `JobRecapView` showing headline + 3 bullets max + "Resume on phone".
- **No watch-side resume.** "Resume on phone" posts to `/mobile/sessions` to register a handoff, surfacing a notification on the paired phone (same primitive as session resume in `01`).
- **Networking:** existing `/watch/sessions` and `/watch/stream/:id` routes get a sibling `GET /watch/recaps?since=<ts>&kind=job&limit=10` (slimmed shape: id, headline, generated_at, kind). No new auth.
- **Cryptography:** P-256 ECDSA, unchanged. Recaps are encrypted in `jobs.db` server-side; over-the-wire they go via the same TLS-or-paired-token transport already in place.

## Counsel & Arena (deferred)

`CounselSession` and Arena votes are in-memory today (`counsel.rs` and the Arena panel state). For them to participate in the recap surface, they first need durable storage:

- Counsel: a new `counsel_sessions` table on `jobs.db` (or on `sessions.db` — TBD; argument for `jobs.db` is encryption parity). Once durable, recap fits with no new shape; the `kind` discriminator extends to `counsel` in a future slice.
- Arena: votes are simple enough to fit in `sessions.db` as a side table, but the recap value is low (a one-liner "preferred A in 4 of 5 prompts" is fine). Probably skip recap for Arena entirely; ship a recap-aware "Arena history" view instead.

This work is **out of scope for v1** of job recaps. Tracked as follow-on slices `J2.x` and `J3.x`.

## Heuristic recap algorithm (job-specific)

1. **Headline:** existing `JobRecord.task` field, trimmed to 80 chars. (This is the user's original task — exactly what they'd want to see.)
2. **Bullets** (3–5):
   - Status bullet: `Completed in 4m 12s` or `Failed: <cancellation_reason>` or `Cancelled by user`.
   - Step bullet: `Ran N tool calls (<top-3-tool-names>)` from `job_events` filtered to step events.
   - Files bullet (if any): `Touched M files: <first 3>, …`.
   - Token / cost bullet: `Used 12.4k tokens (~$0.04)` from existing `tokens_used` and `cost_cents`.
   - Output bullet: first 80 chars of the last assistant message in `job_events`.
3. **next_actions:** mined from the last 3 assistant chunks for imperative-form sentences. Cap 3.
4. **artifacts:** files from `steps`, spawned sub-jobs from event metadata.
5. **resume_hint:** `seed_instruction = next_actions[0]`, `inherit_provider = true`, `inherit_tags = true`.

## Failure modes

| Failure | Behavior |
|---|---|
| Recap timeout during job state transition | State transition still commits; recap is enqueued in `scratchpad` (key: `recap.pending`) for a background pass |
| LLM provider unavailable | Fall back to heuristic |
| Resume of a non-existent or `running` job | `409 Conflict` — only resume from terminal jobs |
| Resume of a `failed` job inherits the failure context as a leading bullet in the new job's first user message | by design — gives the model the "why we tried again" |

## Slicing plan

| Slice | What | Surfaces | Tests |
|---|---|---|---|
| **J1.1** | `recaps` table on `jobs.db` (encrypted) + `parent_job_id`/`resumed_from_recap_id` on `jobs` | daemon | Unit: encryption round-trip, idempotency on `(subject_id, last_event_seq)` |
| **J1.2** | Heuristic job recap generator wired into terminal-state hook | daemon | Unit: synthetic JobRecord → expected recap shape; timeout paths |
| **J1.3** | `POST/GET /v1/recap` (kind=job), `POST /v1/resume` (kind=job) | daemon `serve.rs` | HTTP integration; resume creates new job with parent link |
| **J1.4** | vibeui BackgroundJobsPanel recap row + Resume button | vibeui | RTL: render, click, optimistic update |
| **J1.5** | Settings toggles for auto-recap & generator | vibeui SettingsPanel | RTL |
| **J1.6** | REPL `/recap job` and `/resume job` | vibecli main.rs | REPL integration |
| **M1.2** | Mobile NotificationService recap category + JobRecapView | vibemobile | Widget tests + stub-daemon integration |
| **W1.2** | watchOS complication + Tile, Wear Tile, slim `RecapView` | both watch apps | SwiftUI snapshot, Compose preview |
| **J2.x** (deferred) | Counsel persistence + recap | daemon + vibeui | follow-on |
| **J3.x** (deferred) | Background-fetch path on mobile (true push) | vibemobile + daemon | follow-on |

## Open questions

1. **Webhook recap default.** Off is safer; on is ergonomic for users who already trust their webhook. v1 ships off; revisit after a release.
2. **Cost field on recap.** `cost_cents` is already on `JobRecord` — surface it in the recap, or only in the panel? Surface it (truth in numbers).
3. **Multi-step resume.** "Resume from step 7 of 12" is appealing for failed mid-job runs. Skipped in v1 because it requires a step-level checkpoint format we don't have. Tracked as `J4`.
4. **Counsel session ID overlap.** If/when Counsel persists, do its recaps live in the job-recap table or a sibling? Open until Counsel persistence design lands.
