---
triggers: ["Cloud Storage", "gcs", "gcp storage", "google cloud storage", "gcs bucket", "signed URL gcp", "storage lifecycle"]
tools_allowed: ["read_file", "write_file", "bash"]
requires_bins: ["gcloud"]
category: cloud-gcp
---

# GCP Cloud Storage

When working with Cloud Storage:

1. Create buckets with uniform bucket-level access (`gsutil mb -b on gs://BUCKET`) to simplify permissions; this disables per-object ACLs and relies solely on IAM, which is easier to audit and manage at scale.
2. Generate signed URLs for temporary access using `storage.Client().bucket('b').blob('obj').generate_signed_url(expiration=timedelta(hours=1), method='GET')` in the Python SDK; prefer V4 signatures and keep expiration under 7 days.
3. Configure lifecycle rules in JSON or with `gsutil lifecycle set config.json gs://BUCKET` to automatically transition objects to Nearline/Coldline/Archive after N days and delete old versions, reducing storage costs by 50-90% for aging data.
4. Enable Autoclass on buckets with `--autoclass` to let GCP automatically move objects between storage classes based on access patterns; this eliminates manual lifecycle tuning for unpredictable workloads.
5. Use parallel composite uploads for large files with `gsutil -o 'GSUtil:parallel_composite_upload_threshold=150M' cp bigfile gs://BUCKET/` and resumable uploads via `blob.upload_from_filename(timeout=300)` in the SDK to handle network interruptions.
6. Set up Pub/Sub notifications with `gsutil notification create -t TOPIC -f json gs://BUCKET` to trigger Cloud Functions or downstream processing on object create/delete/archive events without polling.
7. Use dual-region or multi-region buckets (`-l US` or `-l US-EAST1+US-WEST1`) for disaster recovery with turbo replication (`--rpo=ASYNC_TURBO`) to achieve sub-15-minute RPO for critical data.
8. Enable Object Versioning with `gsutil versioning set on gs://BUCKET` for data protection; combine with lifecycle rules to limit the number of noncurrent versions (`"numNewerVersions": 3`) to control storage growth.
9. Use the Transfer Service (`gcloud transfer jobs create`) for large-scale data migrations from AWS S3, Azure Blob, or on-premises sources with bandwidth throttling, scheduling, and automatic retry.
10. Implement customer-managed encryption keys (CMEK) with `gsutil kms authorize -k projects/P/locations/L/keyRings/R/cryptoKeys/K` and set the default key on the bucket to control encryption key lifecycle and meet compliance requirements.
11. Restrict access with VPC Service Controls perimeters to prevent data exfiltration; combine with `allUsers` removal, `storage.publicAccessPrevention=enforced` org policy, and IAM Conditions for time-based or IP-based access rules.
12. Use `gcloud storage insights` and Storage Insights inventory reports to audit bucket composition, identify cost-saving opportunities, and detect publicly accessible objects; export reports to BigQuery for trend analysis.
