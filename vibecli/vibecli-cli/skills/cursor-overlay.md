# Live Collaboration Cursor Overlay

Tracks remote peer cursors for live display in the editor. Extends the CRDT sync module with named, coloured, positioned peer cursors.

## Key Types
- **CursorOverlay** — manages all peer cursors for the local editor
- **PeerCursor** — position, color, display name, selection, file, typing state
- **CursorUpdate** — inbound position event from a remote peer
- **CursorColor** — deterministic palette assignment from peer ID

## Features
- Auto-assigns a distinct colour to each peer from an 8-colour palette
- Ignores local peer's own cursor updates
- Stale detection: cursors inactive for > 30s are pruned
- File-scoped filtering: `cursors_in_file(path)`
- Proximity filter: `cursors_near_line(file, line, radius)`

## Commands
- `/collab cursors` — list active peer cursors
- `/collab status` — show collaborator status line
- `/collab peers` — show all connected peers

## Examples
```
/collab status
# 2 collaborator(s): alice, bob

/collab cursors
# alice @ src/main.rs:42:0 (typing...)
# bob @ src/lib.rs:17:8
```
