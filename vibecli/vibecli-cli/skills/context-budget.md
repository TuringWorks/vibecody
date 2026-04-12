# Context Budget

Token budget enforcement for context windows — soft warn at 80%, auto-prune at 90%, hard block at 100%. Automatically prunes OldToolResult → Attachment → History (never SystemPrompt). Matches GitHub Copilot Workspace v2's context bar.

## When to Use
- Preventing OOM context exhaustion in long agent sessions
- Surfacing a warning when context is filling up
- Automatically pruning low-priority context entries to make room
- Blocking operations that would overflow the hard limit

## Pruning Order
1. **OldToolResult** — oldest tool call results removed first
2. **Attachment** — file attachments removed second
3. **History** — conversation turns removed last
4. **SystemPrompt** — never pruned

## BudgetAction
- `Ok` — below warn threshold
- `Warn { used, limit }` — crossed 80% — show warning to user
- `Prune { bytes_to_free }` — crossed 90% — trigger auto-prune
- `Block { used, limit }` — would exceed hard limit — reject the add

## Commands
- `/budget show` — display current token usage bar
- `/budget set <limit>` — change the hard limit
- `/budget warn <pct>` — change the warn threshold percentage
- `/budget hard <pct>` — change the prune threshold percentage

## Examples
```
/budget show
# [████████████████░░░░] 80% (80,000 / 100,000)

/budget set 200000
# hard limit updated to 200,000 tokens
```
