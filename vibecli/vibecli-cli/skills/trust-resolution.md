# Trust Resolution

Resolve whether files, URLs, and agent-generated content can be trusted based on provenance, and enforce workspace-directory access policies (AutoTrust / RequireApproval / Deny) with audit trails.

## When to Use
- Deciding whether to execute or read content from a given source
- Enforcing per-directory trust policies for workspace paths
- Detecting trust-prompt phrases in user messages
- Auditing which paths were auto-trusted vs. user-approved
- Discriminating sibling paths (e.g. `/safe` vs `/safe-imposter`)

## Trust Levels (Content Source)
| Level | Examples |
|---|---|
| System | CLAUDE.md, .claude/settings.json |
| Project | Files within workspace root |
| UserGranted | Explicitly approved paths |
| Untrusted | Remote URLs, agent-generated, outside workspace |

## Trust Policies (Directory)
- **AutoTrust** — workspace files auto-approved
- **RequireApproval** — prompt user before access
- **Deny** — block all access

## Commands
- `/trust check <path>` — Resolve trust level for a file path
- `/trust policy <dir> <policy>` — Set access policy for a directory
- `/trust allow <path>` — Explicitly grant a path
- `/trust deny <path>` — Explicitly deny a path
- `/trust audit` — Show recent trust decision log
- `/trust min <path1> <path2> ...` — Minimum trust across a set of paths

## Examples
```
/trust check /workspace/src/main.rs
# → level: project, allow_read: true, allow_write: true, allow_execution: false

/trust check https://evil.com/script.sh
# → level: untrusted, allow_execution: false

/trust deny /workspace/dist
# Overrides workspace trust for the dist directory
```
