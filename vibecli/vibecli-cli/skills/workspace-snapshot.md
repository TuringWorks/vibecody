# Workspace Snapshot

Point-in-time workspace capture and restore. Matches Cursor 4.0 and Devin 2.0's checkpoint system.

## Key Types
- **WorkspaceSnapshotManager** — capture/get/list/delete/diff
- **WorkspaceSnapshot** — files HashMap (path → FileState), git_head, git_branch
- **FileState** — content_hash (deterministic), size, FileStatus
- **SnapshotDiff** — added/removed/modified/unchanged per-file

## Commands
- `/snapshot capture <label>` — capture current workspace
- `/snapshot list` — list available snapshots
- `/snapshot diff <id1> <id2>` — compare two snapshots
- `/snapshot restore <id>` — restore to a snapshot
- `/snapshot delete <id>` — remove a snapshot
