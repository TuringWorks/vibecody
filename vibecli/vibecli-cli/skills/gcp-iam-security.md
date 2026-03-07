---
triggers: ["GCP IAM", "gcp iam", "service account", "workload identity federation", "vpc service controls", "secret manager gcp", "cloud kms", "gcp security"]
tools_allowed: ["read_file", "write_file", "bash"]
requires_bins: ["gcloud"]
category: cloud-gcp
---

# GCP IAM & Security

When working with GCP IAM and security:

1. Follow the principle of least privilege by granting predefined roles at the narrowest resource scope; use `gcloud projects add-iam-policy-binding PROJECT --member=serviceAccount:SA --role=roles/storage.objectViewer --condition=...` with IAM Conditions for time-bound or resource-attribute-based access.
2. Use Workload Identity Federation instead of service account keys for external workloads: create a pool with `gcloud iam workload-identity-pools create POOL --location=global`, add an OIDC/AWS provider, and map claims with `--attribute-mapping='google.subject=assertion.sub'`.
3. Eliminate service account keys entirely; for CI systems, use Workload Identity Federation with GitHub Actions (`google-github-actions/auth@v2`), and for GCP-native compute (GCE, GKE, Cloud Run), attach service accounts directly to the resource.
4. Configure organization policies to enforce security guardrails: `gcloud org-policies set-policy policy.yaml` with constraints like `iam.disableServiceAccountKeyCreation`, `compute.requireShieldedVm`, and `storage.uniformBucketLevelAccess`.
5. Set up VPC Service Controls with `gcloud access-context-manager perimeters create` to define a security perimeter around sensitive services (BigQuery, GCS, Vertex AI); use ingress/egress rules for controlled cross-perimeter access rather than bridge perimeters.
6. Store secrets in Secret Manager with `gcloud secrets create SECRET --data-file=secret.txt` and access them at runtime via `secretmanager.Client().access_secret_version(name='projects/P/secrets/S/versions/latest')`; never embed secrets in environment variables or container images.
7. Use Cloud KMS for envelope encryption: create a key ring and key with `gcloud kms keys create KEY --keyring=RING --location=LOCATION --purpose=encryption`, wrap data encryption keys (DEKs) with the KEK, and enable automatic key rotation with `--rotation-period=90d`.
8. Enable Binary Authorization for GKE with attestors backed by KMS keys; require signed attestations from your CI/CD pipeline using `gcloud container binauthz attestations create` before images can be deployed to production clusters.
9. Audit all IAM changes by ensuring Admin Activity logs are always on (they are by default) and enabling Data Access logs for sensitive services; export to BigQuery with `gcloud logging sinks create` for long-term analysis and compliance reporting.
10. Use IAM Recommender to identify and remove excess permissions: review recommendations with `gcloud recommender recommendations list --recommender=google.iam.policy.Recommender` and apply them to shrink roles from broad (Editor) to tight predefined or custom roles.
11. Implement Security Command Center (SCC) Premium for continuous vulnerability scanning, threat detection, and compliance monitoring; use `gcloud scc findings list` to programmatically process findings and integrate with PagerDuty or Slack for alerting.
12. For multi-tenant environments, use separate projects per tenant with a shared VPC host project; apply folder-level IAM bindings and organization policies for consistent governance, and use service account impersonation (`--impersonate-service-account`) for cross-project operations.
