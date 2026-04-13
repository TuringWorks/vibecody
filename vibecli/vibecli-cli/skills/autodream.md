# AutoDream

Background memory consolidation — merge duplicate entries, prune stale ones, and rank survivors by recency and access frequency.

## When to Use
- Compacting long-running agent memory stores that accumulate duplicates over time
- Enforcing a maximum memory size while keeping the most-accessed entries
- Removing entries that have not been relevant for more than N days
- Re-ranking memory before injecting it into a prompt to prioritise hot entries

## Commands
- `/dream consolidate` — Run the full consolidation pipeline on the active memory store
- `/dream stats` — Show merge/prune/kept counts from the last consolidation run
- `/dream policy` — Display the current ConsolidationPolicy (similarity threshold, max age, max entries)
- `/dream policy set max_age_days <N>` — Update the maximum age before an entry is pruned
- `/dream rank` — Print the current memory list sorted by access count

## Examples
```
/dream consolidate
# merged: 12  pruned: 5  kept: 83

/dream policy set max_age_days 14
# Policy updated: max_age_secs = 1209600

/dream rank
# 1. [access:47] api_key_pattern — "Never commit secrets to version control"
# 2. [access:31] preferred_language — "Rust"
```
