# Conversation Branching

Fork a conversation session at any message, explore alternative directions, and compare or restore branches. Matches Cursor 4.0's conversation branch feature.

## When to Use
- Exploring alternative approaches to a refactoring without losing the original thread
- Forking before a risky agent action and restoring if it goes wrong
- Comparing two different agent responses side-by-side
- Maintaining a "main" conversation while experimenting in a branch

## Key Types
- **BranchId** — unique identifier for a branch (`main`, `branch-1`, ...)
- **Branch** — linear message sequence forked from a parent at a `fork_point`
- **BranchManager** — manages all branches, active branch, and fork/restore

## Operations
- `fork_at(message_id, name)` — create a new branch from a specific message
- `checkout(branch_id)` — switch the active conversation branch
- `diff(a, b)` → `(only_in_a, only_in_b)` — messages unique to each branch
- `archive(branch_id)` — soft-delete a branch
- `full_history(branch_id)` — complete message list including inherited prefix

## Commands
- `/branch fork <message-id> <name>` — fork at a specific message
- `/branch restore <branch-id>` — switch to a branch
- `/branch list` — show all non-archived branches
- `/branch diff <a> <b>` — compare two branches

## Examples
```
/branch fork msg-42 "try-iterators"
# Forked from msg-42 → branch-3 "try-iterators"

/branch list
# main (12 messages)  branch-3 "try-iterators" (7 messages)

/branch diff main branch-3
# main only: msg-43: "use a loop here"
# branch-3 only: msg-43: "use .iter().map()"
```
