---
triggers: ["Aurora MySQL", "aurora mysql", "aws aurora mysql", "aurora mysql compatible", "aurora mysql replication"]
tools_allowed: ["read_file", "write_file", "bash"]
requires_bins: ["aws"]
category: cloud-aws
---

# AWS Aurora MySQL

When working with Aurora MySQL:

1. Aurora MySQL is a MySQL 8.0-compatible managed database with up to 5x throughput of standard MySQL — uses a distributed, fault-tolerant storage engine with 6-way replication across 3 AZs.
2. Connection endpoints: writer endpoint for reads/writes, reader endpoint for load-balanced read replicas; `*.cluster-*.rds.amazonaws.com` (writer) and `*.cluster-ro-*.rds.amazonaws.com` (reader).
3. Aurora Serverless v2 for variable workloads: `aws rds create-db-cluster --engine aurora-mysql --serverless-v2-scaling-configuration MinCapacity=0.5,MaxCapacity=16` — scales in 0.5 ACU increments within seconds.
4. Use RDS Proxy for connection pooling: `aws rds create-db-proxy --engine-family MYSQL --auth SecretArn=...` — essential for Lambda/serverless to prevent connection exhaustion; supports IAM authentication.
5. Parallel query for OLAP: `SET aurora_parallel_query = ON` — offloads analytical queries (full scans, aggregations) to the storage layer for up to 100x speedup on large tables without indexes.
6. Backtrack for instant undo: `aws rds backtrack-db-cluster --backtrack-to '2024-01-15T10:00:00Z'` — rewinds the entire cluster to a point in time without restoring from backup (configure `--backtrack-window` in seconds).
7. Clone for dev/test: `aws rds restore-db-cluster-to-point-in-time --restore-type copy-on-write --source-db-cluster mydb-prod` — instant copy-on-write clone, no additional storage cost until data diverges.
8. Global Database for cross-region DR: `aws rds create-global-cluster --global-cluster-identifier my-global --source-db-cluster-identifier mydb` — sub-second replication lag; promote secondary with `failover-global-cluster`.
9. Use `mysql.lambda_async()` to invoke Lambda from Aurora MySQL stored procedures — enables event-driven architectures triggered by database operations.
10. Blue/green deployments: `aws rds create-blue-green-deployment --source mydb --target-engine-version 8.0.36` — test upgrades/schema changes on green, then switchover with minimal downtime.
11. Enhanced monitoring: enable Performance Insights (`--enable-performance-insights`) for top-SQL analysis by wait events; CloudWatch metrics for `AuroraReplicaLag`, `CPUUtilization`, `FreeableMemory`.
12. Binary log replication: `CALL mysql.rds_set_configuration('binlog retention hours', 24)` — retain binlogs for external replication to on-prem MySQL, Debezium CDC, or DMS migration targets.
