---
triggers: ["Redis", "caching redis", "pub/sub", "Redis streams", "TTL", "redis cluster", "session store"]
tools_allowed: ["read_file", "write_file", "bash"]
category: database
---

# Redis

When using Redis:

1. Use Redis for caching (TTL-based), sessions, rate limiting, pub/sub, and queues
2. Always set TTL on cache keys: `SET key value EX 3600` — prevent unbounded memory growth
3. Use `MGET`/`MSET` for bulk operations — reduce round trips
4. Pub/Sub for real-time events: `SUBSCRIBE channel`, `PUBLISH channel message`
5. Redis Streams for durable message queues: `XADD`, `XREAD`, `XACK` with consumer groups
6. Use sorted sets (`ZADD`, `ZRANGEBYSCORE`) for leaderboards, rate limiting, time-series
7. Cache-aside pattern: check cache → miss → fetch from DB → populate cache → return
8. Key naming convention: `namespace:entity:id:field` — e.g., `app:user:123:profile`
9. Use `SCAN` instead of `KEYS` in production — `KEYS` blocks the single-threaded server
10. Lua scripting (`EVAL`) for atomic multi-step operations — avoid race conditions
11. Monitor memory: `INFO memory`, `maxmemory-policy allkeys-lru` for automatic eviction
12. Use Redis Cluster or Sentinel for high availability — don't run a single instance in production
