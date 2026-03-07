---
triggers: ["RDS", "Aurora", "aws rds", "rds proxy", "aurora serverless", "aws database", "rds iam auth", "aurora global database"]
tools_allowed: ["read_file", "write_file", "bash"]
requires_bins: ["aws"]
category: cloud-aws
---

# AWS RDS and Aurora Programming

When working with RDS and Aurora:

1. Use RDS Proxy for connection pooling to prevent connection exhaustion from Lambda or microservices; configure `max_connections_percent` (e.g., 50-80%) and enable IAM authentication on the proxy to eliminate credential rotation in application code.
2. Enable IAM database authentication (`--enable-iam-database-authentication`) and generate auth tokens with `rds.generate_db_auth_token()` (boto3) or `Signer.getAuthToken()` (JS SDK); tokens expire in 15 minutes and the SSL connection is mandatory.
3. Deploy Aurora Serverless v2 with `min_capacity: 0.5` and `max_capacity: 16` ACUs for variable workloads; it scales in 0.5 ACU increments within seconds, but set CloudWatch alarms on `ServerlessDatabaseCapacity` to detect scaling limits.
4. Use read replicas for read-heavy workloads by routing SELECT queries to the reader endpoint (`*.cluster-ro-*.rds.amazonaws.com`); implement connection-level routing in your ORM or use Aurora's built-in reader endpoint with connection load balancing.
5. Configure parameter groups rather than modifying parameters on the instance directly; key tuning params include `max_connections`, `innodb_buffer_pool_size` (Pg: `shared_buffers`), and `slow_query_log`/`log_min_duration_statement` for query analysis.
6. Enable automated backups with a retention period of at least 7 days and take manual snapshots before major migrations; use `aws rds restore-db-instance-to-point-in-time` for granular recovery to any second within the retention window.
7. Implement blue/green deployments (`aws rds create-blue-green-deployment`) for zero-downtime schema migrations and engine upgrades; the green environment is a synchronized replica that switchover promotes, with automatic rollback on failure.
8. Use Aurora Global Database for cross-region disaster recovery with sub-second replication lag; promote a secondary region with `aws rds failover-global-cluster` and update your application's DNS or connection string via Route 53 health checks.
9. Encrypt instances at rest with KMS (`--storage-encrypted --kms-key-id`) at creation time (cannot be added later without snapshot/restore); enforce encryption via SCP denying `rds:CreateDBInstance` without the `rds:StorageEncrypted` condition.
10. Monitor with Performance Insights (`--enable-performance-insights --performance-insights-retention-period 731`) to identify top SQL queries by wait events, load, and latency; query the PI API programmatically to build custom dashboards.
11. Use the Data API for Aurora Serverless to execute SQL over HTTPS without managing connections (`rds-data.execute_statement()`); batch operations with `batch_execute_statement()` and pass parameters with `SqlParameter` typed values to prevent SQL injection.
12. Schedule minor version upgrades during maintenance windows and test major upgrades in a cloned environment first (`aws rds restore-db-cluster-to-point-in-time`); use `aws rds describe-db-engine-versions` to check target compatibility and deprecation timelines.
