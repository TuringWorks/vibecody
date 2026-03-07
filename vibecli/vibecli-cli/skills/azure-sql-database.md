---
triggers: ["Azure SQL", "azure sql database", "azure sql server", "elastic pool", "sql hyperscale", "azure database", "sql managed instance"]
tools_allowed: ["read_file", "write_file", "bash"]
requires_bins: ["az"]
category: cloud-azure
---

# Azure SQL Database

When working with Azure SQL Database:

1. Choose the right purchasing model: DTU for predictable workloads (Basic/Standard/Premium bundles compute, storage, IO), vCore for granular control (General Purpose, Business Critical, Hyperscale); use serverless tier (`az sql db create --compute-model Serverless --auto-pause-delay 60`) for intermittent workloads to auto-pause and save costs.
2. Use elastic pools to share resources across multiple databases: `az sql elastic-pool create --capacity 100 --edition GeneralPurpose`; place databases with complementary usage patterns (one peaks morning, another evening) in the same pool — monitor `eDTU_used_percent` to right-size pool capacity.
3. Configure Hyperscale for databases exceeding 4 TB or needing instant backups and fast restore: Hyperscale uses a distributed storage architecture with up to 4 read replicas (`--read-replicas 2`); scale compute independently of storage with near-zero downtime and 100 TB capacity.
4. Implement geo-replication with `az sql db replica create --partner-server <server> --partner-resource-group <rg>` for disaster recovery; use auto-failover groups (`az sql failover-group create`) for automatic failover with a single connection endpoint — test failover regularly with `az sql failover-group set-primary`.
5. Enable Always Encrypted for column-level encryption of sensitive data: use SSMS wizard or SDK to create column master key (stored in Key Vault) and column encryption key; queries on encrypted columns work transparently with the `Column Encryption Setting=enabled` connection string parameter.
6. Configure auditing and threat detection: `az sql db audit-policy update --state Enabled --storage-account <account>` writes to Blob Storage or Log Analytics; enable Advanced Threat Protection (`az sql db threat-policy update --state Enabled`) for SQL injection detection, anomalous access, and brute-force alerts.
7. Use automatic tuning for self-optimizing performance: `az sql db update --set tags.automaticTuning='{"forceLastGoodPlan":"On","createIndex":"On","dropIndex":"On"}'`; review tuning recommendations in Azure Portal or via `sys.dm_db_tuning_recommendations` DMV before enabling auto-apply in production.
8. Build connection resilience with retry logic: use `SqlConnection` with `ConnectRetryCount=3 ConnectRetryInterval=10` in connection strings; implement `ExecutionStrategy` in Entity Framework Core (`options.EnableRetryOnFailure(maxRetryCount: 5, maxRetryDelay: TimeSpan.FromSeconds(30), errorNumbersToAdd: null)`) for transient fault handling.
9. Use managed identity for authentication: enable Entra ID admin on the server (`az sql server ad-admin create`), create contained users (`CREATE USER [app-identity] FROM EXTERNAL PROVIDER`), and connect with `Authentication=Active Directory Managed Identity` in connection strings — eliminate password rotation entirely.
10. Optimize query performance with Query Store (enabled by default): query `sys.query_store_runtime_stats` for regression detection, use `sys.dm_exec_query_stats` for top resource consumers, and add missing indexes identified in `sys.dm_db_missing_index_details` — validate index impact with `SET STATISTICS IO ON`.
11. Configure private endpoints for network isolation: `az sql server update --public-network-access Disabled` then create private endpoint connections; use VNet service endpoints as a lighter alternative, and restrict firewall rules to specific IP ranges with `az sql server firewall-rule create`.
12. Implement point-in-time restore for operational recovery: `az sql db restore --dest-name restored-db --time "2026-03-06T12:00:00Z"` restores to any point within the retention period (7-35 days); for long-term retention, configure LTR policies (`az sql db ltr-policy set --weekly-retention P4W --monthly-retention P12M`) for compliance.
