---
triggers: ["Cosmos DB", "cosmosdb", "azure cosmos", "cosmos partition", "cosmos change feed", "cosmos consistency", "request units"]
tools_allowed: ["read_file", "write_file", "bash"]
requires_bins: ["az"]
category: cloud-azure
---

# Azure Cosmos DB Programming

When working with Azure Cosmos DB:

1. Choose partition keys with high cardinality and even distribution; avoid hot partitions by never using a low-cardinality field like status or boolean — prefer composite keys like `/tenantId` or synthetic keys combining multiple fields.
2. Budget Request Units (RUs) by profiling queries with `x-ms-request-charge` response header; use `container.read_item()` (point reads ~1 RU) over queries when you have both partition key and id, and set `populate_query_metrics=True` for diagnostics.
3. Use the Change Feed processor SDK pattern (`ChangeFeedProcessorBuilder`) for event-driven architectures; configure `start_from_beginning` vs `start_from_now`, set a dedicated lease container, and handle poison messages with a dead-letter pattern.
4. Select the right consistency level per request — override the account default with `consistency_level` in request options; use Session consistency for single-user flows, Bounded Staleness for multi-region reads, and Strong only when linearizability is required (costs 2x RUs).
5. Configure indexing policy explicitly: exclude unused paths with `"excludedPaths": [{"path": "/*"}]` and include only queried paths; use composite indexes for ORDER BY on multiple fields and spatial indexes for geospatial queries.
6. Set TTL at container level (`default_ttl`) and override per-item with `ttl` property; use `-1` on the container to enable TTL without a default, then set expiration per document for fine-grained lifecycle management.
7. Use the SDK's built-in retry policy and configure `ConnectionPolicy` with `retry_options` for 429 (throttled) responses; implement exponential backoff for bulk operations and use `cosmos_client.create_database_if_not_exists()` for idempotent provisioning.
8. Prefer the Bulk Executor pattern (`container.execute_item_batch()` or the Python SDK's `container.create_item` with `enable_cross_partition_query`) for high-throughput ingestion; batch operations within a single partition key to use transactional batch.
9. Use stored procedures and triggers sparingly — they execute within a single partition key, have a 5-second timeout, and consume RUs from the collection budget; prefer server-side logic only for atomic multi-document transactions.
10. Enable autoscale throughput (`ThroughputProperties.create_autoscale_throughput(max_throughput=4000)`) instead of manual provisioning for variable workloads; set max RU/s to 10x your baseline to handle spikes without 429 errors.
11. Implement multi-region writes with conflict resolution policy (Last Writer Wins using `_ts` or custom merge procedures); use `preferred_locations` in the SDK client to route reads to the nearest region and reduce latency.
12. Secure access with Entra ID RBAC (`CosmosDBDataContributor` role) instead of master keys in production; use `DefaultAzureCredential` with the SDK, rotate keys via `az cosmosdb keys regenerate`, and restrict network access with private endpoints and VNet service endpoints.
