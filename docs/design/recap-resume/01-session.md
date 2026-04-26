# 01 — Session Recap & Resume

**Scope:** chat / agent conversations
**Parent:** [`README.md`](./README.md)
**Status:** Draft · 2026-04-26

---

## What's there today

Grounded in a code survey on `main` at `b1e28ad1`:

- **`SessionStore`** — `vibecli/vibecli-cli/src/session_store.rs` — SQLite-backed (`~/.vibecli/sessions.db`), tables: `sessions`, `messages`, `steps`, `messages_fts`. Session row already has a `summary` column and a `parent_session_id` (branching support, Cursor-4-style). Unencrypted; relies on file permissions.
- **`--resume SESSION_ID`** flag on the CLI — `main.rs:2444`. Already wired: `main.rs:4041`, `main.rs:4051–4063`. Loads a session and re-runs an agent from it. **Bare `--resume` (no `--agent`) currently lists/prints session info** (`main.rs:4052`).
- **`/resume <id_prefix>` REPL command** — `main.rs:4458–4492`.
- **Daemon routes:** `GET /sessions`, `GET /sessions.json`, `GET /view/:id`, `GET /share/:id` (read-only HTML view). No `recap` route, no JSON resume route.
- **vibeui:** `ChatTabManager.tsx` persists chat history to `localStorage` under `vibecody:chat-history` (max 50 sessions). `restoreSession()` (line 290) loads a history entry into a new tab. Tabs themselves are ephemeral — not auto-restored on app reload.
- **vibemobile:** `ChatScreen` already accepts `resumeMachineId` / `resumeSessionId` / `resumeTask` constructor params and calls `/mobile/sessions/{id}/context` to fetch history fresh. No local cache.
- **vibewatch (watchOS + Wear):** `/watch/sessions/{id}/messages` and `/watch/active-session` endpoints exist; both apps fetch fresh on view open. No cache, no recap surface.

So **resume exists in skeleton form on the CLI** and the data is already there. The work is: standardize, generalize across clients, and add **recap** as a first-class artifact.

## Goals

1. Generate a structured recap when a session reaches a terminal state (completed agent run, explicit `/end`, idle timeout, tab close), or on demand.
2. Make recap the universal handoff artifact between clients — opening a session on any surface starts with the recap, transcript fetched on demand.
3. Auto-resume the last session on app startup, opt-in.
4. Heuristic recap by default; LLM recap when the user asks or the session crosses a length threshold.
5. Preserve branching — resume can target a specific message ID, forking a new session, exactly as `fork_session` does today.

## Non-goals

