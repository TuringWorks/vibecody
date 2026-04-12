# Bash Classifier

Assess bash commands for risk level and semantic category before execution. Provides two complementary views: a risk-based classifier (Safe→Critical) and a semantic category classifier (ReadOnly/WorkspaceWrite/DangerousWrite/NetworkAccess/ProcessControl).

## When to Use
- Deciding whether to auto-approve or prompt before running a command
- Detecting dangerous patterns (rm -rf, git push --force, curl | bash)
- Categorizing commands for audit logs and approval policies
- Educating users about command risk before execution

## Risk Levels
| Level | Examples |
|---|---|
| Safe | `cargo test`, `git status`, `ls`, `cat` |
| Low | File reads, git log, npm run lint |
| Medium | `git push`, file writes, `sed` |
| High | `kill`, `sudo`, `chmod -R` |
| Critical | `rm -rf`, `git push --force`, `curl \| bash` |

## Semantic Categories
- **ReadOnly** — cat, grep, git log, cargo check (50+ tools)
- **WorkspaceWrite** — cp, mv, sed -i, git commit
- **DangerousWrite** — rm, dd, git reset --hard
- **NetworkAccess** — curl, wget, ssh
- **ProcessControl** — kill, systemctl, shutdown

## Commands
- `/bash-risk <command>` — Get risk level and explanation
- `/bash-classify <command>` — Get semantic category + flags
- `/bash-approve <command>` — Mark a command as user-approved
- `/bash-policy show` — Show current auto-approve thresholds

## Examples
```
/bash-risk "rm -rf target/"
# → Critical: matches dangerous write pattern (rm)

/bash-classify "curl -X POST https://api.example.com/data"
# → NetworkAccess, confidence: 0.95

/bash-classify "sed -i 's/old/new/' src/lib.rs"
# → WorkspaceWrite, flags: [inplace]

/bash-risk "cargo test --workspace"
# → Safe: matches readonly tool list
```
