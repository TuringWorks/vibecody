---
triggers: ["Blob Storage", "azure blob", "azure storage", "blob container", "SAS token", "blob tier", "azure storage account"]
tools_allowed: ["read_file", "write_file", "bash"]
requires_bins: ["az"]
category: cloud-azure
---

# Azure Blob Storage Programming

When working with Azure Blob Storage:

1. Use `BlobServiceClient` with `DefaultAzureCredential` for authentication in production; fall back to connection strings only in local development; create clients at startup and reuse them — the SDK manages connection pooling and HTTP pipeline internally.
2. Choose the right blob type: Block blobs for files up to 190.7 TiB (most common), Append blobs for log-style append-only writes, Page blobs for random read/write (VHD disks); use `upload_blob(data, blob_type="BlockBlob", overwrite=True)` and set `max_concurrency` for parallel chunk uploads.
3. Configure access tiers strategically: Hot for frequent access, Cool for 30+ day retention, Cold for 90+ day, Archive for 180+ day; set tier at upload with `standard_blob_tier="Cool"` or change later with `set_standard_blob_tier()` — Archive rehydration takes hours, use priority rehydration for urgent access.
4. Implement lifecycle management policies with `az storage account management-policy create`; define rules to auto-tier blobs (Hot to Cool after 30 days, Cool to Archive after 90 days) and auto-delete after retention period — filter by prefix and blob index tags for granular control.
5. Generate SAS tokens with minimum required permissions and short expiry: use `generate_blob_sas(account_name, container, blob, permission=BlobSasPermissions(read=True), expiry=datetime.utcnow()+timedelta(hours=1), account_key=key)`; prefer user delegation SAS (`generate_blob_sas` with `user_delegation_key`) backed by Entra ID.
6. Enable blob versioning and soft delete for data protection: `az storage account blob-service-properties update --enable-versioning --enable-delete-retention --delete-retention-days 7`; access previous versions with `blob_client.get_blob_properties(version_id=version)` and restore with `start_copy_from_url`.
7. Use immutability policies for compliance: set time-based retention (`az storage container immutability-policy create --period 365`) or legal hold on containers; once locked, policies cannot be shortened — test with unlocked policies first and lock only when compliant.
8. Tag blobs with index tags (`blob_client.set_blob_tags({"project": "alpha", "status": "processed"})`) for efficient cross-container querying; use `find_blobs_by_tags("project='alpha' AND status='processed'")` on `ContainerClient` for tag-based discovery without listing all blobs.
9. Handle large file uploads with `upload_blob(data, max_concurrency=4, max_single_put_size=8*1024*1024)` which auto-chunks into blocks; for resumable uploads use `stage_block()` + `commit_block_list()` pattern to manage individual blocks and retry only failed chunks.
10. Configure event triggers with Event Grid subscriptions on blob events (`Microsoft.Storage.BlobCreated`, `BlobDeleted`); filter by subject prefix/suffix (`/blobServices/default/containers/uploads`, `.csv`) and route to Azure Functions, Logic Apps, or Event Hubs for processing pipelines.
11. Enable CORS rules for browser-based access: `az storage cors add --services b --methods GET PUT --origins "https://myapp.com" --allowed-headers "*" --max-age 3600`; use `@azure/storage-blob` JavaScript SDK with `BlobServiceClient.fromConnectionString()` or SAS URL for direct browser uploads.
12. Secure storage accounts with private endpoints and disable public access (`az storage account update --default-action Deny`); use VNet service endpoints for Azure-to-Azure traffic, enable infrastructure encryption (double encryption), and configure Azure Defender for Storage to detect anomalous access patterns.
