---
triggers: ["DuckDB", "duckdb", "duckdb query", "analytical query", "duckdb parquet", "olap database", "embedded analytics"]
tools_allowed: ["read_file", "write_file", "bash"]
category: database
---

# DuckDB

When working with DuckDB:

1. DuckDB is an embedded OLAP database (like SQLite for analytics) — no server needed; open with `duckdb mydb.duckdb` or use in-memory: `import duckdb; con = duckdb.connect()`.
2. Query files directly without loading: `SELECT * FROM 'data.parquet'`, `SELECT * FROM 'data.csv'`, `SELECT * FROM read_json_auto('data.json')`. Supports glob patterns: `SELECT * FROM 'logs/*.parquet'`.
3. Parquet is the preferred format: `COPY table TO 'output.parquet' (FORMAT PARQUET)` for writes; read with automatic schema detection and predicate pushdown for fast filtering.
4. Use window functions extensively — DuckDB is optimized for analytical queries: `SELECT *, SUM(amount) OVER (PARTITION BY user_id ORDER BY date ROWS BETWEEN UNBOUNDED PRECEDING AND CURRENT ROW) as running_total`.
5. Python integration: `con.sql("SELECT ...").df()` returns a Pandas DataFrame; `con.sql("SELECT ...").pl()` returns Polars; `duckdb.sql("SELECT * FROM df")` queries DataFrames directly without copying.
6. Use `CREATE TABLE ... AS SELECT` (CTAS) for materialized transformations; use `CREATE VIEW` for virtual tables that query on access.
7. S3/GCS/Azure integration: `SET s3_region='us-east-1'; SET s3_access_key_id='...'; SELECT * FROM 's3://bucket/path/*.parquet'` — reads remote Parquet files with predicate pushdown.
8. Install extensions: `INSTALL httpfs; LOAD httpfs;` for S3/HTTP access; `INSTALL spatial; LOAD spatial;` for geospatial queries; `INSTALL postgres_scanner` to query PostgreSQL directly.
9. Aggregate functions: `LIST()`, `HISTOGRAM()`, `QUANTILE_CONT()`, `APPROX_COUNT_DISTINCT()`, `STRING_AGG()`, `ARG_MIN()`/`ARG_MAX()` for the row with min/max value.
10. Use `PIVOT` and `UNPIVOT` for reshaping data: `PIVOT sales ON product USING SUM(amount)` transforms rows to columns.
11. JSON handling: `json_extract(col, '$.key')`, `json_extract_string()`, auto-detect with `read_json_auto()`; unnest JSON arrays with `UNNEST()`.
12. Performance: DuckDB uses vectorized execution and columnar storage; it auto-parallelizes queries across cores; use `EXPLAIN ANALYZE` to profile; set `SET threads TO 8` to control parallelism.
13. Export: `COPY (SELECT ...) TO 'output.csv' WITH (HEADER, DELIMITER ',')` or to Parquet, JSON, Excel formats.
14. Rust integration: use the `duckdb` crate — `Connection::open_in_memory()?; conn.execute_batch("CREATE TABLE ...")?; let mut stmt = conn.prepare("SELECT ...")?;`.
