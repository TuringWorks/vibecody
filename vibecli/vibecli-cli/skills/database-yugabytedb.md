---
triggers: ["YugabyteDB", "yugabyte", "ysql", "ycql", "distributed postgresql", "yugabyte cluster"]
tools_allowed: ["read_file", "write_file", "bash"]
category: database
---

# YugabyteDB

When working with YugabyteDB:

1. YugabyteDB is a distributed PostgreSQL-compatible database — use YSQL (PostgreSQL-compatible) API with standard `psql`, Prisma, or any PostgreSQL driver; near-100% PostgreSQL syntax compatibility.
2. Use hash-sharded primary keys for even distribution: `CREATE TABLE orders (id UUID DEFAULT gen_random_uuid(), ... PRIMARY KEY (id))`. Use `ASC`/`DESC` for range-sharded keys when range scans are primary access pattern.
3. Multi-region deployment: `CREATE TABLESPACE us_east WITH (replica_placement='{"num_replicas":3, "placement_blocks":[{"cloud":"aws","region":"us-east-1","zone":"us-east-1a","min_num_replicas":1}]}')`.
4. Geo-partitioned tables: `CREATE TABLE users (..., region TEXT) PARTITION BY LIST (region)` with tablespaces to pin partitions to specific regions — keeps data close to users for low latency.
5. Transactions: default isolation is snapshot (equivalent to PostgreSQL's REPEATABLE READ); supports SERIALIZABLE; distributed transactions across tablets with automatic conflict resolution.
6. Colocated tables: `CREATE DATABASE mydb WITH COLOCATION = true` — small tables share a single tablet, reducing overhead; add `WITH (COLOCATION = false)` to large tables that need independent sharding.
7. YCQL API (Cassandra-compatible): use for high-throughput key-value workloads where Cassandra-style data modeling is preferred; `CREATE TABLE ... WITH transactions = {'enabled': true}` enables ACID on YCQL.
8. CDC with `yb_cdc`: stream row changes to Kafka, Debezium, or custom consumers for real-time data pipelines and event-driven architectures.
9. xCluster replication: async replication between clusters in different regions for disaster recovery; `yb-admin setup_universe_replication` configures bidirectional replication.
10. Backup: `yb-admin create_snapshot` for consistent distributed snapshots; PITR (point-in-time recovery) with `yb-admin restore_snapshot_schedule` for fine-grained recovery.
11. Performance: use `EXPLAIN (ANALYZE, DIST)` to see distributed query plan with per-node stats; key metrics: `rpc_latency`, `tablet_splits`, `rocksdb_*` in the YB Master/TServer dashboards.
12. YugabyteDB Managed: fully managed DBaaS on AWS/GCP/Azure with auto-scaling, VPC peering, and enterprise security.
