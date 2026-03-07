---
triggers: ["Vertex AI", "vertex ai", "gcp ml", "vertex pipeline", "vertex endpoint", "gemini api gcp", "vertex model", "google ai platform"]
tools_allowed: ["read_file", "write_file", "bash"]
requires_bins: ["gcloud"]
category: cloud-gcp
---

# GCP Vertex AI

When working with Vertex AI:

1. Use the Gemini API via Vertex AI SDK with `vertexai.init(project=PROJECT, location=REGION)` and `GenerativeModel('gemini-1.5-pro').generate_content(prompt)` for enterprise-grade access with IAM, VPC-SC, and data residency controls versus the consumer API.
2. Create training pipelines with `aiplatform.CustomContainerTrainingJob(display_name='...', container_uri='gcr.io/PROJECT/train:v1', model_serving_container_image_uri='...')` and call `.run()` to submit; always specify `machine_type`, `accelerator_type`, and `accelerator_count` explicitly for reproducibility.
3. Deploy models to endpoints with traffic splitting: `endpoint.deploy(model, traffic_percentage=10, machine_type='n1-standard-4', min_replica_count=1, max_replica_count=5)` to canary new model versions before shifting 100% traffic.
4. Use Vertex AI Vector Search (formerly Matching Engine) with `aiplatform.MatchingEngineIndex.create_tree_ah_index()` for billion-scale nearest-neighbor search; deploy to an index endpoint and query with `index_endpoint.find_neighbors()` for RAG applications.
5. Leverage Model Garden to deploy open-source models (Llama, Gemma, Mistral) with one click or via `aiplatform.Model.upload()` using pre-built serving containers; compare performance in Vertex AI Model Evaluation before production deployment.
6. Structure experiments with `aiplatform.init(experiment='exp-name')` and log metrics inside runs with `aiplatform.log_metrics({'accuracy': 0.95, 'f1': 0.93})` to track hyperparameter tuning results and compare runs in the console.
7. Use Feature Store for online/offline feature serving: `aiplatform.FeatureOnlineStore.create_bigtable_store()` for low-latency serving and BigQuery offline store for training; keep feature definitions in a Feature Registry for team-wide reuse.
8. Run batch predictions with `model.batch_predict(job_display_name='batch-jan', gcs_source='gs://input/*.jsonl', gcs_destination_prefix='gs://output/', machine_type='n1-standard-8')` for cost-efficient inference on large datasets without maintaining an always-on endpoint.
9. Build ML pipelines with Kubeflow Pipelines SDK v2: define components with `@component` decorators, compile with `compiler.Compiler().compile(pipeline, 'pipeline.json')`, and submit via `aiplatform.PipelineJob(template_path='pipeline.json').run()`.
10. Use custom serving containers by implementing a health check at `/health` and prediction handler at `/predict`; build on the pre-built prediction container base images (`us-docker.pkg.dev/vertex-ai/prediction/...`) for GPU driver compatibility.
11. Control costs by setting `max_replica_count=0` on endpoints to scale to zero when idle (Dedicated VM), using Spot VMs for training jobs with `scheduling.strategy='SPOT'`, and monitoring `aiplatform.googleapis.com/prediction/online/cpu/utilization`.
12. Secure the pipeline with VPC-SC perimeters around the Vertex AI service, CMEK encryption on datasets and models with `--encryption-spec-key-name=projects/P/locations/L/keyRings/R/cryptoKeys/K`, and IAM roles scoped to `roles/aiplatform.user` for data scientists.
