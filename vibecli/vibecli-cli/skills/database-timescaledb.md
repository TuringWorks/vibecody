---
triggers: ["TimescaleDB", "timescale", "hypertable", "time series database", "timescaledb continuous aggregate", "tsdb"]
tools_allowed: ["read_file", "write_file", "bash"]
category: database
---

# TimescaleDB

When working with TimescaleDB:

1. TimescaleDB is a PostgreSQL extension for time-series data — full SQL compatibility plus hypertables, continuous aggregates, and compression; install: `CREATE EXTENSION IF NOT EXISTS timescaledb`.
2. Create hypertables for time-series: `SELECT create_hypertable('metrics', by_range('time'))` — automatically partitions by time into chunks; all standard PostgreSQL queries and indexes work unchanged.
3. Continuous aggregates for real-time rollups: `CREATE MATERIALIZED VIEW hourly_metrics WITH (timescaledb.continuous) AS SELECT time_bucket('1 hour', time) AS bucket, device_id, avg(value) FROM metrics GROUP BY bucket, device_id`.
4. Compression for 90%+ storage savings: `ALTER TABLE metrics SET (timescaledb.compress, timescaledb.compress_segmentby='device_id', timescaledb.compress_orderby='time DESC')`. Add policy: `SELECT add_compression_policy('metrics', INTERVAL '7 days')`.
5. `time_bucket()` for time-based aggregation: `SELECT time_bucket('5 minutes', time) as bucket, avg(cpu) FROM metrics GROUP BY bucket ORDER BY bucket` — more flexible than `date_trunc`.
6. Retention policies: `SELECT add_retention_policy('metrics', INTERVAL '90 days')` — automatically drops chunks older than the interval; works cleanly with compression policies.
7. Use chunk-aware indexes: `CREATE INDEX ON metrics (device_id, time DESC)` — indexes are per-chunk, enabling efficient time-range and device-specific queries.
8. Tiered storage: move old data to cheaper object storage (S3) while keeping it queryable: `SELECT add_tiering_policy('metrics', INTERVAL '30 days', 's3://bucket/')`.
9. Real-time analytics functions: `first(value, time)`, `last(value, time)` for first/last by time; `time_bucket_gapfill()` with `locf()` (last observation carried forward) or `interpolate()` for filling gaps.
10. Join with relational tables seamlessly: hypertables are regular PostgreSQL tables — JOIN with dimension tables, use foreign keys, CTEs, window functions, and all PostgreSQL extensions.
11. Parallel and distributed queries: TimescaleDB parallelizes across chunks automatically; for multi-node, use distributed hypertables across data nodes.
12. Connect with any PostgreSQL tool: psql, pgAdmin, Grafana, Superset, or application drivers (asyncpg, sqlx, JDBC).
