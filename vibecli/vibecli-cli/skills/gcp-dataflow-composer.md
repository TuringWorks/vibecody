---
triggers: ["Dataflow", "cloud composer", "apache beam", "gcp dataflow", "beam pipeline", "cloud composer dag", "dataproc spark", "gcp data pipeline"]
tools_allowed: ["read_file", "write_file", "bash"]
requires_bins: ["gcloud"]
category: cloud-gcp
---

# GCP Dataflow & Cloud Composer

When working with Dataflow and Cloud Composer:

1. Build Apache Beam pipelines with the Python SDK using `beam.Pipeline(options=PipelineOptions(['--runner=DataflowRunner', '--project=P', '--region=R', '--temp_location=gs://...']))` and prefer `beam.Map`, `beam.FlatMap`, and `beam.ParDo` transforms with type hints for clarity and optimization.
2. For streaming pipelines, apply windowing with `beam.WindowInto(window.FixedWindows(60))` or sliding/session windows, and use `beam.CombineGlobally().without_defaults()` for aggregations; set `--streaming` and handle late data with `allowed_lateness` and `accumulation_mode=ACCUMULATING`.
3. Use Dataflow Flex Templates (`gcloud dataflow flex-template build gs://BUCKET/template.json --image=IMAGE --sdk-language=PYTHON`) for parameterized, reusable pipelines that can be launched via API, Cloud Scheduler, or Composer without redeploying code.
4. Optimize Dataflow costs by setting `--max-num-workers`, `--autoscaling-algorithm=THROUGHPUT_BASED`, and `--machine-type=n1-standard-4`; use `--dataflow-service-options=enable_hot_key_logging` to detect key skew that causes worker imbalance.
5. Use `beam.io.ReadFromBigQuery(query='SELECT ...')` and `beam.io.WriteToBigQuery(table='P:D.T', write_disposition='WRITE_APPEND', method='STREAMING_INSERTS')` for BigQuery integration; prefer `FILE_LOADS` method for batch pipelines to reduce costs.
6. Deploy Cloud Composer 2 environments with `gcloud composer environments create ENV --image-version=composer-2.x.x-airflow-2.x.x --environment-size=small` and configure workloads resources to right-size the Autopilot GKE cluster underlying Composer.
7. Structure Airflow DAGs with `@dag` and `@task` decorators (TaskFlow API) for Python-native workflows; use `GCSToGCSOperator`, `BigQueryInsertJobOperator`, and `DataflowStartFlexTemplateOperator` from the Google provider package instead of `BashOperator` with `gcloud` commands.
8. Parameterize DAGs with Airflow Variables and Connections stored in Secret Manager by configuring `secrets_backend=airflow.providers.google.cloud.secrets.secret_manager.CloudSecretManagerBackend` in `airflow.cfg` overrides.
9. Implement idempotent tasks by using `execution_date` as a partition key, enabling BigQuery `WRITE_TRUNCATE` per-partition loads, and setting `depends_on_past=True` with `wait_for_downstream=True` for sequential dependency chains.
10. For Spark workloads, use Dataproc Serverless (`gcloud dataproc batches submit spark --class=Main --jars=gs://...`) for ephemeral jobs or Dataproc clusters with autoscaling policies for interactive notebooks; trigger from Composer with `DataprocSubmitJobOperator`.
11. Use Data Fusion for visual ETL pipeline design when non-engineers need to build integrations; deploy with `gcloud data-fusion instances create` and export pipelines as JSON for version control and CI/CD deployment.
12. Monitor pipeline health with Dataflow job metrics in Cloud Monitoring (`dataflow.googleapis.com/job/element_count`, `system_lag`); set up alerts on `system_lag > 300s` for streaming jobs, and use Composer's built-in Airflow UI plus `dag_processing_time` metrics to detect DAG parse delays.
