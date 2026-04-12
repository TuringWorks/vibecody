# Recovery Recipes

Automatic error recovery using pattern-matched recipes with attempt-bounded retry, backoff, inject, and escalate actions. Prevents infinite retry loops and surfaces actionable suggestions when retries are exhausted.

## When to Use
- Automatically retrying rate-limited API calls with backoff
- Recovering from network timeouts with exponential retry
- Suggesting `cargo check` after a compilation failure
- Recommending `cargo test --fix` after test failures
- Resolving merge conflicts or permission errors automatically

## Built-in Recipes
| Pattern | Action | Max Attempts |
|---|---|---|
| Rate limit (429) | Backoff 60s | 3 |
| Network error | Retry 5s | 4 |
| Compilation error | Run `cargo check` | 2 |
| Test failure | Run `cargo test` | 2 |
| Merge conflict | Run `git merge --strategy` | 2 |
| Permission denied | Escalate to user | 1 |

## Recovery Actions
- **Backoff** — wait N seconds then retry
- **Retry** — immediate retry with optional delay
- **Inject** — run a corrective command before retrying
- **Escalate** — surface to user with explanation

## Commands
- `/recover suggest <error>` — Get recovery recommendation for an error
- `/recover status` — Show attempt counts for active recipes
- `/recover reset <recipe>` — Reset attempt counter for a recipe
- `/recover list` — Show all available recipes
- `/recover add <pattern> <action>` — Add a custom recovery recipe

## Examples
```
/recover suggest "rate limit exceeded"
# → Backoff recipe: wait 60s, retry (attempt 1/3)

/recover suggest "error[E0382]: borrow of moved value"
# → Inject recipe: run `cargo check` to identify full error context

/recover status
# rate-limit-backoff: 2/3 attempts used
# compile-fix: 1/2 attempts used
```
