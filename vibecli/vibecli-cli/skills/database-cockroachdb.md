---
triggers: ["CockroachDB", "cockroach", "cockroachdb", "crdb", "distributed sql", "cockroach cluster", "cockroach serverless"]
tools_allowed: ["read_file", "write_file", "bash"]
category: database
---

# CockroachDB

When working with CockroachDB:

1. CockroachDB is a distributed SQL database — PostgreSQL-compatible wire protocol and SQL syntax; use standard `psql`, Prisma, or any PostgreSQL driver to connect.
2. Use `UUID` primary keys with `DEFAULT gen_random_uuid()` instead of sequential integers — sequential keys cause hotspots in distributed environments because all inserts go to the same range.
3. Multi-region deployments: `ALTER DATABASE db SET PRIMARY REGION 'us-east1'; ALTER DATABASE db ADD REGION 'eu-west1'; ALTER TABLE users SET LOCALITY REGIONAL BY ROW` — data locality follows the user.
4. Transactions are serializable by default (strongest isolation); use `BEGIN; ... COMMIT;` with automatic retries; implement retry loops in application code: `SAVEPOINT cockroach_restart; ... RELEASE SAVEPOINT cockroach_restart`.
5. Secondary indexes: `CREATE INDEX idx_email ON users(email) STORING (name)` — the `STORING` clause avoids index join lookups (like PostgreSQL's `INCLUDE`).
6. Change Data Capture: `CREATE CHANGEFEED FOR TABLE orders INTO 'kafka://broker:9092' WITH updated, resolved` — stream row changes to Kafka, cloud storage, or webhooks.
7. `AS OF SYSTEM TIME` for follower reads: `SELECT * FROM users AS OF SYSTEM TIME '-10s'` reads from any replica (not just leaseholder) — reduces latency for stale-tolerant queries.
8. Import data: `IMPORT INTO table CSV DATA ('s3://bucket/data.csv')` for bulk loads from cloud storage; use `COPY FROM STDIN` for smaller datasets via the SQL shell.
9. Schema changes are online and non-blocking by default — `ALTER TABLE ADD COLUMN`, `CREATE INDEX` run without locking the table; monitor progress with `SHOW JOBS`.
10. Use CockroachDB Serverless for development/small workloads (auto-scaling, pay-per-query); Dedicated for production with reserved capacity and SLA guarantees.
11. Monitoring: built-in DB Console at `:8080` shows SQL activity, replication, storage, and hot ranges; `crdb_internal.node_statement_statistics` for query performance analysis.
12. Backup/restore: `BACKUP DATABASE db INTO 's3://bucket/backup' AS OF SYSTEM TIME '-10s'`; incremental backups with `BACKUP ... INCREMENTAL FROM LATEST IN 's3://bucket/backup'`.
