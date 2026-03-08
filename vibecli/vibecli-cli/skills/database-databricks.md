---
triggers: ["Databricks", "databricks sql", "Delta Lake", "delta table", "Unity Catalog", "databricks notebook", "lakehouse", "spark sql databricks"]
tools_allowed: ["read_file", "write_file", "bash"]
category: database
---

# Databricks / Delta Lake

When working with Databricks:

1. Delta Lake is the default storage format — ACID transactions on data lakes; create tables: `CREATE TABLE events USING DELTA LOCATION 's3://bucket/events'` or `df.write.format("delta").save("/path")`.
2. Use Unity Catalog for governance: `CREATE CATALOG my_catalog; CREATE SCHEMA my_catalog.my_schema; CREATE TABLE my_catalog.my_schema.users (...)`. Three-level namespace: catalog.schema.table.
3. Time travel: `SELECT * FROM events VERSION AS OF 5` or `TIMESTAMP AS OF '2024-01-01'`; `DESCRIBE HISTORY events` shows all versions; `RESTORE TABLE events TO VERSION AS OF 3` to rollback.
4. MERGE for upserts: `MERGE INTO target USING source ON target.id = source.id WHEN MATCHED THEN UPDATE SET * WHEN NOT MATCHED THEN INSERT *` — efficient CDC and SCD Type 2 patterns.
5. Z-ORDER for query optimization: `OPTIMIZE events ZORDER BY (user_id, date)` co-locates related data for faster filtering; use on columns frequently in WHERE/JOIN clauses (up to 4 columns).
6. Liquid clustering (Databricks-specific, replaces Z-ORDER + partitioning): `CREATE TABLE t CLUSTER BY (col1, col2)` with automatic incremental clustering — simpler and more adaptive.
7. Photon engine: enable for 2-8x faster SQL/DataFrame workloads; automatically activated on Photon-capable clusters; best for aggregations, joins, and string operations.
8. Structured Streaming: `spark.readStream.format("delta").table("events").writeStream.format("delta").trigger(availableNow=True).toTable("agg")` for incremental processing.
9. SQL Warehouses for BI: use Serverless SQL Warehouses for ad-hoc queries and dashboard backends; connect Tableau/Power BI via JDBC/ODBC or the Databricks connector.
10. Use `databricks-sdk` (Python) or REST API for automation: workspace management, job scheduling, cluster operations, Unity Catalog administration.
11. Databricks SQL: full ANSI SQL with extensions — window functions, CTEs, PIVOT/UNPIVOT, QUALIFY, lateral views, higher-order functions (`TRANSFORM`, `FILTER`, `AGGREGATE`).
12. Performance: use `ANALYZE TABLE events COMPUTE STATISTICS FOR ALL COLUMNS` for optimizer stats; `EXPLAIN EXTENDED` for query plans; `VACUUM events RETAIN 168 HOURS` to clean up old files.
13. MLflow integration: track experiments, log models, register in Model Registry, deploy as serving endpoints — all within the Databricks workspace.
14. Delta Sharing: share data across organizations without copying: `CREATE SHARE my_share; ALTER SHARE my_share ADD TABLE my_catalog.my_schema.events`.
