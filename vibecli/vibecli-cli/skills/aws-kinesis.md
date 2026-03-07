---
triggers: ["Kinesis", "aws kinesis", "kinesis stream", "kinesis firehose", "KCL", "kinesis data analytics", "aws streaming"]
tools_allowed: ["read_file", "write_file", "bash"]
requires_bins: ["aws"]
category: cloud-aws
---

# AWS Kinesis Streaming

When working with AWS Kinesis:

1. Choose partition keys that distribute data evenly across shards; avoid hot shards by using high-cardinality keys (e.g., `userId` or UUID) rather than low-cardinality keys like `region`; monitor `WriteProvisionedThroughputExceeded` and `ReadProvisionedThroughputExceeded` metrics per shard.
2. Use the Kinesis Producer Library (KPL) for high-throughput producers: it aggregates multiple user records into single Kinesis records (up to 1 MB) and buffers with configurable `RecordMaxBufferedTime` (default 100ms) to maximize PutRecords efficiency.
3. Use the Kinesis Client Library (KCL) v2 for consumers: it handles shard discovery, lease management via DynamoDB, and checkpointing; implement `processRecords` idempotently since records may be redelivered after failures.
4. Enable enhanced fan-out (`RegisterStreamConsumer`) for consumers that need dedicated 2 MB/s read throughput per shard instead of sharing the default 2 MB/s across all consumers via `GetRecords`; each consumer gets a push-based subscription via `SubscribeToShard`.
5. Use Kinesis Data Firehose for zero-code delivery to S3, Redshift, OpenSearch, or Splunk; configure buffering hints (`SizeInMBs: 64`, `IntervalInSeconds: 60`) to balance latency vs. file size, and enable Parquet/ORC conversion with a Glue table schema.
6. Handle `ProvisionedThroughputExceededException` with exponential backoff; for `PutRecords` batch calls, retry only the failed records from `Records[].ErrorCode` rather than resending the entire batch.
7. Use on-demand capacity mode for unpredictable traffic (auto-scales to 200 MB/s write, doubles every 15 min) or provisioned mode with `UpdateShardCount` for predictable workloads; resharding operations split or merge shards without data loss.
8. Deaggregate KPL-aggregated records on the consumer side using the KCL (automatic) or the `aws-kinesis-agg` library for Lambda consumers; without deaggregation, you process aggregated blobs instead of individual user records.
9. Integrate Kinesis with Lambda using event source mappings: set `BatchSize` (up to 10,000), `MaximumBatchingWindowInSeconds`, `ParallelizationFactor` (up to 10 concurrent Lambda invocations per shard), and `BisectBatchOnFunctionError` to isolate poison records.
10. Use server-side encryption with KMS (`StreamEncryption: {EncryptionType: "KMS", KeyId: "alias/kinesis-key"}`) to encrypt data at rest; rotate KMS keys automatically and restrict `kinesis:PutRecord` to authorized producers via IAM policies.
11. Set data retention from 24 hours (default) up to 365 days for replay scenarios; use `GetShardIterator` with `AT_TIMESTAMP` to replay from a specific point in time for reprocessing or backfilling.
12. Compare Kinesis Data Streams vs. MSK (Managed Kafka): choose Kinesis for serverless simplicity and AWS-native integration; choose MSK when you need Kafka protocol compatibility, topic compaction, consumer groups, or are migrating existing Kafka workloads.
