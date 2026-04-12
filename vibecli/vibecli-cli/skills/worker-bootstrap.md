# Worker Bootstrap

Validate agent capability whitelists and enforce token budgets when spawning worker agents. Prevents workers from acquiring capabilities beyond what the parent agent explicitly authorizes.

## When to Use
- Spawning sub-agents with a restricted capability set
- Capping token budgets for cost-controlled agent tasks
- Auditing what capabilities a worker requested vs. was granted
- Building multi-agent pipelines with least-privilege workers

## Capabilities
| Capability | Description |
|---|---|
| ReadFiles | Access local filesystem for reading |
| WriteFiles | Modify files on disk |
| RunBash | Execute shell commands |
| BrowseWeb | Make HTTP requests |
| CallTools | Invoke registered agent tools |
| SpawnAgents | Create child agents |
| AccessDatabase | Query/write databases |
| ReadGit | Run read-only git commands |
| Custom(name) | Named capability for extension |

## Commands
- `/worker spawn --caps <list> --budget <n>` — Spawn worker with capability set
- `/worker capabilities` — Show available capability names
- `/worker audit <worker-id>` — Show what a worker requested vs. granted
- `/worker revoke <worker-id> <cap>` — Remove a capability from a running worker
- `/worker budget <worker-id>` — Show token budget status

## Examples
```
/worker spawn --caps "ReadFiles,RunBash" --budget 50000
# Worker created with: ReadFiles + RunBash only, max 50k tokens

/worker spawn --caps "ReadFiles,WriteFiles,SpawnAgents" --budget 200000
# Warning: SpawnAgents not in parent whitelist — rejected

/worker audit w-abc123
# Requested: ReadFiles, WriteFiles, RunBash, SpawnAgents
# Granted:   ReadFiles, WriteFiles, RunBash
# Denied:    SpawnAgents (not in parent allowlist)
```
