---
triggers: ["BigQuery", "bigquery", "gcp bigquery", "bq query", "bigquery ml", "bigquery streaming", "bigquery partition"]
tools_allowed: ["read_file", "write_file", "bash"]
requires_bins: ["gcloud"]
category: cloud-gcp
---

# GCP BigQuery

When working with BigQuery:

1. Always partition tables by date or timestamp columns using `TIME_UNIT_COLUMN` partitioning (`--time_partitioning_field`) and add clustering on high-cardinality filter columns to minimize slot usage and scanned bytes.
2. Use `bq query --dry_run` or the `dryRun` job configuration flag to estimate bytes processed before running expensive queries; set per-user and per-project `maximumBytesBilled` to prevent runaway costs.
3. For streaming inserts, use the `insertAll` API with `insertId` for deduplication, but prefer the Storage Write API (`BigQueryWriteClient.appendRows`) for higher throughput and exactly-once semantics via committed streams.
4. Create materialized views with `CREATE MATERIALIZED VIEW ... AS SELECT` for frequently aggregated queries; BigQuery automatically rewrites base-table queries to hit the materialized view when beneficial.
5. Use `EXPORT DATA OPTIONS(uri='gs://bucket/path/*.parquet', format='PARQUET')` for large result sets instead of downloading through the REST API to avoid memory limits and reduce egress costs.
6. Schedule recurring queries with `bq mk --transfer_config --data_source=scheduled_query --target_dataset=...` or the Data Transfer Service API, using parameterized `@run_date` for incremental loads.
7. Enable BI Engine reservations on hot datasets via `bq update --bi_reservation --project_id=... --reservation_size=...` to accelerate dashboards with in-memory caching.
8. For BigQuery ML, train models inline with `CREATE OR REPLACE MODEL dataset.model OPTIONS(model_type='LOGISTIC_REG') AS SELECT ...` and evaluate with `ML.EVALUATE`; export to Vertex AI for serving with `bq extract --model`.
9. Use `INFORMATION_SCHEMA.JOBS_BY_PROJECT` to audit slot consumption and identify expensive queries; set up custom cost dashboards filtering by `total_bytes_billed` and `user_email`.
10. Apply column-level security with `ALTER TABLE t SET OPTIONS (labels=[('sensitivity','high')])` and policy tags from Data Catalog to restrict PII access without duplicating data.
11. Use `bq load --source_format=PARQUET --hive_partitioning_mode=AUTO` for external data loads and prefer columnar formats (Parquet, ORC) over CSV/JSON to reduce load costs and improve schema inference.
12. Grant `roles/bigquery.dataViewer` at the dataset level and `roles/bigquery.jobUser` at the project level as the minimal IAM pair; never grant `roles/bigquery.admin` to service accounts running scheduled workloads.
