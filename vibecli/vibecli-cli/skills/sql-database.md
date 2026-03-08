---
triggers: ["SQL", "SQL query", "SELECT", "JOIN", "stored procedure", "database query", "SQL optimization", "SQL injection prevention", "relational database", "DDL", "DML"]
tools_allowed: ["read_file", "write_file", "bash"]
category: sql
---

# SQL Best Practices

When writing SQL queries and database code:

1. Always use parameterized queries — never concatenate user input into SQL strings; use `WHERE id = ?` or `WHERE id = @id` with bound parameters to prevent SQL injection.
2. Use explicit `JOIN` syntax over implicit joins: `SELECT a.name, b.total FROM orders b INNER JOIN customers a ON a.id = b.customer_id` — clearer intent, easier to spot missing join conditions.
3. Avoid `SELECT *` in production code — specify columns explicitly for clarity, performance (less data transferred), and stability (schema changes won't break queries).
4. Use appropriate indexes: create indexes on columns used in `WHERE`, `JOIN`, `ORDER BY`, and `GROUP BY`; use composite indexes for multi-column filters with leftmost prefix rule; avoid over-indexing (slows writes).
5. Understand `NULL` semantics: `NULL = NULL` is `NULL` (not `TRUE`); use `IS NULL` / `IS NOT NULL`; `COALESCE(col, default)` for null substitution; `NULL` values are excluded from `COUNT(column)` but included in `COUNT(*)`.
6. Use CTEs (Common Table Expressions) for readability: `WITH active_users AS (SELECT * FROM users WHERE active = 1) SELECT * FROM active_users WHERE created_at > '2025-01-01'` — recursive CTEs handle hierarchical data.
7. Use window functions for analytics: `ROW_NUMBER() OVER (PARTITION BY dept ORDER BY salary DESC)` for ranking; `LAG/LEAD` for previous/next row; `SUM() OVER (ORDER BY date ROWS UNBOUNDED PRECEDING)` for running totals.
8. Write idempotent migrations: `CREATE TABLE IF NOT EXISTS`; `ALTER TABLE ... ADD COLUMN IF NOT EXISTS`; use migration tools (Flyway, Liquibase, Alembic) with versioned scripts — never modify already-applied migrations.
9. Use `EXPLAIN` / `EXPLAIN ANALYZE` to understand query plans — look for sequential scans on large tables, nested loops with high row counts, and missing index usage; optimize the slowest queries first.
10. Normalize to 3NF for OLTP, denormalize strategically for OLAP — use foreign keys for referential integrity; use `ON DELETE CASCADE` or `ON DELETE SET NULL` with deliberate intent.
11. Use transactions for multi-statement operations: `BEGIN; UPDATE ...; INSERT ...; COMMIT;` — set appropriate isolation levels; use `SAVEPOINT` for partial rollback within transactions.
12. Prefer `EXISTS` over `IN` for correlated subqueries: `WHERE EXISTS (SELECT 1 FROM orders WHERE orders.user_id = users.id)` — often more efficient; `NOT EXISTS` is clearer than `NOT IN` (which has NULL pitfalls).
13. Use `UNION ALL` instead of `UNION` when duplicates are acceptable — `UNION` performs an implicit `DISTINCT` sort; `UNION ALL` is significantly faster for large result sets.
14. Paginate with keyset pagination for large datasets: `WHERE id > @last_id ORDER BY id LIMIT 20` — avoids the O(n) offset scan of `OFFSET/LIMIT` which degrades on deep pages.
