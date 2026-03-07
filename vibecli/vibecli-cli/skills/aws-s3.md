---
triggers: ["S3", "aws s3", "s3 bucket", "presigned URL", "s3 multipart", "s3 lifecycle", "s3 event notification"]
tools_allowed: ["read_file", "write_file", "bash"]
requires_bins: ["aws"]
category: cloud-aws
---

# AWS S3 Programming

When working with AWS S3:

1. Use the v3 SDK modular imports (`@aws-sdk/client-s3`, `@aws-sdk/s3-request-presigner`) or boto3's `s3.meta.client` over the resource interface to keep memory usage low and calls explicit.
2. Generate presigned URLs with short expiry (`getSignedUrl(client, new GetObjectCommand({...}), { expiresIn: 900 })`) and never embed credentials; validate the bucket and key server-side before signing.
3. Use multipart upload for objects over 100 MB: call `CreateMultipartUpload`, upload parts concurrently with `UploadPart` (5-15 MB chunks), and always call `CompleteMultipartUpload` or `AbortMultipartUpload` to avoid orphaned parts accruing storage costs.
4. Set a lifecycle rule with `AbortIncompleteMultipartUpload` (e.g., 7 days) on every bucket to garbage-collect abandoned multipart uploads automatically.
5. Enable S3 Event Notifications (`s3:ObjectCreated:*`) routed to SQS or EventBridge rather than polling with `ListObjectsV2`; use the event's `s3.object.key` (URL-decode it) to process objects reactively.
6. Enforce server-side encryption by default with a bucket policy denying `s3:PutObject` when `s3:x-amz-server-side-encryption` is absent; prefer SSE-KMS with a customer-managed key for audit trail via CloudTrail.
7. Use S3 Access Points to scope IAM policies per application or team instead of complex bucket policies; attach a VPC endpoint to the access point for private-only access.
8. Apply S3 Object Lambda access points to transform data on read (redact PII, decompress, resize images) without storing transformed copies, reducing storage duplication.
9. Implement retry with exponential backoff for `SlowDown` (HTTP 503) errors; the SDK's built-in retry strategy handles this, but set `maxAttempts: 5` and monitor `s3:5xxErrors` in CloudWatch.
10. Use `S3 Select` or `SelectObjectContent` API to query CSV/JSON/Parquet in-place with SQL expressions, reducing data transfer by up to 80% compared to downloading full objects.
11. Tag objects at upload time (`Tagging: "env=prod&team=data"`) and reference tags in lifecycle rules for intelligent tiering transitions (Standard -> IA -> Glacier) to cut storage costs by 40-70%.
12. Block all public access at the account level with `PutPublicAccessBlock`, then grant exceptions per bucket only when explicitly required; use `aws s3api get-bucket-policy-status` to audit public exposure.
