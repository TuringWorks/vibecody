---
layout: page
title: Recap & Resume
permalink: /recap/
---

# Recap & Resume

> When you close a chat tab, finish a job, or restore a session days later, VibeCody pins a four-block recap above the transcript: **headline · what happened · next actions · artifacts**. One click on **Resume from here** reopens the work where you left off — with the right model, the right context, and the right pending tasks.

The recap surface is built around two principles:

1. **Recaps are written, not generated on read.** The heuristic generator runs at *write time* (tab close, job complete, idle timeout) so reading is always cheap and offline.
2. **Resume is explicit.** Recaps are surfaces; pressing the Resume button is the action. Nothing auto-resumes without a click.

---

## Surfaces

| Surface | What it shows | Where |
|---|---|---|
| **Desktop** (VibeUI) | Inline `RecapCard` above the chat transcript when a session is restored | `vibeui/src/components/RecapCard.tsx` |
| **CLI** | `/recap` slash command in the REPL | F1.4 |
| **Mobile** (Flutter) | Recap header on the sessions list + per-session detail | M1.1 |
| **watchOS** | `RecapView.swift` — read-only glance | W1.1 |
| **Wear OS** | `RecapScreen.kt` — read-only glance | W1.1 |

The **daemon** (`vibecli serve`) is the source of truth. All five surfaces consume the same `Recap` shape over `/v1/recap` (desktop) or `/watch/sessions/:id/recap` (watch read-only).

---

## How recaps get written

Three triggers ship today:

1. **Tab close** (F2.3) — closing a chat tab fires `auto_recap_on_close` if the tab carried > N messages. The heuristic generator runs in-process; the recap lands on `sessions.db` before the tab unmounts.
2. **Idle timeout** (F2.1, opt-in) — after the configured idle window (default 30 min), the daemon writes a recap for any session that doesn't already have a fresh one.
3. **Job complete** (J1.2) — `JobManager::mark_terminal()` auto-generates a job-kind recap on every terminal transition (Complete / Failed / Cancelled). The recap is best-effort: a generation failure is logged but never aborts the job.

The user can also write recaps manually:

```bash
# CLI
vibecli repl
> /recap                                    # write a recap for the current session

# HTTP
curl -X POST http://127.0.0.1:7878/v1/recap \
     -H 'content-type: application/json' \
     -d '{"kind":"session","subject_id":"<sid>","generator":"heuristic","force":true}'
```

`force: true` overwrites any existing recap for the same `(subject_id, last_message_id)` pair; without it the request 409s when a recap is already in place.

---

## The Recap shape

```typescript
interface Recap {
  id: string;
  kind: "session" | "job";
  subject_id: string;
  generator: { type: "heuristic" } | { type: "llm", provider, model } | { type: "user_edited" };
  headline: string;                  // ≤ 80 chars, trailing punctuation stripped
  bullets: string[];                 // "What happened" — short imperative phrases
  next_actions: string[];            // imperative sentences pulled from the last message
  artifacts: { kind, label, locator }[];
  resume_hint?: ResumeHint;
  last_message_id?: number;          // session: msg.id · job: event.seq
  created_at: number;                // unix seconds
}
```

The shape is identical for sessions and jobs. The `kind` field tells consumers which store the `subject_id` belongs to (`sessions.db` vs `jobs.db`).

---

## Resume

Recap → Resume is a single Tauri command (desktop) / HTTP `POST /v1/resume` (anywhere):

```bash
curl -X POST http://127.0.0.1:7878/v1/resume \
     -H 'content-type: application/json' \
     -d '{"from_recap_id":"<recap-id>"}'
```

The daemon dispatches by recap kind:

- **Session resume** → opens a new chat tab seeded with the session's last messages, the recap's `seed_instruction`, and the same provider/model.
- **Job resume** → spawns a fresh job whose `parent_job_id` and `resumed_from_recap_id` link back to the source. The new job inherits the parent's workspace + approval policy.

`POST /v1/resume` accepts either:

- `from_recap_id: <id>` — the daemon probes both stores to find the recap and dispatches accordingly.
- `from_subject_id: <id>` + explicit `kind: "session" | "job"` — for clients that already know which store owns the subject.

