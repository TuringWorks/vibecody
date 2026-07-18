---
layout: page
title: Sessions
permalink: /sessions/
---

The Session Browser is the panel for inspecting, replaying, forking, and deleting your past VibeCody conversations. Every interactive vibecli or vibecoder session writes a JSONL trace to `<workspace>/.vibecli/traces/`; the panel reads from that directory directly.

This page covers the desktop session browser. The chat-tab side of session lifecycle (auto-save, recap-on-close) is documented in [`docs/chat-tabs`](./chat-tabs.md), and the cross-client recap shape lives in [`docs/recap`](./recap.md).

---

## What's stored on disk

For each session, three files in `<workspace>/.vibecli/traces/`:

| File | Contents | Required? |
|---|---|---|
| `<session-id>.jsonl` | One trace entry per line — tool calls, outputs, timestamps | ✅ always written |
| `<session-id>-messages.json` | Reconstructed user / assistant messages | optional |
| `<session-id>-context.json` | Workspace context snapshot | optional |

The `.jsonl` is the source of truth. The two sidecars are convenience caches for the UI — the browser falls back to reconstructing messages from the JSONL when `-messages.json` is missing.

`session-id` is typically `<adventure-name>-<unix-ts>` from the chat tab manager, or the literal id assigned by an external entry point.

---

## What you can do

### List + search

The Sessions tab lists everything in the trace directory, newest-first. The search box filters by **session id** (case-insensitive substring). Title-based search isn't useful because sessions don't have human-meaningful titles at the trace level — chat-tab "titles" are a UI-only concept stored in `localStorage`.

### Replay

Click a session row → the Replay tab loads `get_session_detail` and renders messages one at a time with **Prev** / **Next** stepping and a `Step N / M` indicator. Each message is colour-coded by role:

| Role | Colour |
|---|---|
| user | accent |
| assistant | success-green |
| system | warning-amber |

### Fork

Fork copies the session's `.jsonl` and any sidecars to a new id `fork-<original>-<unix-ts>`. The original is untouched. Useful when you want to branch off a long conversation without losing the canonical history.

### Delete (two-click confirm)

Delete is a **two-click confirmation** to prevent accidents:

1. First click arms the delete — the button label switches to **"Confirm?"** and its `aria-label` updates to *"Confirm delete session X — second click commits"*.
2. Second click within 5 seconds commits the delete.
3. **Auto-cancel** after 5 seconds, on click elsewhere, or on Escape.

The delete is destructive on disk — the `.jsonl`, `-messages.json`, and `-context.json` files are removed. There's no recycle bin. If you want a soft-delete pattern, fork first.

### Stats

The Stats tab shows totals (sessions / messages / size) and a top-10 by-size bar chart so you can quickly find sessions to prune in a workspace that's getting large.

---

## /health declaration

`features.sessions` declares the surface:

```json
{
  "available": true,
  "transport": "tauri-desktop",
  "trace_dir": ".vibecli/traces/"
}
```

There is no daemon-side state to probe — sessions are filesystem-backed in the user's workspace. Cross-client gating reads this entry only to confirm the desktop UI is shipping; mobile/watch clients have their own session APIs.

---

## Observability

Backend operations emit structured tracing events under `vibecody::sessions`:

```bash
RUST_LOG=vibecody::sessions=info vibecli serve
```

Events:

```
INFO  vibecody::sessions: session.delete
  session_id=sess-alpha-1700000000 trace_size=12048

INFO  vibecody::sessions: session.fork
  parent_id=sess-alpha-1700000000 new_id=fork-sess-alpha-1700000000-1714900000

WARN  vibecody::sessions: session.delete.rejected: path traversal
  session_id=../etc/passwd
```

The `path traversal` rejection is a security-relevant signal: any client that sends a session_id with `..`, `/`, or `\` is either misbehaving or under attack. Surface this in operator dashboards.

Session contents are **never** logged.

---

## Accessibility

- The status banner uses `role="status"` for info events (Session deleted, Forked → ...) and `role="alert"` with `aria-live="assertive"` for failures, so AT users hear destructive failures immediately and routine confirmations get a polite announcement.
- Both the metadata row and the explicit Fork/Delete buttons are keyboard-reachable. The metadata row activates the Replay tab on Enter or Space.
- Delete-button `aria-label` carries the confirmation state ("requires second click to confirm" → "second click commits") so screen-reader users can hear that the destructive action is two-stage without seeing the button text change.

---

## Cross-client behaviour

| Client | Sessions UI | Trace dir |
|---|---|---|
| **VibeCoder / VibeApp** | Full browser | reads `<workspace>/.vibecli/traces/` |
| **VibeMobile** | List + read-only replay (different shape — uses recap) | uses `/v1/recap` for summaries |
| **VibeWatch** | Active-session indicator only | n/a |
| **IDE plugins** | None | n/a |

The browser is desktop-only. Mobile and watch clients use a different model — they consume recap summaries via `/v1/recap` rather than raw trace files.

---

## Troubleshooting

### "No sessions found in .vibecli/traces/"

Either you haven't run any chat sessions yet, or you're pointing the panel at the wrong workspace. The workspace input at the top of the panel defaults to `.` (current working directory of the daemon). Type the absolute path of your project root and click Refresh.

### "Session X not found"

You're trying to delete or replay a session that no longer exists on disk. Click Refresh to reload the list, or check the trace directory directly:

```bash
ls <workspace>/.vibecli/traces/
```

### "Invalid session ID"

The daemon rejected an id containing `..`, `/`, or `\`. This is a path-traversal guard — sessions from a healthy client should never trigger it. If you're seeing this in normal usage, check for renamed `.jsonl` files in the trace directory.

### "Replay shows assistant messages with [tool] prefixes"

The session has no `-messages.json` sidecar so the daemon is reconstructing messages from the JSONL trace. Tool calls show as `[tool_name] <input_summary>`. If the messages sidecar exists but is corrupt, delete it and replay will regenerate from JSONL.

### "Fork created a session that immediately disappears"

Forks land in the same trace directory and should appear after Refresh. If they don't, check disk space — the fork copies the entire JSONL and may fail silently mid-copy on a full disk. Run `df -h` and verify writable permissions on `<workspace>/.vibecli/traces/`.

---

## Related

- **Chat Tabs:** [`docs/chat-tabs`](./chat-tabs.md) — auto-save / recap-on-close lifecycle
- **Recap & Resume:** [`docs/recap`](./recap.md) — cross-client summary shape
- **Source:** `vibecoder/src/components/SessionBrowserPanel.tsx` (562 LOC) · backend `vibecoder/src-tauri/src/commands.rs` (`list_sessions`, `get_session_detail`, `delete_session`, `fork_session`)
- **Tests:** `vibecoder/src/components/__tests__/SessionBrowserPanel.bdd.test.tsx` (9 BDD scenarios)
