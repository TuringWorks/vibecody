---
triggers: ["Aurora PostgreSQL", "aurora postgres", "aws aurora postgresql", "aurora pg", "aurora postgresql compatible"]
tools_allowed: ["read_file", "write_file", "bash"]
requires_bins: ["aws"]
category: cloud-aws
---

# AWS Aurora PostgreSQL

When working with Aurora PostgreSQL:

1. Aurora PostgreSQL is a PostgreSQL 16-compatible managed database with up to 3x throughput of standard PostgreSQL — distributed storage with 6-way replication, automatic failover, and up to 15 read replicas.
2. Connection endpoints: writer endpoint for reads/writes (`*.cluster-*.rds.amazonaws.com`), reader endpoint for load-balanced reads (`*.cluster-ro-*.rds.amazonaws.com`); use `target_session_attrs=read-write` in libpq for automatic routing.
3. Aurora Serverless v2: `--serverless-v2-scaling-configuration MinCapacity=0.5,MaxCapacity=32` — auto-scales compute in 0.5 ACU increments; set CloudWatch alarms on `ServerlessDatabaseCapacity` for visibility.
4. Aurora Optimized Reads: enable for up to 8x faster queries on large datasets by using local NVMe storage for temp files and buffer pool overflow — ideal for `ORDER BY`, `GROUP BY`, and hash joins on large tables.
5. Babelfish for SQL Server compatibility: `CREATE EXTENSION babelfishpg_tsql` — run SQL Server T-SQL queries and applications against Aurora PostgreSQL with minimal code changes; listen on TDS port 1433.
6. pgvector extension for AI: `CREATE EXTENSION vector; CREATE TABLE items (embedding vector(1536)); CREATE INDEX ON items USING hnsw (embedding vector_cosine_ops)` — ANN search for RAG and semantic similarity.
7. Advisory locks for distributed coordination: `SELECT pg_advisory_lock(hash)` — useful for leader election and job deduplication in multi-instance applications.
8. Logical replication: `CREATE PUBLICATION my_pub FOR TABLE orders; CREATE SUBSCRIPTION my_sub CONNECTION '...' PUBLICATION my_pub` — replicate to external PostgreSQL, Debezium, or data warehouses.
9. RDS Data API for serverless: `aws rds-data execute-statement --resource-arn ... --sql "SELECT * FROM users WHERE id = :id" --parameters '[{"name":"id","value":{"longValue":42}}]'` — HTTP-based SQL without connection management.
10. Global Database: cross-region replication with < 1 second lag; managed failover with RPO < 1 second; `aws rds failover-global-cluster --global-cluster-identifier my-global --target-db-cluster-identifier us-west-cluster`.
11. Export to S3: `SELECT * FROM aws_s3.query_export_to_s3('SELECT * FROM events', ...)` or `aws rds start-export-task` for full cluster snapshots to S3 in Parquet format for analytics.
12. Monitoring: Performance Insights with `pi.db.load.avg` for top SQL by wait events; `pg_stat_statements` for query-level stats; `aurora_stat_backend_waits` for Aurora-specific wait events.
