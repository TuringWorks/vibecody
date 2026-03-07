---
triggers: ["Cloud SQL", "gcp cloud sql", "cloud sql proxy", "alloydb", "gcp postgres", "gcp mysql", "google sql database"]
tools_allowed: ["read_file", "write_file", "bash"]
requires_bins: ["gcloud"]
category: cloud-gcp
---

# GCP Cloud SQL

When working with Cloud SQL:

1. Always connect through the Cloud SQL Auth Proxy (`cloud-sql-proxy PROJECT:REGION:INSTANCE`) for encrypted, IAM-authenticated connections without managing SSL certificates or allowlisting IPs; in GKE, run it as a sidecar container.
2. Enable IAM database authentication with `gcloud sql instances patch INSTANCE --database-flags=cloudsql.iam_authentication=on` and create IAM users with `CREATE USER 'sa@project.iam' LOGIN` to eliminate password management for service accounts.
3. Configure Private IP connectivity (`--network=VPC_NETWORK --no-assign-ip`) to keep database traffic off the public internet; combine with VPC Service Controls for defense-in-depth.
4. Set up automated backups with `--backup-start-time=02:00` and enable point-in-time recovery (`--enable-bin-log` for MySQL, WAL archiving is automatic for PostgreSQL) with a retention period of at least 7 days.
5. Use read replicas (`gcloud sql instances create REPLICA --master-instance-name=PRIMARY`) to offload read traffic; connect readers to the replica endpoint and handle replication lag in application code by checking `pg_last_wal_replay_lsn()`.
6. Configure maintenance windows with `--maintenance-window-day=SUN --maintenance-window-hour=4` and set `--deny-maintenance-period` around critical business dates to control when disruptive updates occur.
7. Right-size instances by monitoring `cloudsql.googleapis.com/database/cpu/utilization` and `disk/utilization`; use `--tier=db-custom-CPU-RAM` for precise sizing and enable storage auto-resize with `--storage-auto-increase` to prevent disk-full outages.
8. For PostgreSQL workloads requiring horizontal reads and columnar analytics, evaluate AlloyDB (`gcloud alloydb clusters create`) which offers 4x throughput over standard Cloud SQL PostgreSQL with built-in columnar engine for HTAP queries.
9. Use database flags judiciously: set `--database-flags=max_connections=200,log_min_duration_statement=1000` to tune connection limits and enable slow query logging; review flags against the Cloud SQL supported flags list as not all engine flags are available.
10. Implement connection pooling with PgBouncer or the Cloud SQL Language Connectors (`cloud-sql-python-connector`) which manage connection lifecycle, IAM token refresh, and SSL without the proxy binary.
11. Export data with `gcloud sql export sql INSTANCE gs://BUCKET/dump.sql --database=DB` for logical backups or use `gcloud sql export csv` for analytics pipelines; schedule exports via Cloud Scheduler calling a Cloud Function.
12. Set up high availability with `--availability-type=REGIONAL` for automatic failover to a standby in another zone; test failover with `gcloud sql instances failover INSTANCE` during maintenance windows to validate application reconnection logic.
