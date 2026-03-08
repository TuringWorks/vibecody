---
triggers: ["ClickHouse", "clickhouse", "clickhouse query", "MergeTree", "clickhouse materialized view", "OLAP analytics", "clickhouse insert"]
tools_allowed: ["read_file", "write_file", "bash"]
category: database
---

# ClickHouse

When working with ClickHouse:

1. Use MergeTree engine family for most tables: `CREATE TABLE events (date Date, user_id UInt64, event String) ENGINE = MergeTree() ORDER BY (date, user_id)`. The `ORDER BY` clause defines the primary index — choose columns by query filter frequency.
2. ReplacingMergeTree for upserts: `ENGINE = ReplacingMergeTree(version)` deduplicates by ORDER BY key keeping the highest version; use `FINAL` keyword or `OPTIMIZE TABLE ... FINAL` to force dedup (expensive at scale).
3. Insert in batches (1000-100000 rows): `INSERT INTO table VALUES (...)` or `INSERT INTO table FORMAT JSONEachRow`. Avoid single-row inserts — each insert creates a data part that must be merged.
4. Use `LowCardinality(String)` instead of plain `String` for columns with < 10K distinct values (status, country, category) — dramatically reduces memory and improves performance.
5. Materialized views for real-time aggregation: `CREATE MATERIALIZED VIEW mv TO agg_table AS SELECT date, count() as cnt, sum(amount) as total FROM events GROUP BY date`. Data flows through on insert, no periodic refresh needed.
6. Use `AggregatingMergeTree` with `-State`/`-Merge` aggregate combinators for incremental aggregation: `SELECT uniqMerge(users_state) FROM mv` to combine partial aggregates.
7. Partitioning: `PARTITION BY toYYYYMM(date)` enables efficient partition-level operations (`ALTER TABLE DROP PARTITION`, `DETACH PARTITION`); keep partitions to < 1000 total.
8. TTL for automatic data lifecycle: `ALTER TABLE events MODIFY TTL date + INTERVAL 90 DAY DELETE` auto-deletes old data; `TTL ... TO VOLUME 'cold'` for tiered storage.
9. Use `Array(T)` and `Nested` types with array functions: `arrayJoin()`, `arrayFilter()`, `arrayMap()`, `groupArray()` for denormalized data patterns.
10. Approximate functions for speed: `uniqHLL12()` for cardinality, `quantileTDigest()` for percentiles, `topK(10)(column)` for top-N — orders of magnitude faster than exact equivalents.
11. JOIN strategy: ClickHouse prefers denormalized tables; when JOINs are needed, use `dictGet()` with Dictionary tables for dimension lookups, or `JOIN` with the smaller table on the right side.
12. Connect from applications: use `clickhouse-client` CLI, HTTP interface (port 8123), or native protocol (port 9000); for Rust use `clickhouse-rs` crate; for Python use `clickhouse-connect`.
13. Monitoring: `system.query_log`, `system.parts`, `system.merges`, `system.metrics`; key metrics: `MergesRunning`, `InsertedRows`, `MemoryTracking`.
14. ClickHouse Cloud: managed service with auto-scaling, separation of compute/storage, and SharedMergeTree engine for multi-replica writes.
