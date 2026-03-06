---
triggers: ["ETL", "data pipeline", "data validation", "ELT", "data engineering", "Airflow", "dbt"]
tools_allowed: ["read_file", "write_file", "bash"]
category: workflow
---

# Data Pipeline Design

When building data pipelines:

1. ETL vs ELT: use ELT (extract, load, transform) for cloud data warehouses — transform in SQL
2. Idempotent operations: re-running a pipeline should produce the same result — no duplicates
3. Use dbt for SQL transformations: models, tests, documentation, lineage tracking
4. Validate data at ingestion: schema validation, null checks, range checks, uniqueness
5. Use Airflow/Dagster/Prefect for orchestration — DAGs define dependencies between tasks
6. Partition data by date for efficient processing — process only new/changed partitions
7. Dead letter queue: route failed records for investigation — don't lose data silently
8. Schema evolution: handle added/removed/renamed columns — use schema registry
9. Backfill strategy: support re-processing historical data without affecting current pipeline
10. Monitoring: track record counts, processing time, error rates, data freshness
11. Testing: unit test transformations, integration test with sample data, data quality checks
12. Lineage: document where data comes from and where it goes — essential for compliance
