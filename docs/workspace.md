---
layout: page
title: Workspace
permalink: /workspace/
---

# Workspace

The Workspace is the folder VibeCody operates inside — every file read, every git operation, every agent task is rooted at the active workspace path. Picking a workspace is the first thing you do after launching VibeUI/VibeApp; it's the one input that dominates everything downstream.

This page documents the desktop workspace switcher. The daemon side (`add_workspace_folder`, `get_workspace_folders`) is shared across clients — but mobile / watch clients don't pick a workspace today; they consume sessions from whichever workspace the daemon was launched in.

---

## Picking a workspace

Three entry points:

| Entry | Trigger |
|---|---|
| **System folder picker** | "Open Folder" button in the sidebar empty-state, or `⌘O` / `Ctrl+O` |
| **Recents click** | Click any entry in the "Recent" list shown below the Open Folder button |
| **Programmatic** | A Tauri / SDK client invokes `add_workspace_folder` directly |

All three converge on the same backend path: `add_workspace_folder(path)` validates the path, sets it as the active workspace, and updates the LRU recents.

---

## Validation

`add_workspace_folder` rejects bad input with an explicit error rather than mutating state into a broken configuration:

| Input | Behavior |
|---|---|
| Non-existent path | `Err("Path does not exist: …")` — `workspace.add.rejected: path does not exist` warn event |
| Path is a file | `Err("Path is not a directory: …")` — `workspace.add.rejected: not a directory` warn event |
| Valid directory | Active workspace updated, added to recents (move-to-front) |

This guards the most common footgun — copy-pasting a path that was renamed/deleted while the user wasn't looking. Without validation, every panel that reads `vibeui_workspace` from localStorage would silently fail until the user noticed.

---

## Recents

Recents are an LRU list of the last **10** workspaces, persisted in `~/.vibeui/recent-workspaces.json` as a JSON array (most-recent-first). The list:

- **Self-prunes** on read: `list_recent_workspaces` filters out entries whose paths no longer exist on disk. A renamed or deleted project doesn't keep haunting the list.
- **Move-to-front semantics**: re-opening a recent doesn't duplicate the entry; it bubbles to position 0. Idempotent.
- **Capped at 10**: oldest entry is dropped on overflow.
- **Manual remove**: each row has an `×` button that calls `remove_recent_workspace` — useful when you renamed a project but the recent still points at the old path.

The empty-state UI surfaces recents below the "Open Folder" button. Clicking a recent has the same effect as picking it through the system folder dialog (validation runs, panels notified via `vibeui:workspace-changed`).

---

## Cross-panel notification

When the workspace changes, the panel emits a `vibeui:workspace-changed` window event with the new path as `event.detail`:

```ts
window.addEventListener("vibeui:workspace-changed", (e) => {
  const newPath = (e as CustomEvent<string>).detail;
  // re-fetch panel state for the new workspace
});
```

Panels that read workspace-scoped data (Sessions, Memory, Diffcomplete, Agent, etc.) listen for this event and reload. If you write a new panel that depends on the workspace, listen for this event — don't assume a one-shot read of `vibeui_workspace` from localStorage at mount is enough.

---

## /health declaration

`features.workspace`:

```json
{
  "available": true,
  "transport": "tauri-desktop",
  "recents_path": "~/.vibeui/recent-workspaces.json",
  "recents_cap": 10,
  "validates": ["exists", "is_directory"]
}
```

The `validates` array is the audit trail — clients integrating workspace flows from another surface can read this to know what level of validation the daemon enforces. Adding a new validator (e.g. `is_git_repo`) appends to this array.

---

## Observability

Backend events under `vibecody::workspace`:

```bash
RUST_LOG=vibecody::workspace=info vibecli serve
```

Events:

```
INFO  vibecody::workspace: workspace.add path=/Users/me/projects/foo recent_count=3
WARN  vibecody::workspace: workspace.add.rejected: path does not exist path=/tmp/deleted-project
WARN  vibecody::workspace: workspace.add.rejected: not a directory path=/Users/me/some-file.txt
INFO  vibecody::workspace: workspace.recent.remove path=/old/project remaining=4
```

Workspace **paths are logged** because they're already operator-facing (visible in the panel, in startup banners, in `/health` derived state). File contents inside the workspace are NEVER logged from this target.

---

## Accessibility

- Recents list rows are keyboard-reachable (`role="region"` parent with `aria-label="Recent workspaces"`); each row's open button activates on Enter / Space and carries a verbose `aria-label`: *"Open recent workspace: /path"*.
- Per-row remove (`×`) buttons gain `aria-label="Remove /path from recents"` so AT users hear which entry they're about to drop.
- The Open Folder button gains `aria-label="Open folder via system picker"` so its purpose is clear when read alongside the recents list (otherwise both buttons would announce as just "Open").

---

## Cross-client behaviour

| Client | Workspace UI |
|---|---|
| **VibeUI / VibeApp** | Full picker + recents |
| **VibeMobile** | Inherits the daemon's active workspace; can't change it |
| **VibeWatch** | Same — read-only inheritance |
| **IDE plugins** | Use the IDE's own workspace; tell the daemon via `add_workspace_folder` on workspace open |

The daemon stores at most one active workspace at a time today. Multi-workspace is a future feature — the `workspace.folders()` API returns a Vec but the desktop UI currently only renders the first entry.

---

## Troubleshooting

### "I picked a folder but the file tree is still empty"

Either the folder is empty (genuinely) or the path doesn't exist. Watch `RUST_LOG=vibecody::workspace=info` for `workspace.add.rejected` events — if you see one, the folder you picked is gone or has been renamed since.

### "Recents list never appears"

`list_recent_workspaces` returns `[]` until you've opened at least one workspace. Open a folder once and the list appears next launch.

### "A recent points at a renamed project"

The list self-prunes when the path no longer exists, but if the path STILL exists at the old location (just empty), it'll keep showing. Click the `×` to remove that entry from recents.

### "Panels show stale data after switching workspaces"

The panel didn't subscribe to `vibeui:workspace-changed`. File a bug on the panel — the contract is that every workspace-scoped panel re-fetches on this event.

### "Cmd+O doesn't open the folder picker"

The keyboard shortcut binding requires focus to be in the main app shell. If you're focused inside an embedded webview or the chat textarea, `⌘O` may be captured by that surface — click the canvas first.

---

## Related

- **Source:**
  - `vibeui/src-tauri/src/commands.rs` — `add_workspace_folder`, `get_workspace_folders`, `list_recent_workspaces`, `remove_recent_workspace`
  - `vibeui/src/App.tsx` — the picker UI + recents rendering
- **Sessions:** [`docs/sessions`](./sessions.md) — session list reads from `<workspace>/.vibecli/traces/`
- **Agent Panel:** [`docs/agent-panel`](./agent-panel.md) — agent runs are rooted at the active workspace
