---
triggers: ["Airflow", "Prefect", "Dagster", "pipeline orchestration", "DAG", "ETL orchestration", "data pipeline"]
tools_allowed: ["read_file", "write_file", "bash"]
category: data-engineering
---

# Data Pipeline Orchestration Best Practices

When working with data pipeline orchestration:

1. Design DAGs with idempotent tasks that produce the same result on re-execution — use date-partitioned writes, upserts instead of inserts, and deterministic transformations so that backfills and retries are safe; never rely on side effects like incrementing counters or sending notifications within core data tasks.
2. Use the right Airflow operators for each task — PythonOperator for custom logic, BashOperator for shell commands, sensor operators (FileSensor, ExternalTaskSensor) for waiting on external dependencies, and provider-specific operators (BigQueryOperator, S3ToRedshiftOperator) for managed service integrations rather than writing boilerplate API calls in PythonOperator.
3. Build Prefect flows with built-in retry logic (`@task(retries=3, retry_delay_seconds=60)`), result caching (`cache_key_fn` with task input hashing), and structured concurrency using `task.submit()` for parallel execution — leverage Prefect's automatic state tracking to resume failed flows from the point of failure.
4. Model Dagster pipelines as software-defined assets with `@asset` decorators — define IO managers for reading/writing to storage systems, use asset groups for logical organization, and leverage Dagster's asset lineage graph to understand upstream/downstream dependencies and data freshness.
5. Define task dependencies explicitly and use trigger rules to handle partial failures — in Airflow, use `trigger_rule=TriggerRule.ALL_DONE` for cleanup tasks, `NONE_FAILED` for conditional branches, and `ONE_SUCCESS` for fan-in patterns; avoid complex XCom-based branching in favor of clear DAG structure.
6. Implement backfill strategies that respect downstream dependencies — partition data by date, pass execution dates as parameters rather than using `datetime.now()`, and use catchup/backfill mechanisms (Airflow's `catchup=True`, Dagster's asset materialization) to reprocess historical data without disrupting current runs.
7. Manage secrets through dedicated secret backends — use Airflow's Connections and Variables with a secrets backend (AWS Secrets Manager, HashiCorp Vault), Prefect's Secret blocks, or Dagster's resource configuration rather than hardcoding credentials in DAG code or environment variables on worker nodes.
8. Set up monitoring and alerting for pipeline failures — configure email/Slack callbacks on task failure (`on_failure_callback` in Airflow), define SLAs with `sla` parameter for deadline tracking, use Prefect automations or Dagster sensors for proactive alerting, and track pipeline duration trends to catch performance regressions early.
9. Test DAGs locally before deployment — validate Airflow DAG parsing with `python dag_file.py` and `airflow dags test`, unit test individual task functions in isolation with mock inputs, use Prefect's local flow runner and Dagster's `materialize` for local asset testing, and run integration tests against staging environments.
10. Support parameterized runs for flexibility — use Airflow's `params` and Jinja templating, Prefect's flow parameters with type validation, and Dagster's run config and `@configurable` resources to allow runtime customization of file paths, date ranges, feature flags, and environment targets without code changes.
11. Integrate data quality checks at pipeline boundaries — use Great Expectations for schema validation, null checks, uniqueness constraints, and statistical distribution tests; run quality suites after ingestion and before publishing to downstream consumers; fail pipelines on critical quality violations and log warnings for non-critical anomalies.
12. Design scheduling patterns around data availability rather than fixed cron intervals — use sensor-based triggers that wait for upstream data to land, event-driven scheduling (Dagster sensors, Prefect event triggers) for real-time pipelines, and stagger schedules to avoid resource contention during peak processing windows.
