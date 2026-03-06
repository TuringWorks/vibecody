---
triggers: ["PostgreSQL", "JSONB", "CTE", "window function", "partitioning", "postgres index", "pg query"]
tools_allowed: ["read_file", "write_file", "bash"]
category: database
---

# PostgreSQL

When working with PostgreSQL:

1. Use `JSONB` for semi-structured data — supports indexing with GIN and `@>` containment operator
2. Use CTEs (`WITH` clauses) for complex queries — improve readability, enable recursive queries
3. Window functions: `ROW_NUMBER()`, `RANK()`, `LAG()`, `LEAD()` with `OVER (PARTITION BY ... ORDER BY ...)`
4. Create indexes for foreign keys and frequently-queried columns — use `EXPLAIN ANALYZE` to verify
5. Use `CREATE INDEX CONCURRENTLY` to avoid locking the table during index creation
6. Partitioning: use range partitioning for time-series, list partitioning for categories
7. Use `UPSERT`: `INSERT ... ON CONFLICT (key) DO UPDATE SET ...` for idempotent writes
8. Use `pg_stat_statements` to find slow queries — optimize the top-N by total time
9. Connection pooling: use PgBouncer or built-in pool — limit connections to `max_connections * 0.8`
10. Use `VACUUM ANALYZE` regularly — or configure `autovacuum` properly
11. Use advisory locks for application-level locking: `pg_advisory_lock(hash)`
12. Enum types: use `CREATE TYPE status AS ENUM (...)` for fixed sets — validates at DB level
