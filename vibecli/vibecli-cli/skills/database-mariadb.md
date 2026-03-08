---
triggers: ["MariaDB", "mariadb", "mariadb query", "galera cluster", "mariadb columnstore", "mariadb replication"]
tools_allowed: ["read_file", "write_file", "bash"]
category: database
---

# MariaDB

When working with MariaDB:

1. MariaDB is a MySQL-compatible fork with additional features — most MySQL syntax, tools, and connectors work unchanged; use MariaDB for Galera Cluster, ColumnStore, or temporal tables.
2. Use InnoDB (default) for OLTP workloads; use ColumnStore engine for OLAP: `CREATE TABLE analytics (...) ENGINE=ColumnStore` — columnar storage optimized for aggregations on large datasets.
3. Galera Cluster for multi-master replication: all nodes accept reads and writes with synchronous replication; `wsrep_cluster_size` must be odd (3/5/7) for quorum; use `wsrep_sst_method=mariabackup` for state transfers.
4. Temporal tables (system-versioned): `CREATE TABLE orders (..., PERIOD FOR SYSTEM_TIME) WITH SYSTEM VERSIONING` — automatic row history; query past state: `SELECT * FROM orders FOR SYSTEM_TIME AS OF '2024-01-01'`.
5. Sequences: `CREATE SEQUENCE order_seq START WITH 1 INCREMENT BY 1; SELECT NEXT VALUE FOR order_seq` — more flexible than `AUTO_INCREMENT` for distributed systems.
6. Window functions: full support for `ROW_NUMBER()`, `RANK()`, `DENSE_RANK()`, `NTILE()`, `LAG()`, `LEAD()`, `FIRST_VALUE()`, `LAST_VALUE()` with `OVER (PARTITION BY ... ORDER BY ...)`.
7. JSON support: `JSON_TABLE()` to convert JSON to relational rows; `JSON_ARRAYAGG()` and `JSON_OBJECTAGG()` for aggregation; virtual columns on JSON paths for indexing.
8. MaxScale for proxy/load balancing: read-write splitting, connection pooling, query filtering, and failover — configure with `readwritesplit` router for automatic read/write routing.
9. Use `mariadb-backup` (based on Percona XtraBackup) for hot, non-blocking physical backups; supports incremental backups and compressed streams.
10. Oracle compatibility mode: `SET SQL_MODE='ORACLE'` enables PL/SQL-compatible syntax including packages, `%TYPE`, `%ROWTYPE`, and `EXCEPTION` handling for migrations from Oracle.