The `branch_on_resume` flag (in the recap's `resume_hint`) lets the user open a new branch from the resume point instead of continuing in the same chain — useful for "what if" exploration.

---

## Generators

Two generators ship; only one is GA today:

| Generator | Status | When to use |
|---|---|---|
| **heuristic** | ✅ GA | The default for everything. Instant, offline, deterministic. |
| **llm** | ⚠️ experimental (returns 501) | Will produce richer recaps once F2.4 lands. Off by default; experimental flag. |
| **user_edited** | ✅ GA | After a user manually edits a heuristic recap, the `generator` field flips to `user_edited` so future tooling knows it was hand-curated. |

The `/health.features.recap` block lists which generators are GA vs experimental:

```bash
curl http://127.0.0.1:7878/health | jq '.features.recap'
```

```json
{
  "available": true,
  "transport": "daemon-http",
  "routes_prefix": "/v1/recap",
  "generators_ga": ["heuristic"],
  "generators_experimental": ["llm"],
  "kinds": ["session", "job"]
}
```

Clients **must not** pre-select an experimental generator without a feature flag — see [feature-flags](../feature-flags/) for the canonical pattern.

---

## Storage

| Recap kind | Database | Path |
|---|---|---|
| Session | `sessions.db` (recaps table) | `~/.vibecli/sessions.db` |
| Job | `jobs.db` (recaps table) | `~/.vibecli/jobs.db` |

The dual-table approach (rather than a single `recaps.db`) keeps recap lifetime tied to the subject's lifetime. Delete a session → the session's recaps go with it. Same for jobs.

---

## CLI

```bash
# Inside the REPL
> /recap                              # write a recap for the current session
> /recap --force                      # overwrite an existing one for the same last-msg
> /recap show                         # render the latest recap for the current session
```

For job recaps:

```bash
# List recaps for a job
curl 'http://127.0.0.1:7878/v1/recap?kind=job&subject_id=<job-sid>'
```

---

## Configuration

```toml
# ~/.vibecli/config.toml

[recap]
# Auto-write a recap when a chat tab closes with this many or more messages.
# 0 disables; default 8.
auto_on_close_min_messages = 8

# Idle window before the daemon auto-writes a recap for an open session.
# 0 disables; default "30m".
auto_on_idle = "30m"

# Default generator for the auto path. Currently the only GA option.
generator = "heuristic"

# When true, the desktop UI auto-pins a RecapCard above any restored
# session that has at least one recap. Default true.
pin_on_restore = true
```

---

## Troubleshooting

### Recap card doesn't appear after restoring a session

Check whether a recap was ever written for that session:

```bash
curl 'http://127.0.0.1:7878/v1/recap?kind=session&subject_id=<sid>' | jq '.recaps | length'
```

If the count is 0: the session was closed without crossing the auto-write threshold (default 8 messages). Manually write one with `> /recap` in the REPL.

### "kind not supported in F1.2"

The current daemon only honors `kind: "session"` on POST. Job recaps are written *automatically* by `JobManager::mark_terminal` (J1.2) — there's no client-facing "create a job recap" surface today; the daemon does it for you when a job ends.

### "generator not implemented"

You've requested `generator: "llm"`. This generator is experimental (returns 501) until F2.4 lands. Use `generator: "heuristic"` instead, or wait for the LLM path.

### Resume opens a fresh tab without the recap context

The Resume button passes the `recap_id` to `/v1/resume`. If the new tab doesn't seed correctly, check the daemon log under target `vibecody::recap` — the resume helper logs the chosen path (session vs. job), the `seed_instruction`, and any context-load failures.

### Two recaps for the same session

You ran `POST /v1/recap` twice without `force: true`, but somehow both succeeded. Check the unique index — it should reject duplicates on `(subject_id, last_message_id)`. If it doesn't, re-create the index by running the migration in `session_store.rs::ensure_recaps_schema`.

---

## Observability

Every recap operation emits structured `tracing` events under the `vibecody::recap` target:

```bash
RUST_LOG=vibecody::recap=info vibecli serve
```

Examples:

```
INFO vibecody::recap: recap.post: persisted heuristic session recap
  subject_id=ses_a1b2c3 recap_id=rec_x9y8 bullets=4 next_actions=2 artifacts=1

WARN vibecody::recap: recap.post: insert failed
  subject_id=ses_a1b2c3 error="UNIQUE constraint failed"
```

User content (headline, bullets, next-actions) is **not** logged at any level — only counts and stable IDs. No telemetry leaves your machine.

---

## Cross-client consistency

| Client | Recap surface |
|---|---|
| **VibeUI (desktop)** | `RecapCard` pinned above the chat transcript on session restore; auto-write on tab close (F2.3); manual write in any tab. |
| **VibeCLI** | `/recap` slash command; HTTP API for scripting. |
| **VibeMobile** | Recap header in the sessions list (M1.1); per-session detail card; Resume button opens the resume flow on the daemon. |
| **VibeWatch (watchOS)** | `RecapView` — glance-only read of the freshest recap (W1.1). |
| **VibeWatch (Wear OS)** | `RecapScreen` — glance-only read of the freshest recap (W1.1). |
| **IDE plugins** | Not surfaced in the IDE chrome today; consume via `/v1/recap` HTTP if needed. |
| **Agent SDK** | `/v1/recap` HTTP routes available; no SDK abstraction today. |

The daemon is the source of truth for the `Recap` shape. Watch clients are explicitly read-only — they never trigger generation, only display the freshest recap that the desktop or mobile flow already wrote.

---

## Related

- **Design docs:** [`docs/design/recap-resume/`](https://github.com/TuringWorks/vibecody/blob/main/docs/design/recap-resume/) — Per-kind specs (session, job, diffcomplete) + the cross-cutting `Recap` shape.
- **Source:** `vibecli/vibecli-cli/src/recap.rs` (heuristic generator) · `vibeui/src/components/RecapCard.tsx` (UI) · `vibemobile/lib/screens/sessions_screen.dart` (mobile).
- **Diffcomplete:** [`/diffcomplete/`](../diffcomplete/) — uses the same `Recap` shape for AI-edit summaries.
