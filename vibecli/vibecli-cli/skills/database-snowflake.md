---
triggers: ["Snowflake", "snowflake sql", "snowflake warehouse", "snowpark", "snowflake stage", "snowflake pipe", "data warehouse snowflake"]
tools_allowed: ["read_file", "write_file", "bash"]
category: database
---

# Snowflake

When working with Snowflake:

1. Snowflake separates compute (virtual warehouses) from storage — scale warehouses independently; use `ALTER WAREHOUSE wh SET WAREHOUSE_SIZE = 'XLARGE'` or enable auto-scaling: `MIN_CLUSTER_COUNT=1 MAX_CLUSTER_COUNT=3`.
2. Use `COPY INTO` for bulk loading: `COPY INTO my_table FROM @my_stage FILE_FORMAT = (TYPE = 'PARQUET')`. Create stages for S3/GCS/Azure: `CREATE STAGE my_stage URL='s3://bucket/' CREDENTIALS=(AWS_KEY_ID='...')`.
3. Snowpipe for continuous loading: `CREATE PIPE my_pipe AUTO_INGEST=TRUE AS COPY INTO table FROM @stage` — automatically loads files as they arrive in cloud storage via SQS/EventGrid notifications.
4. Time travel: `SELECT * FROM table AT(OFFSET => -3600)` or `AT(TIMESTAMP => '2024-01-01'::TIMESTAMP)` — query historical data up to 90 days; `UNDROP TABLE accidentally_dropped`.
5. Zero-copy cloning: `CREATE TABLE dev_table CLONE prod_table` — instant, no storage cost until data diverges; clone entire databases/schemas for development environments.
6. Semi-structured data: `VARIANT` column type stores JSON/Avro/Parquet; query with `:` notation: `SELECT data:user:name::STRING FROM events`. Use `FLATTEN()` to unnest arrays and objects.
7. Materialized views: `CREATE MATERIALIZED VIEW mv AS SELECT ...` — auto-maintained, transparent query acceleration; use for pre-aggregated dashboards and frequently-filtered dimensions.
8. Tasks and streams for ELT: `CREATE STREAM changes ON TABLE raw_events` captures CDC; `CREATE TASK transform_task SCHEDULE='5 MINUTE' AS INSERT INTO processed SELECT * FROM changes`.
9. User-defined functions: SQL, JavaScript, Python, or Java UDFs; `CREATE FUNCTION my_func(x INT) RETURNS INT LANGUAGE PYTHON RUNTIME_VERSION='3.9' AS $$ def my_func(x): return x * 2 $$`.
10. Snowpark for Python/Scala/Java DataFrames: `session.table("events").filter(col("type") == "click").group_by("user_id").agg(count("*"))` — pushes computation to Snowflake's engine.
11. Dynamic tables: `CREATE DYNAMIC TABLE cleaned_events TARGET_LAG='1 hour' WAREHOUSE=wh AS SELECT ... FROM raw_events` — declarative data pipelines that auto-refresh.
12. Access control: use RBAC with `GRANT SELECT ON TABLE t TO ROLE analyst`; row access policies: `CREATE ROW ACCESS POLICY region_policy AS (region VARCHAR) RETURNS BOOLEAN -> current_role() = 'ADMIN' OR region = 'US'`.
13. Cost management: use Resource Monitors (`CREATE RESOURCE MONITOR`) to set credit quotas; auto-suspend warehouses after inactivity (`AUTO_SUSPEND=300`); right-size warehouses using Query Profile.
14. Data sharing: `CREATE SHARE my_share; GRANT USAGE ON DATABASE db TO SHARE my_share` — share live data with other Snowflake accounts without copying.
