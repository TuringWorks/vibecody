---
triggers: ["database", "SQL", "migration", "index", "query optimization"]
tools_allowed: ["read_file", "write_file", "bash"]
category: database
---

# Database Patterns

1. Always use migrations — never modify schema manually in production
2. Add indexes for columns used in WHERE, JOIN, ORDER BY clauses
3. Use connection pooling (r2d2, sqlx, pg-pool)
4. Prefer batch operations over N+1 queries
5. Use transactions for multi-step operations: BEGIN/COMMIT/ROLLBACK
6. Use `EXPLAIN ANALYZE` to profile slow queries
7. Normalize to 3NF by default, denormalize only when benchmarks justify it
8. Add NOT NULL constraints where appropriate — nullable columns are bugs waiting to happen
9. Use UUIDs or ULIDs for distributed-safe primary keys
10. Always have a `created_at` and `updated_at` timestamp on every table
