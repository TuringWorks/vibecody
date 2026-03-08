---
triggers: ["MLOps", "ML pipeline", "model training", "feature store", "model registry", "model serving", "experiment tracking", "MLflow", "Kubeflow", "model monitoring"]
tools_allowed: ["read_file", "write_file", "bash"]
category: data
---

# MLOps and Machine Learning Operations

When working with ML pipelines, model lifecycle management, and production ML systems:

1. Orchestrate ML pipelines as DAGs using tools like Kubeflow Pipelines, Airflow, or Prefect; define each step (data extraction, preprocessing, training, evaluation, deployment) as an isolated containerized component with explicit input/output artifacts and versioned dependencies.

2. Design a feature store with separate offline (batch) and online (low-latency) stores; compute features once and share across training and serving to prevent training-serving skew, version feature definitions, track feature lineage back to raw data sources, and document each feature's business meaning and computation logic.

3. Track every experiment with MLflow, Weights & Biases, or Neptune: log hyperparameters, metrics at each epoch, model artifacts, dataset versions, git commit hashes, and environment specifications; use experiment comparison views to identify the best-performing configurations.

4. Maintain a model registry (MLflow Model Registry, Vertex AI Model Registry) as the single source of truth for production-ready models; enforce stage transitions (staging, canary, production, archived) with approval gates, attach validation reports to each version, and tag models with metadata (owner, use case, data lineage).

5. Serve models behind a standardized inference API using TensorFlow Serving, Triton Inference Server, or a custom FastAPI/gRPC service; implement model versioning in the endpoint path, support batched inference for throughput, and set latency SLOs with circuit breakers for graceful degradation.

6. Implement A/B testing for model rollouts by routing a percentage of traffic to the challenger model; define success metrics (accuracy, business KPIs, latency) and statistical significance thresholds before starting the experiment, and automate rollback if the challenger underperforms.

7. Monitor models in production for data drift (input feature distribution shift) and concept drift (degraded prediction accuracy); use statistical tests (KS test, PSI, Jensen-Shannon divergence) on incoming data distributions, set threshold-based alerts, and trigger automated retraining when drift exceeds tolerances.

8. Version datasets alongside code using DVC (Data Version Control) or lakehouse table snapshots; store data in content-addressable storage, link dataset versions to experiment runs, and ensure that any model can be reproduced by checking out the corresponding code commit and data version.

9. Automate hyperparameter tuning with Optuna, Ray Tune, or SageMaker HPO; use Bayesian optimization or Hyperband for efficient search, define the search space declaratively, set early stopping criteria to kill underperforming trials, and log all trial results to the experiment tracker.

10. Manage GPU cluster resources with Kubernetes and operators (KubeRay, Volcano, or Run:ai); implement job queuing with priority classes, set resource quotas per team, use spot/preemptible instances for training with checkpointing for fault tolerance, and right-size GPU allocations based on utilization metrics.

11. Optimize inference performance through model quantization (INT8/FP16), pruning (remove low-magnitude weights), knowledge distillation (train smaller student models), ONNX Runtime conversion for cross-framework optimization, and batching strategies that balance latency and throughput.

12. Build CI/CD for ML that validates not just code quality (linting, unit tests) but also model quality: run training on a data subset, assert minimum metric thresholds, validate model size and latency against SLOs, scan for bias with fairness metrics (demographic parity, equalized odds), and gate deployments on all checks passing.

13. Implement model explainability as a production requirement: integrate SHAP or LIME for feature importance, log explanation artifacts alongside predictions for regulated use cases, build dashboards showing global feature importance trends, and provide per-prediction explanations via API for downstream consumers.

14. Design for reproducibility end-to-end: pin all library versions in lock files, set random seeds for all stochastic operations, use deterministic data loading (sorted inputs, fixed splits), containerize the training environment, and document the full pipeline configuration so any model version can be exactly reproduced.