- Replacing the existing per-tab localStorage history in vibeui (kept as a UI-side cache; recap is the durable cross-device artifact).
- Resuming Counsel/Arena/Diffcomplete from a session recap — those have their own scopes (docs `02` and `03`).
- Server-side conversation summarization across multiple sessions (that's memory, not recap).

## Triggers

| Trigger | Default | Notes |
|---|---|---|
| Agent task completes | Auto-recap (heuristic) | Status moves to `complete` or `failed` in `sessions` table |
| User runs `/recap` in REPL | Recap (heuristic, or LLM with `/recap --llm`) | Idempotent: returns existing recap if no new messages since last recap |
| User closes a vibeui chat tab | Auto-recap (heuristic) | Already persists to localStorage; new flow also POSTs `/v1/recap` |
| Session idle > 30 min | Auto-recap (heuristic) | Configurable in Settings; off by default for privacy |
| Tab reload / app restart | No auto-recap; offer auto-resume of *last* session | "Last session" = most recent recap with `resume_hint` set |

**Idempotency rule:** generating a recap when one already exists for `(subject_id, latest_message_id)` returns the existing recap unchanged. To force regeneration, the user passes `force=true` (REPL: `/recap --regen`).

## Data model

Inherits the cross-cutting `Recap` shape from [README.md](./README.md). Session-specific fields on `ResumeHint`:

```rust
pub struct ResumeHint {
    pub target: ResumeTarget::Session(SessionId),
    pub from_message: Option<MessageId>,   // resume cursor; None = end of transcript
    pub seed_instruction: Option<String>,  // pre-fill the next prompt input
    pub branch_on_resume: bool,            // if true, resume forks a new session_id
}
```

`branch_on_resume` defaults to `true` when `from_message` is set (forking from mid-conversation), `false` when resuming from the tail.

## Storage

New table on `sessions.db`:

```sql
CREATE TABLE IF NOT EXISTS recaps (
    id TEXT PRIMARY KEY,                    -- ULID
    kind TEXT NOT NULL DEFAULT 'session',   -- always 'session' in this table; kept for cross-store union queries
    subject_id TEXT NOT NULL,               -- session_id
    last_message_id TEXT,                   -- the latest message included; idempotency key
    workspace TEXT,
    generated_at TEXT NOT NULL,             -- ISO-8601
    generator_kind TEXT NOT NULL,           -- 'heuristic' | 'llm' | 'user_edited'
    generator_provider TEXT,
    generator_model TEXT,
    headline TEXT NOT NULL,
    body_json TEXT NOT NULL,                -- bullets + next_actions + artifacts + resume_hint, JSON
    token_input INTEGER,
    token_output INTEGER,
    schema_version INTEGER NOT NULL DEFAULT 1,
    FOREIGN KEY (subject_id) REFERENCES sessions(id) ON DELETE CASCADE
);

CREATE INDEX IF NOT EXISTS idx_recaps_subject ON recaps(subject_id);
CREATE INDEX IF NOT EXISTS idx_recaps_generated ON recaps(generated_at);
CREATE UNIQUE INDEX IF NOT EXISTS uq_recaps_subject_last_msg ON recaps(subject_id, last_message_id);
```

Migration via `maybe_add_table_if_needed()` — same idempotent pattern used elsewhere in `session_store.rs`.

## RPC contract

### `POST /v1/recap`

Generate (or fetch idempotently) a session recap.

```jsonc
// request
{
  "kind": "session",
  "subject_id": "01HK...",
  "force": false,            // re-generate even if up-to-date recap exists
  "generator": "heuristic",  // "heuristic" | "llm" | "auto" (auto = heuristic if < 20 turns, else llm)
  "provider": null,          // optional override; defaults to user's selected provider
  "model": null              // optional override
}

// response: 200 OK with the Recap JSON shape from README.md
```

Errors: `404` if `subject_id` not found, `409 force=false` and recap is current, `503` if LLM requested and provider unavailable.

### `GET /v1/recap/:id` and `GET /v1/recap?kind=session&subject_id=…`

Standard fetch / list. List supports `?limit=` and `?cursor=` (newest-first pagination on `generated_at`).

### `POST /v1/resume`

```jsonc
// request
{
  "from_recap_id": "01HMRECAP...",   // OR from_subject_id + kind
  "from_subject_id": null,
  "kind": "session",
  "from_message": null,              // overrides recap.resume_hint.from_message
  "seed_instruction": null,          // overrides recap.resume_hint.seed_instruction
  "branch": null,                    // overrides recap.resume_hint.branch_on_resume
  "client": "vibeui"                 // for telemetry / activity tracking
}

// response 200
{
  "handle": "01HRESUME...",          // poll via GET /v1/resume/:handle
  "resumed_session_id": "01HK...",   // = original if !branch, new ULID if branch
  "primed_message_count": 42,        // messages reloaded into context
  "ready": false                     // true once context is loaded and provider is warm
}
```

`GET /v1/resume/:handle` polls readiness; `ready: true` means a subsequent `/chat` or `/agent` call can be issued.

### REPL extensions

- `/recap` — print the most recent recap for the active session, generating one if absent (heuristic).
- `/recap --llm` — force LLM regeneration.
- `/recap <session_id>` — print recap for any session.
- `/recap --edit` — open `$EDITOR` on the recap headline + bullets + next_actions; saves as `generator: user_edited`.
- `vibecli --resume <id> --from-recap` — when entering REPL, prime context from recap rather than full transcript (cheaper on long sessions).

## Per-surface UX

### vibecli REPL / TUI

- **End-of-agent print:** when an `--agent` task terminates, print the recap headline + bullets + next_actions inline, before the prompt returns. (Suppressed with `--no-recap` or `VIBECLI_NO_RECAP=1`.)
- **Session-list view** (`vibecli --resume` with no id): each row shows headline + relative time + provider; "r" key resumes, "v" views.
- **REPL `/recap`** as above.

### vibeui (Tauri)

- **Chat tab footer:** when a tab is closed via the X, a tiny non-blocking toast confirms "Recap saved — restore from History". The History panel (already exists, line ~290 in `ChatTabManager.tsx`) gains a recap card per entry instead of just the title.
- **Tab restore flow:** `restoreSession()` is extended — on click, the tab opens with a **Recap card pinned at the top of the transcript** (collapsible, default open). The card has a "Resume from here" button that calls `/v1/resume` with the recap's hint.
- **Settings → Sessions** (new subsection in `SettingsPanel.tsx`):
  - "Recap on tab close" — default on
  - "Recap on idle (after N min)" — default off, N=30 if enabled
  - "Recap generator" — heuristic (default) | LLM (with provider picker)
  - "Auto-resume last session on startup" — default off
- **Reuses existing tokens** from `vibeui/design-system` — no new design primitives. Recap card uses the same panel/card pattern as the existing History entries.

### vibemobile (Flutter)

- **ChatScreen header:** when `resumeSessionId` is set, fetch `/v1/recap?kind=session&subject_id=…&limit=1` *before* `/mobile/sessions/{id}/context`. Render the recap card at the top of the transcript; transcript loads underneath.
- **Sessions tab:** the existing list view (currently shows session title + machine + time) gains a 1-line headline from the recap. Tap → opens ChatScreen with recap in resume hint.
- **No mobile-side recap composition.** Mobile reads recaps; it doesn't generate them. (Consistent with: mobile is a thin client over the daemon.)

### vibewatch (watchOS + Wear OS)

- **New `RecapView`** (SwiftUI + Compose). Reached from the session-picker via a long-press on a session row. Shows: headline, 3 bullets max, "Continue on phone" button.
- **"Continue on phone"** posts `/mobile/sessions` to register a handoff (this endpoint already exists), then surfaces a notification on the paired phone.
- **No watch-side recap composition.** Same constraint as mobile.
- **Cryptography reminder:** watch device keys remain **P-256 ECDSA**. No new auth primitives introduced. (Per `CLAUDE.md`.)

## Heuristic recap algorithm

For sessions where heuristic is sufficient (fast, free, offline):

1. **Headline:** the first user message, trimmed to ≤ 80 chars, with trailing punctuation stripped. If first message is a `/command`, use the first prose user message after it.
2. **Bullets** (3–5):
   - If `steps` table has tool calls: one bullet per *distinct* tool name with count, e.g. "Ran `cargo test` (3×)".
   - Files touched: collect from tool input/output summaries; one bullet per file path with verb (read / wrote / deleted) inferred from tool name.
   - If the agent ended with status `failed`: a bullet starting "Stopped: " with the failure reason.
3. **next_actions:** parse the last 3 assistant messages for imperative-form sentences ("Next, …", "TODO:", "Should also …"). Cap at 3.
4. **artifacts:** unique file paths from `steps` (kind: File), spawned job IDs from message metadata (kind: Job).
5. **resume_hint:** `from_message = last_message_id`, `seed_instruction = next_actions[0]` if present.

The heuristic is deterministic, runs in <50ms on a 200-message session, and produces a recap that is *usable* even without an LLM. The LLM path is an upgrade, not a requirement.

## LLM recap prompt

```
You are summarizing a coding-assistant conversation for the user who had it.
Write in second person ("you did X"), concise, no fluff. Output JSON only:

{
  "headline": "<single line, <=80 chars, no trailing period>",
  "bullets": ["<3-7 bullets, each <=120 chars, what *happened*>"],
  "next_actions": ["<0-3 imperative bullets, what to do next>"]
}

Constraints:
- Do not invent details. If you didn't see it in the transcript, don't write it.
- Reference files by path, not by description.
- Don't restate the user's instructions; describe what was done about them.
```

Provided context: last 50 messages + all `steps` rows (deduplicated tool calls). Token budget cap: 8k input / 400 output. Provider = user's currently selected provider (no silent fan-out).

## Failure modes

| Failure | Behavior |
|---|---|
| LLM provider down during recap | Fall back to heuristic; mark generator as `heuristic` and add a footer bullet "(LLM unavailable; auto-summary)" |
| `sessions.db` locked (concurrent writer) | Retry 3× with exponential backoff (50ms, 200ms, 800ms); surface error, recap is non-blocking |
| Recap generation timeout (>10s LLM) | Cancel, fall back to heuristic |
| Session has 0 messages | Skip recap; not an error |
| User edits recap, then session continues | New messages → new recap on next trigger; user-edited recap remains accessible via list, marked `superseded_by` |

## Telemetry

Local-only counters in `~/.vibecli/jobs.db` (`scratchpad` table, key `recap.metrics`):

- `recap.generated.heuristic`, `recap.generated.llm`, `recap.generated.user_edited`
- `recap.resume.invoked`, `recap.resume.branched`
- `recap.heuristic.ms.p50`, `recap.heuristic.ms.p99`

No off-device transmission.

## Slicing plan

| Slice | What | Surfaces | Tests |
|---|---|---|---|
| **F1.1** | `recaps` table migration + `Recap`/`RecapKind` Rust types + heuristic generator | vibecli daemon | Unit: heuristic on synthetic transcripts; idempotency on `(subject_id, last_message_id)` |
| **F1.2** | `POST/GET/PATCH/DELETE /v1/recap` routes (Session kind only) | daemon `serve.rs` + Tauri wrapper command | HTTP integration tests; auth enforcement |
| **F1.3** | `POST /v1/resume` + `GET /v1/resume/:handle` | daemon | End-to-end: generate recap → resume → verify primed message count |
| **F1.4** | REPL `/recap` and `/recap --edit`; end-of-agent auto-print | vibecli main.rs | REPL behavior tests |
| **F2.1** | vibeui Settings → Sessions toggles | vibeui SettingsPanel | RTL: toggle persists to localStorage |
| **F2.2** | Recap card pinned to restored tab | vibeui ChatTabManager | RTL: render + "Resume from here" calls Tauri command |
| **F2.3** | Auto-recap on tab close | vibeui ChatTabManager | RTL: close fires `diffcomplete_generate`-style Tauri command (renamed `recap_generate`) |
| **M1.1** | Flutter ChatScreen recap header + recap-aware sessions list | vibemobile | Widget tests; one integration test against a stub daemon |
| **W1.1** | watchOS `RecapView` + Wear `RecapScreen` | both watch apps | Snapshot test (SwiftUI), Compose preview |
| **F3.1** | Auto-resume-last-session on startup | vibeui App.tsx | E2E (manual sign-off) |

Each slice ships independently and each is `cargo test --workspace` + `npm run -w vibeui test` green before merge.

## Open questions

1. Should the recap include the *first* assistant message verbatim as a "what the model said it'd do" reference? (Probably not — invites drift between intent and reality.)
2. Edit-conflict behavior: if two clients open the same session and both regenerate the recap, last-write-wins is fine for v1. Multi-device merge is out of scope.
3. Should idle-timeout triggers be per-tab or per-daemon? Per-tab is cleaner but requires the frontend to push activity heartbeats. Defer until F2 is in user hands.
