---
triggers: ["Cloud Functions", "Cloud Run", "GCP serverless", "google cloud functions", "cloud run jobs"]
tools_allowed: ["read_file", "write_file", "bash"]
category: devops
---

# Google Cloud Functions and Cloud Run

When working with Google Cloud Functions and Cloud Run:

1. Choose Cloud Functions for simple event-driven handlers (Pub/Sub, GCS, Firestore triggers) and Cloud Run for containerized services needing custom runtimes, WebSockets, streaming, or longer execution times (up to 60 minutes).

2. Use Cloud Functions 2nd gen (backed by Cloud Run) for all new functions — it provides concurrency per instance, longer timeouts (up to 60 minutes), traffic splitting, and Eventarc triggers: `gcloud functions deploy myFunc --gen2 --runtime=nodejs20 --trigger-http`.

3. Structure Cloud Functions with a clear entry point: `exports.handler = async (req, res) => {}` for HTTP, `exports.handler = async (event, context) => {}` for event-driven. Keep the handler thin and delegate to imported modules.

4. Deploy Cloud Run services from a Dockerfile or source with buildpacks: `gcloud run deploy my-service --source . --region us-central1`. Set `--min-instances=1` to avoid cold starts and `--concurrency=80` to match your app's threading model.

5. Use Cloud Run Jobs for batch and scheduled work — define a job that runs to completion: `gcloud run jobs create etl-job --image=gcr.io/proj/etl --tasks=10 --max-retries=3`. Trigger on schedule with Cloud Scheduler.

6. Configure CPU allocation wisely: Cloud Run defaults to CPU-only-during-requests — set `--cpu-always-allocated` if you need background processing. Use `--cpu=2 --memory=1Gi` and benchmark to find optimal settings.

7. Handle authentication with IAM: use `--no-allow-unauthenticated` for internal services, and validate ID tokens with `const ticket = await client.verifyIdToken({idToken, audience})`. Use service-to-service auth with automatic OIDC tokens.

8. Integrate with Eventarc for event-driven Cloud Run: `gcloud eventarc triggers create my-trigger --destination-run-service=my-service --event-filters="type=google.cloud.storage.object.v1.finalized"` — supports 90+ Google Cloud event types.

9. Use Secret Manager for credentials: `gcloud run deploy --set-secrets=DB_PASS=my-secret:latest` mounts secrets as environment variables or volumes. Never bake secrets into container images.

10. Set up VPC Connector for private network access: `--vpc-connector=my-connector` enables Cloud Run to reach Cloud SQL, Memorystore, and internal services without public endpoints.

11. Implement graceful shutdown — Cloud Run sends SIGTERM before killing instances. Handle it to drain connections and flush buffers: `process.on('SIGTERM', async () => { await db.close(); process.exit(0); })`. The shutdown window is 10 seconds by default.

12. Monitor with Cloud Logging structured JSON, Cloud Trace for latency analysis, and Cloud Monitoring custom metrics. Set up alerts on error rate and p99 latency: `gcloud monitoring policies create --policy-from-file=alert.json`.
