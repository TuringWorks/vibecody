---
triggers: ["cache invalidation", "LRU cache", "CDN", "HTTP caching", "cache-control", "memoization", "ETag"]
tools_allowed: ["read_file", "write_file", "bash"]
category: performance
---

# Caching Strategies

When implementing caching:

1. Cache-aside: app checks cache → miss → load from source → store in cache → return
2. Use TTL (Time-To-Live) for everything — unbounded caches cause memory issues
3. HTTP caching: use `Cache-Control: public, max-age=31536000` for immutable assets (hashed filenames)
4. Use `ETag` + `If-None-Match` for conditional requests — 304 Not Modified saves bandwidth
5. CDN: cache static assets at the edge — CloudFront, Cloudflare, or Fastly
6. Cache invalidation strategies: TTL expiry, event-driven purge, versioned keys
7. LRU (Least Recently Used): good default eviction policy for bounded caches
8. Memoization: cache function results in memory — use `lru_cache` (Python), `Map` (JS), `HashMap` (Rust)
9. Write-through cache: update cache AND database together — consistency at cost of write latency
10. Cache stampede prevention: use locking or stale-while-revalidate to avoid thundering herd
11. Multi-level caching: L1 in-process → L2 Redis → L3 database — check each level
12. Never cache: authenticated responses (without Vary), error responses, non-idempotent operations
