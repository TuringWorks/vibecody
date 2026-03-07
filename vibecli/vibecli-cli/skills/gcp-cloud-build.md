---
triggers: ["Cloud Build", "gcp cloud build", "cloud deploy", "artifact registry", "cloud build trigger", "gcp cicd", "cloud build yaml"]
tools_allowed: ["read_file", "write_file", "bash"]
requires_bins: ["gcloud"]
category: cloud-gcp
---

# GCP Cloud Build CI/CD

When working with Cloud Build:

1. Structure `cloudbuild.yaml` with explicit step dependencies using `waitFor` to parallelize independent steps: set `waitFor: ['-']` for steps that can start immediately and `waitFor: ['step-id']` for dependent steps to reduce total build time.
2. Use substitutions for environment-specific values: define `substitutions: { _ENV: 'staging', _REGION: 'us-central1' }` in the config and override at trigger time with `--substitutions=_ENV=prod`; prefix custom substitutions with `_` to avoid conflicts with built-in variables.
3. Create build triggers with `gcloud builds triggers create github --repo-name=REPO --branch-pattern='^main$' --build-config=cloudbuild.yaml` and use `--include-build-logs=INCLUDE_BUILD_LOGS_WITH_STATUS` to surface build logs in GitHub commit status checks.
4. Push images to Artifact Registry instead of Container Registry: `gcloud artifacts repositories create REPO --repository-format=docker --location=REGION` and use `${_REGION}-docker.pkg.dev/${PROJECT_ID}/REPO/IMAGE` as your image path for IAM-scoped access and vulnerability scanning.
5. Use private worker pools (`gcloud builds worker-pools create POOL --region=REGION --peered-network=VPC`) for builds that need VPC access, larger machine types, or custom networking; specify `options: { pool: { name: 'projects/P/locations/L/workerPools/POOL' } }`.
6. Implement Cloud Deploy pipelines with `gcloud deploy releases create REL-001 --delivery-pipeline=PIPELINE --region=REGION` for progressive rollouts; define stages (dev, staging, prod) in `clouddeploy.yaml` with approval gates on production targets.
7. Cache build artifacts between steps using `/workspace` (shared across steps) and across builds using `gsutil cp` to/from GCS in first/last steps; for Docker, use `--cache-from` with a previously pushed image layer to speed up rebuilds.
8. Enable vulnerability scanning on Artifact Registry with `gcloud artifacts docker images scan IMAGE` and set up Binary Authorization policies to block deployment of images with CRITICAL CVEs; use on-demand scanning in the build pipeline before pushing.
9. Integrate Skaffold with Cloud Deploy by defining `skaffold.yaml` with build/deploy profiles; Cloud Deploy uses Skaffold under the hood, so `skaffold render` and `skaffold apply` phases map to your pipeline stages automatically.
10. Set build timeouts with `timeout: '1200s'` in `cloudbuild.yaml` and configure the service account with `options: { logging: CLOUD_LOGGING_ONLY }` to reduce costs; grant only `roles/cloudbuild.builds.builder` and the minimum additional roles the build needs.
11. Use approval gates in Cloud Deploy with `gcloud deploy targets update prod --require-approval` and approve releases with `gcloud deploy rollouts approve ROLLOUT --delivery-pipeline=PIPELINE` to enforce manual review before production deployments.
12. Monitor build performance with `cloudbuild.googleapis.com/build/trigger_build_count` and `build_duration` metrics; set up alerts for build failures using Cloud Monitoring notification channels and create dashboards to track build success rates by trigger.
