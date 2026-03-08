---
triggers: ["Dremio", "dremio", "dremio query", "data lakehouse", "dremio reflections", "apache iceberg dremio", "dremio sonar"]
tools_allowed: ["read_file", "write_file", "bash"]
category: database
---

# Dremio

When working with Dremio:

1. Dremio is a lakehouse query engine — query data lakes (S3/ADLS/GCS) with standard SQL without ETL; it uses Apache Iceberg as its native table format and Apache Arrow for in-memory columnar processing.
2. Connect data sources: Dremio supports S3, ADLS, GCS, HDFS, NAS, PostgreSQL, MySQL, SQL Server, Oracle, Elasticsearch, MongoDB, and more. Add sources via the UI or REST API.
3. Spaces and folders organize virtual datasets: create a Space for a team, organize datasets in folders, and grant access via role-based permissions.
4. Reflections for acceleration: raw reflections (materialized subsets) and aggregation reflections (pre-computed aggregates) transparently accelerate queries. `ALTER DATASET events CREATE RAW REFLECTION rfl USING DISPLAY (col1, col2) DISTRIBUTE BY (col1)`.
5. Query Iceberg tables directly: `SELECT * FROM s3_source."bucket"."path/to/iceberg_table"`. Supports time travel, schema evolution, partition evolution, and hidden partitioning.
6. Use ANSI SQL with extensions: window functions, CTEs, PIVOT, FLATTEN (for nested JSON/Parquet), `CONVERT_FROM(col, 'JSON')` for semi-structured data.
7. Virtual datasets: create views over raw data for self-service analytics: `CREATE VDS my_space.clean_events AS SELECT * FROM raw.events WHERE valid = true`. Changes propagate to all dependent datasets.
8. Dremio Arctic: managed Iceberg catalog with git-like branching — `CREATE BRANCH dev FROM main`, experiment with schema/data changes, then `MERGE BRANCH dev INTO main`.
9. COPY INTO for ingestion: `COPY INTO target_table FROM '@source/path' FILE_FORMAT = (TYPE = 'PARQUET')` loads data from object storage into managed Iceberg tables.
10. REST API: `POST /api/v3/sql` to execute queries; `GET /api/v3/catalog` to browse datasets; `POST /api/v3/reflection` to manage reflections programmatically.
11. Connect BI tools: ODBC/JDBC drivers for Tableau, Power BI, Looker; Arrow Flight for high-performance programmatic access from Python (pyarrow), Rust, or Go.
12. Performance tuning: use `EXPLAIN PLAN` to inspect query plans; check reflection usage with `sys.reflections` and `sys.materializations`; monitor with `sys."query-history"`.
13. Row-level and column-level security: use `GRANT SELECT ON COLUMNS (col1, col2)` for column masking; define row access policies with user context functions.
14. Dremio Sonar: the SQL analytics engine supports sub-second queries on petabyte-scale data through reflections and Arrow-based execution.
