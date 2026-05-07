---
layout: page
title: Chat Tabs
permalink: /chat-tabs/
---

# Chat Tabs

The chat tab manager is the heart of VibeCody's conversation UX in VibeUI and VibeCLI App. It owns multiple concurrent chat sessions, per-tab provider overrides, session history, recap-on-close, and Watch active-session sync. This page documents the user-facing surface; the cross-client `Recap` shape lives in [`docs/recap`](./recap.md).

---

## What you get

- **Multiple tabs**, each with its own message history, provider, and (optional) agent loop.
- **Auto-named tabs** — every new tab gets a unique adventure name from a 30-entry pool (refreshed from the daemon's `get_adventure_names` on mount).
- **Inline rename** — double-click a tab title to edit it; Enter saves, Escape cancels.
- **Per-tab provider** — override the global provider for one conversation without disturbing the others. The global top-bar selector resets every tab back to the global default; click "reset" on a tab to drop a manual override.
- **History** — closing a tab with messages auto-saves it. The History panel restores any past session into a new tab.
- **Recap-on-close** — when enabled in Settings → Sessions (default on), closing a tab triggers a recap generation. The recap pins to the restored tab on next open.
- **Watch sync** — when an Apple Watch / Wear OS companion switches its active session, VibeUI follows automatically (Google Docs-style).

---

## Keyboard navigation

The tab strip is a proper [WAI-ARIA tablist](https://www.w3.org/WAI/ARIA/apg/patterns/tabs/). Focus the tab strip and:

| Key | Action |
|---|---|
| `←` / `→` | Move to previous / next tab (wraps) |
| `Home` | Jump to first tab |
| `End` | Jump to last tab |
| `Enter` (on a focused tab) | Select that tab |

Tab titles are also focusable spans — double-click or press Enter on a focused title to start an inline rename.

---

## History

History is stored in `localStorage` under key `vibecody:chat-history`, capped at **50 entries**. Each entry has:

| Field | Meaning |
|---|---|
| `id` | Stable session id, reused across saves |
| `title` | First user message (truncated) on first save, or the tab title at close |
| `provider` | The provider active when saved |
| `messages` | The full transcript |
| `savedAt` | UNIX ms — most-recent first |
| `recapSubjectId` | F2.2 — daemon-side `subject_id` of the recap, when one exists |

**Save once → updates in place.** Saving a tab a second time replaces its existing history entry rather than stacking duplicates. Same when closing a previously-saved tab.

**Restoring** opens a new tab pre-populated with the transcript and pins the recap card (if the entry has `recapSubjectId` and the daemon's `recap_get_for_session` succeeds). Closing the restored tab updates the same entry.

**Clear All** deletes every history entry. Tab → history bindings are invalidated, so a subsequent Save creates a fresh entry rather than reviving deleted ids.

---

## Recap on tab close

When the user closes a tab with at least one message and the **Recap on tab close** preference is enabled:

1. The tab's transcript is persisted to history.
2. The daemon's `recap_generate` is called with `subject_id = tab.id`.
3. On success, the history entry gains `recapSubjectId = tab.id`.
4. On failure (daemon offline, `subject_id` not in `sessions.db`), the close completes silently — no banner, no retry. The history entry stays without a `recapSubjectId`, so a future restore will simply not show a recap card.

Toggle this in Settings → Sessions. The preference is read from `localStorage:vibeui-sessions.recapOnTabClose`; defaults to `true` if the key is missing or corrupt.

---

## Inline error banner

Most failures degrade silently — the transcript is always still intact in the active tab, and partial network availability is normal. The exception is **Resume from here** on a recap card: if `recap_resume_session` rejects, an inline alert banner appears above the chat (`role="alert"`, auto-dismissed after 6 seconds, or via the **Dismiss** button). The banner reads:

> Couldn't resume from recap — the daemon may be offline. Your messages are still here.

This is the only user-visible failure surface today; quieter failures (recap-fetch on restore, recap-generate on close) intentionally do not surface a banner because they are not blocking the user from continuing.

---

## /health declaration

`features.chat_tabs` declares the feature for cross-client gating:

```json
{
  "available": true,
  "transport": "tauri-desktop",
  "history_key": "vibecody:chat-history",
  "history_cap": 50
}
```

This is a **declaration**, not a probe — there is no daemon-side state to check. Clients gating UI on the existence of chat-tab support read this entry; mobile / watch clients ignore it because they have no tabbed UI.

---

## Cross-client behaviour

| Client | Tab strip | History | Recap-on-close |
|---|---|---|---|
| **VibeUI / VibeApp (desktop)** | ✅ | ✅ localStorage | ✅ |
| **VibeMobile** | ❌ single-session UI | n/a | n/a — uses `/v1/recap` directly |
| **VibeWatch (watchOS / Wear OS)** | ❌ single-session UI | n/a | n/a |
| **IDE plugins** | ❌ — chat is per-editor pane | n/a | n/a |

VibeUI and VibeApp share the exact same `ChatTabManager.tsx`. The implementation is intentionally desktop-only — small-screen clients use a single-session model.

---

## Watch active-session sync

When the Apple Watch or Wear OS companion app switches its active session, the desktop subscribes via `useWatchActiveSession` and calls `setActiveTabId` if the corresponding tab is open. The reverse direction (desktop → watch) is owned by the watch companion's session-list refresh — see [`docs/watch-integration`](./WATCH-INTEGRATION.md).

If the active session id from the watch doesn't match an open tab, the call is a no-op — VibeUI does NOT auto-restore from history on a watch trigger, because the session may not be in the user's history yet.

---

## Troubleshooting

### "Closing a tab loses my messages"

Closing the **last** tab is blocked. Closing any other tab auto-saves it to history (if it has any messages). Open History to restore.

### "I see duplicate sessions in History"

Restoring a session and saving updates the same entry. If you see duplicates, the original was deleted between restore and save — fixable by clicking Clear All and re-saving the tabs you care about.

### "Recap card never appears on a restored session"

The history entry needs `recapSubjectId` for the card to render. Older history (saved before recap-resume shipped) lacks this field. Close the tab again with **Recap on tab close** enabled to backfill.

### "Watch active-session changes don't switch my tab"

The watch sends a session id; VibeUI only switches if a tab with that id is already open. History entries are not auto-restored on a watch trigger. Open History and Restore manually if needed.

---

## Related

- **Recap & Resume:** [`docs/recap`](./recap.md) — the cross-client recap shape and `/v1/recap` API
- **Watch integration:** [`docs/watch-integration`](./WATCH-INTEGRATION.md) — pairing + session sync
- **Source:** `vibeui/src/components/ChatTabManager.tsx` (804 LOC) · tests in `vibeui/src/components/__tests__/ChatTabManager.bdd.test.tsx`
