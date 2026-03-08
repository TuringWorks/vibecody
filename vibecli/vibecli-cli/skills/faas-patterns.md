---
triggers: ["FaaS", "faas", "function as a service", "serverless patterns", "cold start", "function composition", "serverless architecture", "event driven serverless"]
tools_allowed: ["read_file", "write_file", "bash"]
category: architecture
---

# FaaS Architecture Patterns

When working with FaaS architecture patterns:

1. Mitigate cold starts by keeping function packages small (under 50 MB), minimizing dependency imports, using provisioned concurrency (Lambda) or min-instances (Cloud Functions/Azure), and choosing runtimes with fast startup (Go, Rust, Python) over heavy runtimes (Java/Spring without SnapStart).
2. Implement function composition using step functions or durable orchestrators (AWS Step Functions, Azure Durable Functions, GCP Workflows) rather than direct function-to-function HTTP calls; orchestrators provide retry, state persistence, and visual debugging that raw chaining lacks.
3. Design fan-out/fan-in patterns with a dispatcher function that publishes work items to a queue or invokes child functions in parallel, and an aggregator that collects results; use Step Functions Map state or Durable Functions fan-out for managed orchestration with error handling.
4. Apply the saga pattern for distributed transactions across functions by implementing compensating actions for each step; on failure, execute compensation functions in reverse order — store saga state in a durable store (DynamoDB, Cosmos DB) and use idempotency keys.
5. Ensure idempotency in every function by deriving a deduplication key from the event (message ID, request ID) and checking it against a state store before processing; this prevents duplicate side effects from at-least-once delivery guarantees in queues and event streams.
6. Implement dead-letter queue (DLQ) handling by configuring DLQ destinations on event source mappings and subscribing a monitor function to the DLQ that alerts, logs context, and optionally retries with exponential backoff or routes to manual review.
7. Apply event sourcing with functions by writing immutable events to a log (Kinesis, EventBridge, Kafka) and using functions as event handlers that project read models; separate command functions (write events) from query functions (read projections) following CQRS.
8. Chain functions through events rather than synchronous calls to reduce coupling and latency sensitivity; publish domain events to SNS/EventBridge/Pub/Sub and let downstream functions subscribe independently, enabling parallel processing and independent scaling.
9. Instrument functions with structured logging (JSON), distributed tracing (X-Ray, OpenTelemetry), and custom metrics (CloudWatch EMF, Prometheus push gateway); include correlation IDs in all log entries and propagate trace context through event metadata.
10. Optimize cost by right-sizing memory allocation (Lambda performance scales with memory), setting aggressive timeouts, using reserved concurrency to cap spend, and choosing ARM-based runtimes (Graviton2 on Lambda) for up to 20% cost reduction at equal performance.
11. Structure code with the hexagonal architecture pattern: separate the handler entry point (adapter) from business logic (domain) and external integrations (ports); this enables unit testing business logic without invoking the FaaS runtime and simplifies cross-platform portability.
12. Handle partial failures in batch-processing functions (SQS batch, Kinesis) by reporting individual item failures using partial batch response (`batchItemFailures` on Lambda) rather than failing the entire batch; this prevents successfully processed items from being reprocessed.
