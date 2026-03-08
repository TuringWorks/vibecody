---
triggers: ["TiDB", "tidb", "tidb cloud", "tikv", "tiflash", "htap database", "mysql distributed"]
tools_allowed: ["read_file", "write_file", "bash"]
category: database
---

# TiDB

When working with TiDB:

1. TiDB is a MySQL-compatible distributed database with HTAP (hybrid transactional/analytical processing) — connect with any MySQL driver; supports most MySQL 8.0 syntax including stored procedures and triggers.
2. Architecture: TiDB (SQL layer, stateless) + TiKV (row store, Raft consensus) + TiFlash (columnar store, real-time analytics). TiFlash replicates data from TiKV automatically for analytical queries.
3. Use `AUTO_RANDOM` instead of `AUTO_INCREMENT` for primary keys: `CREATE TABLE orders (id BIGINT PRIMARY KEY AUTO_RANDOM, ...)` — distributes inserts across all TiKV nodes, preventing write hotspots.
4. TiFlash for analytics: `ALTER TABLE events SET TIFLASH REPLICA 1` — creates a columnar replica; the optimizer automatically routes OLAP queries to TiFlash while OLTP goes to TiKV.
5. Partition tables for large datasets: `PARTITION BY RANGE (YEAR(created_at)) (PARTITION p2023 VALUES LESS THAN (2024), PARTITION p2024 VALUES LESS THAN (2025))` — enables partition pruning and efficient time-range queries.
6. Read replicas via Follower Read: `SET @@tidb_replica_read = 'closest-replicas'` — read from the nearest TiKV replica instead of the leader, reducing cross-AZ latency for stale-tolerant reads.
7. Placement rules: control data locality with `ALTER TABLE t SET PLACEMENT POLICY p1` where policy specifies regions/zones for compliance or latency requirements.
8. CDC (TiCDC): `cdc cli changefeed create --sink-uri='kafka://broker:9092/topic'` — stream row changes to Kafka, MySQL, or S3 for downstream consumption.
9. Use `EXPLAIN ANALYZE` to inspect query execution plans; `INFORMATION_SCHEMA.SLOW_QUERY` for slow query analysis; TiDB Dashboard for cluster-wide monitoring.
10. Backup & restore: `br backup full --pd "pd-host:2379" --storage "s3://bucket/backup"` for distributed backup via BR (Backup & Restore) tool; supports incremental and snapshot backups.
11. TiDB Cloud: managed service on AWS/GCP with Serverless tier (auto-scaling, pay-per-use) and Dedicated tier (reserved capacity); free tier available for development.
12. Migration from MySQL: use TiDB Data Migration (DM) for continuous replication from MySQL/MariaDB; `dmctl start-task task.yaml` handles full dump + incremental binlog sync.
