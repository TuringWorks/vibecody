---
triggers: ["DynamoDB", "dynamodb", "dynamo table", "dynamodb stream", "single table design", "GSI", "DynamoDB DAX"]
tools_allowed: ["read_file", "write_file", "bash"]
requires_bins: ["aws"]
category: cloud-aws
---

# AWS DynamoDB Data Modeling and SDK Usage

When working with DynamoDB:

1. Design tables with single-table patterns: use generic key names (`PK`, `SK`) with prefixed values (`USER#123`, `ORDER#2024-01-15`) so that multiple entity types share one table and queries use `begins_with(SK, 'ORDER#')` for efficient access patterns.
2. Create GSIs for each additional access pattern; use sparse indexes by only projecting items that have the GSI key attributes, keeping index storage and write costs minimal.
3. Use `TransactWriteItems` for cross-item atomic writes (up to 100 items, 4 MB) with condition expressions to enforce business invariants like unique constraints: `ConditionExpression: "attribute_not_exists(PK)"`.
4. Enable DynamoDB Streams with `NEW_AND_OLD_IMAGES` and attach a Lambda consumer for CDC pipelines; process records idempotently using the `eventID` as a deduplication key.
5. Set TTL on a numeric epoch attribute (e.g., `expiresAt`) to auto-delete expired items at no cost; items are removed within 48 hours of expiry and appear in Streams with `userIdentity.type: "Service"`.
6. Use `BatchWriteItem` (up to 25 items) and `BatchGetItem` (up to 100 items) for bulk operations; always retry `UnprocessedItems`/`UnprocessedKeys` with exponential backoff.
7. Query with `ProjectionExpression` to return only needed attributes, reducing read capacity consumption; use `ExpressionAttributeNames` for reserved words (`#status` -> `status`).
8. Use PartiQL via `ExecuteStatement` for familiar SQL-like syntax in ad-hoc queries, but prefer the document client's `QueryCommand` in hot paths for lower latency and explicit capacity control.
9. Deploy DAX (DynamoDB Accelerator) for read-heavy workloads needing microsecond latency; use the DAX client as a drop-in replacement and set item cache TTL to match your staleness tolerance.
10. Choose on-demand capacity for unpredictable workloads and provisioned with auto-scaling (target 70% utilization) for steady traffic; switch modes at most once per 24 hours.
11. Implement optimistic locking with a `version` attribute and `ConditionExpression: "#v = :expectedVersion"` on updates; increment the version on every write to prevent lost updates in concurrent scenarios.
12. Use `aws dynamodb describe-table` to monitor `ConsumedCapacity` and enable CloudWatch contributor insights to identify hot partition keys; redistribute access by adding a random suffix (write sharding) to high-cardinality keys.
