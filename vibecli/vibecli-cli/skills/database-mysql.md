---
triggers: ["MySQL", "mysql query", "InnoDB", "mysql replication", "mysql index", "mysql performance", "mysql migration"]
tools_allowed: ["read_file", "write_file", "bash"]
category: database
---

# MySQL

When working with MySQL:

1. Use InnoDB engine (default since 5.5) — supports transactions, row-level locking, foreign keys, and crash recovery; never use MyISAM for new tables.
2. Use `EXPLAIN ANALYZE` (MySQL 8.0.18+) to profile queries with actual execution stats; for older versions use `EXPLAIN FORMAT=JSON` for cost estimates and access patterns.
3. Index strategy: create composite indexes matching `WHERE` clause column order (leftmost prefix rule); use `SHOW INDEX FROM table` and check cardinality; cover queries with covering indexes to avoid table lookups.
4. Use `INSERT ... ON DUPLICATE KEY UPDATE` for upserts; use `INSERT IGNORE` when you want to silently skip duplicates on unique constraints.
5. Connection pooling: use ProxySQL or application-level pools (HikariCP for Java, sqlx pool for Rust); set `max_connections` based on available RAM (~150-300 for typical workloads), monitor with `SHOW PROCESSLIST`.
6. Use `utf8mb4` charset and `utf8mb4_unicode_ci` collation (not `utf8` which is only 3-byte and can't store emoji/CJK); set at server, database, and table level.
7. Partitioning: use RANGE partitioning for time-series data (`PARTITION BY RANGE (YEAR(created_at))`); use LIST partitioning for categorical data; partition pruning only works when the partition key is in the WHERE clause.
8. Replication: set up semi-synchronous replication for durability (`rpl_semi_sync_source_enabled=1`); use GTID-based replication (`gtid_mode=ON`) for easier failover; route reads to replicas, writes to primary.
9. Use prepared statements to prevent SQL injection and improve performance through query plan caching: `PREPARE stmt FROM 'SELECT * FROM users WHERE id = ?'`.
10. JSON support (MySQL 5.7+): use `JSON` column type with `JSON_EXTRACT()`, `->` and `->>` operators; create generated columns with functional indexes for JSON path queries.
11. Window functions (MySQL 8.0+): `ROW_NUMBER()`, `RANK()`, `DENSE_RANK()`, `LAG()`, `LEAD()`, `NTILE()` with `OVER (PARTITION BY ... ORDER BY ...)` for analytics queries.
12. Use `pt-query-digest` (Percona Toolkit) to analyze slow query logs; enable `slow_query_log` with `long_query_time=1` and `log_queries_not_using_indexes=ON`.
13. Online DDL: use `ALTER TABLE ... ALGORITHM=INPLACE, LOCK=NONE` for non-blocking schema changes; for large tables use `pt-online-schema-change` or `gh-ost` to avoid locking.
14. Backup strategy: use `mysqldump --single-transaction` for logical backups of InnoDB; use Percona XtraBackup for hot physical backups; test restores regularly.
