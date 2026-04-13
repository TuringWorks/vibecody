# Prompt Cache

Static prefix caching — freeze system prompt, tools JSON, and config JSON into a FNV-1a cache key. Reusing the cached prefix achieves linear (not quadratic) cost growth across multi-turn sessions.

## When to Use
- Long-running chat sessions where the system prompt and tools do not change between turns
- Multi-step agent loops that re-send the same large system context on every call
- Cost observability — tracking cache hit rate to verify the cache is effective
- Invalidating a stale prefix after updating the system prompt mid-session

## Commands
- `/cache stats` — Show hits, misses, entries, and hit rate for the active cache
- `/cache key` — Print the current FNV-1a key for the active system/tools/config triple
- `/cache invalidate` — Remove the current prefix from the cache (forces re-insert on next call)
- `/cache clear` — Flush the entire cache
- `/cache hit-rate` — Print the current hit rate as a percentage

## Examples
```
/cache stats
# hits: 47  misses: 3  entries: 2  hit_rate: 94.0%

/cache key
# CacheKey(0x3f7a2e91b4c8d012)

/cache invalidate
# Removed 1 entry. Next call will re-insert.
```
