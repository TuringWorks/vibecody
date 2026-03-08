---
triggers: ["data lakehouse", "Delta Lake", "Apache Iceberg", "Apache Hudi", "data lake", "Parquet", "data catalog", "data mesh", "data governance", "medallion architecture"]
tools_allowed: ["read_file", "write_file", "bash"]
category: data
---

# Data Lakehouse Architecture

When working with data lakehouse platforms, table formats, and data governance:

1. Structure your lakehouse using the medallion architecture with three layers: bronze (raw ingestion, append-only, schema-on-read), silver (cleaned, deduplicated, conformed schemas with data type enforcement), and gold (business-level aggregates and feature tables optimized for consumption by analytics and ML).

2. Choose an open table format (Delta Lake, Apache Iceberg, or Apache Hudi) based on your ecosystem: Delta Lake for tight Spark/Databricks integration, Iceberg for engine-agnostic access (Spark, Trino, Flink, Dremio), and Hudi for upsert-heavy and CDC workloads; all three provide ACID transactions, schema evolution, and time travel.

3. Optimize Parquet files by targeting 128MB-1GB per file, using Snappy or Zstandard compression, setting row group sizes to 128MB, and leveraging column statistics (min/max) for predicate pushdown; avoid the small files problem by running compaction jobs that merge undersized files.

4. Implement schema evolution safely: add columns as nullable by default, use column mapping (by-name rather than by-position) to allow renames and drops, evolve nested struct fields incrementally, and maintain a schema registry or catalog that tracks the full schema history.

5. Leverage time travel for debugging, auditing, and reproducibility: query historical snapshots by version or timestamp, restore tables to previous versions for recovery, and set retention policies (e.g., 30 days) that balance storage cost with the ability to audit past states.

6. Register all tables in a data catalog (Unity Catalog, AWS Glue, Hive Metastore, or Polaris) that provides centralized metadata management, schema discovery, data lineage tracking, access control, and cross-engine table access through a unified namespace.

7. Implement data quality checks at each medallion layer using frameworks like Great Expectations, Deequ, or Delta Live Tables expectations; validate row counts, null ratios, referential integrity, statistical distributions, and business rules, and quarantine rows that fail checks.

8. Ingest change data capture (CDC) streams from operational databases using Debezium or database-native CDC; apply changes to lakehouse tables using merge (upsert) operations that match on primary keys, handle deletes with soft-delete flags or hard deletes per retention policy.

9. Design partition strategies aligned with query patterns: partition by date (year/month/day) for time-series data, by region or tenant for multi-tenant workloads; avoid over-partitioning (aim for partitions > 1GB), and use hidden partitioning (Iceberg) or generated columns (Delta) to decouple physical layout from logical queries.

10. Apply Z-ordering (Delta Lake) or sort-order optimization (Iceberg) to co-locate related data within files for multi-dimensional query predicates; choose Z-order columns based on the most common filter columns (e.g., user_id, event_type), and rerun optimization after significant data changes.

11. Enforce data governance policies with column-level access control, row-level security filters, data masking for PII (tokenization or hashing), and classification tags (sensitivity levels); audit all data access and maintain compliance with GDPR, CCPA, or HIPAA through automated policy enforcement.

12. Implement data mesh principles by organizing lakehouse tables into domain-owned data products; each domain team owns their bronze-to-gold pipeline, publishes gold tables as data products with SLAs (freshness, quality), and the platform team provides self-service infrastructure and federated governance.

13. Schedule and monitor maintenance operations: run compaction (bin-packing) to merge small files, vacuum/expire snapshots to reclaim storage, analyze tables to update column statistics, and set up alerting on table health metrics (file count, average file size, partition skew).

14. Enable cross-engine interoperability by using open table formats with REST catalogs; ensure that Spark, Trino, Flink, and Python (PyArrow/DuckDB) can all read and write the same tables without data duplication, and test query consistency across engines.
