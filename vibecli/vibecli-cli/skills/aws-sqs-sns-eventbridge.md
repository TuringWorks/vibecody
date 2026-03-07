---
triggers: ["SQS", "SNS", "EventBridge", "aws messaging", "dead letter queue", "sns topic", "event bus", "sqs fifo"]
tools_allowed: ["read_file", "write_file", "bash"]
requires_bins: ["aws"]
category: cloud-aws
---

# AWS Messaging Services (SQS, SNS, EventBridge)

When working with AWS messaging services:

1. Use SQS FIFO queues (`.fifo` suffix) when ordering and exactly-once processing matter; set `MessageGroupId` to partition ordered streams and `MessageDeduplicationId` (or enable content-based dedup) to prevent duplicates within the 5-minute window.
2. Configure a dead-letter queue (DLQ) on every SQS queue with `maxReceiveCount: 3-5`; monitor `ApproximateNumberOfMessagesVisible` on the DLQ via CloudWatch alarms and set up a redrive policy to replay failed messages.
3. Use SNS message filtering policies (`FilterPolicy` on the subscription) to route events to specific SQS queues or Lambda functions by attribute, eliminating consumer-side filtering logic and reducing costs.
4. Implement the SNS fan-out pattern by subscribing multiple SQS queues to one SNS topic; enable `RawMessageDelivery` on SQS subscriptions to avoid double-JSON encoding and simplify consumer parsing.
5. Use EventBridge rules with detailed event patterns (`{"source": ["myapp"], "detail-type": ["OrderPlaced"], "detail": {"amount": [{"numeric": [">", 100]}]}}`) for content-based routing without custom code.
6. Register event schemas in the EventBridge Schema Registry and use the schema discovery feature to auto-generate bindings; validate events against schemas in producers to catch contract violations early.
7. Set SQS visibility timeout to 6x your Lambda function timeout (or processing time) to prevent duplicate processing; call `ChangeMessageVisibility` to extend it for long-running tasks.
8. Use EventBridge Pipes to connect sources (SQS, DynamoDB Streams, Kinesis) to targets with optional filtering, enrichment, and transformation in a single resource instead of glue Lambda functions.
9. Batch SQS sends with `SendMessageBatch` (up to 10 messages) and receives with `MaxNumberOfMessages: 10` and `WaitTimeSeconds: 20` (long polling) to reduce API calls and cost by up to 90%.
10. Enable server-side encryption on SQS queues (`KmsMasterKeyId`) and SNS topics; use the `aws:kms` condition key in queue policies to enforce encrypted-only publishing.
11. Set EventBridge retry policy (`MaximumRetryAttempts`, `MaximumEventAgeInSeconds`) and a DLQ on each rule target; use the `$.detail.metadata.idempotencyKey` in consumers for safe retries.
12. Use SQS temporary queues (via the Temporary Queue Client) for request-reply patterns instead of creating permanent queues per requester, leveraging virtual queues that multiplex over a single physical queue.
