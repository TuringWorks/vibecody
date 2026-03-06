---
triggers: ["SQLite", "WAL mode", "FTS5", "embedded database", "rusqlite", "better-sqlite3"]
tools_allowed: ["read_file", "write_file", "bash"]
category: database
---

# SQLite

When using SQLite:

1. Enable WAL mode: `PRAGMA journal_mode=WAL;` — allows concurrent reads with writes
2. Use `PRAGMA busy_timeout=5000;` — wait up to 5s for locks instead of failing immediately
3. Use `PRAGMA foreign_keys=ON;` — SQLite doesn't enforce FK constraints by default
4. FTS5 for full-text search: `CREATE VIRTUAL TABLE docs USING fts5(title, body)` — fast text search
5. JSON1 extension: `json_extract()`, `json_each()` — query JSON columns directly
6. Transactions: batch writes in explicit transactions — 100x faster than autocommit
7. Connection pooling: use one writer + multiple readers in WAL mode
8. Index: create indexes for WHERE, JOIN, ORDER BY columns — use `EXPLAIN QUERY PLAN`
9. Type affinity: SQLite is dynamically typed — use `STRICT` tables for type enforcement
10. Backup: use `.backup` command or `sqlite3_backup_*` API — atomic, consistent copy
11. Rust: use `rusqlite` with `bundled` feature — no external SQLite dependency
12. Good for: embedded apps, CLI tools, prototypes, low-write workloads — not for high-concurrency web
