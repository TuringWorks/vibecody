# Workspace Fingerprint

Generate and compare FNV-1a workspace fingerprints from git HEAD, branch, and tracked file hashes. Detects workspace changes across sessions and enables session namespace isolation.

## When to Use
- Detecting whether a workspace changed between agent sessions
- Generating stable session namespaces tied to workspace state
- Finding which session fingerprint matches the current workspace
- Verifying that cached context is still valid for the current state
- Debugging "why did the agent restart" by comparing fingerprints

## Fingerprint Components
- **head** — git HEAD commit hash
- **branch** — current branch name
- **files** — map of `path → FNV-1a(content)` for tracked files

A fingerprint matches another if head + branch are equal AND no tracked file changed.

## FingerprintDiff
When fingerprints don't match, `diff()` returns:
- `added` — new files since the last fingerprint
- `removed` — deleted files
- `modified` — files with changed content hashes

## Commands
- `/fingerprint show` — Display current workspace fingerprint
- `/fingerprint compare <session-id>` — Compare current vs saved fingerprint
- `/fingerprint diff` — Show file-level changes since last fingerprint
- `/fingerprint save` — Save current fingerprint to the store
- `/fingerprint namespace` — Show session namespace for current workspace

## Examples
```
/fingerprint show
# head: a4b2c1d  branch: main  files: 47  hash: 0x3f7a2e91b4c8d012

/fingerprint diff
# modified: src/lib.rs, src/agent.rs
# added: src/new_module.rs
# removed: src/old_module.rs

/fingerprint namespace
# namespace: main-a4b2c1d (stable for current HEAD)
```
