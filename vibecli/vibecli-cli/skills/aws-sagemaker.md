---
triggers: ["SageMaker", "aws sagemaker", "sagemaker endpoint", "sagemaker pipeline", "ml training aws", "sagemaker studio", "model deployment aws"]
tools_allowed: ["read_file", "write_file", "bash"]
requires_bins: ["aws"]
category: cloud-aws
---

# AWS SageMaker ML Platform

When working with AWS SageMaker:

1. Use SageMaker Pipelines for end-to-end ML workflows: define `ProcessingStep` (data prep), `TrainingStep` (model training), `EvaluationStep` (metrics), `ConditionStep` (quality gate), and `RegisterModel` step to automate the full lifecycle with version tracking and lineage.
2. Choose the right instance type for training: use `ml.g5.xlarge` for single-GPU experiments, `ml.p4d.24xlarge` for distributed training, and spot instances (`use_spot_instances=True`, `max_wait_seconds`) to save up to 90% on training costs with automatic checkpointing to S3.
3. Deploy real-time endpoints with auto-scaling: configure `TargetTrackingScalingPolicy` on `SageMakerVariantInvocationsPerInstance` (target: 100-500), set `MinInstanceCount: 1` and `MaxInstanceCount` based on peak traffic, and use `ml.g5` instances for GPU inference.
4. Use serverless inference (`ServerlessConfig: {MemorySizeInMB: 2048, MaxConcurrency: 20}`) for intermittent traffic patterns to eliminate idle costs; cold starts are 1-2 seconds, so pair with provisioned concurrency for latency-sensitive endpoints.
5. Implement batch transform for offline predictions: call `create_transform_job` with S3 input/output paths, set `BatchStrategy: "MultiRecord"` and `MaxPayloadInMB: 6` to process large datasets efficiently without maintaining a persistent endpoint.
6. Use SageMaker Feature Store for ML feature management: ingest features with `put_record()` to both online (low-latency DynamoDB) and offline (S3 Parquet) stores; query offline features with Athena for training and online features for real-time inference.
7. Register models in SageMaker Model Registry with approval workflows: set `ModelApprovalStatus: "PendingManualApproval"`, attach model metrics and bias reports, then promote to `"Approved"` via CI/CD or manual review before production deployment.
8. Use SageMaker Inference Recommender to benchmark model performance across instance types: run `create_inference_recommendations_job` to get cost-per-inference, latency percentiles, and throughput metrics to select the optimal deployment configuration.
9. Implement A/B testing with production variants: deploy multiple model versions on the same endpoint with traffic splitting (`InitialVariantWeight: 0.1` for the new model) and monitor `Invocations`, `ModelLatency`, and custom CloudWatch metrics before shifting traffic.
10. Use SageMaker Processing jobs with `ScriptProcessor` or `SKLearnProcessor` for data preparation, evaluation, and post-processing; mount S3 data with `ProcessingInput`, write results with `ProcessingOutput`, and keep processing logic in version-controlled scripts.
11. Implement model monitoring with `ModelQualityMonitoringJobDefinition` and `DataQualityMonitoringJobDefinition`: schedule hourly/daily checks against a baseline, detect data drift and prediction quality degradation, and trigger retraining pipelines via EventBridge on violations.
12. Secure SageMaker resources by deploying in a VPC with private subnets, enabling encryption at rest (`KmsKeyId` on training jobs, endpoints, and notebooks), restricting S3 access with VPC endpoints, and using IAM execution roles scoped to specific S3 prefixes and ECR repositories.
