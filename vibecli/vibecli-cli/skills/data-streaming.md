---
triggers: ["stream processing", "Apache Kafka", "Apache Flink", "event streaming", "Kafka Streams", "Apache Pulsar", "real-time analytics", "event sourcing", "CQRS", "CDC"]
tools_allowed: ["read_file", "write_file", "bash"]
category: data
---

# Data Streaming and Event Processing

When working with stream processing, event-driven architectures, and real-time data pipelines:

1. Design Kafka topics with appropriate partitioning: choose partition keys that distribute load evenly (user ID, device ID) while preserving ordering guarantees within a partition; start with a partition count that accommodates 2-3x current peak throughput, since partitions can be increased but never decreased.

2. Achieve exactly-once semantics in Kafka by enabling idempotent producers (`enable.idempotence=true`), using transactions for atomic multi-partition writes (`transactional.id`), and configuring consumers with `isolation.level=read_committed` to filter out uncommitted messages.

3. Implement stream processing with Apache Flink or Kafka Streams based on requirements: use Flink for complex event processing, large-state operations, and SQL-based streaming analytics; use Kafka Streams for lightweight, library-embedded processing that avoids managing a separate cluster.

4. Apply event sourcing by storing every state change as an immutable event in an append-only log; derive current state by replaying events, maintain materialized views (projections) for query performance, and implement snapshots at regular intervals to bound replay time during recovery.

5. Implement CQRS (Command Query Responsibility Segregation) by separating write models (optimized for validation and event generation) from read models (optimized for query patterns); update read models asynchronously from the event stream, and accept eventual consistency with clearly communicated staleness bounds.

6. Set up Change Data Capture with Debezium connectors for your database (PostgreSQL, MySQL, MongoDB); capture row-level changes as events with before/after states, configure the outbox pattern for reliable event publishing, and handle schema changes by evolving Avro/Protobuf schemas in the schema registry.

7. Enforce schema governance with a schema registry (Confluent Schema Registry, Apicurio) using Avro, Protobuf, or JSON Schema; configure compatibility modes (backward, forward, or full) per topic, validate schemas at produce time, and automate schema evolution checks in CI pipelines.

8. Choose windowing strategies based on business requirements: tumbling windows for fixed-interval aggregations (1-minute counts), sliding windows for moving averages, session windows for user activity grouping with configurable inactivity gaps, and use watermarks to handle late-arriving events with allowed lateness thresholds.

9. Manage stateful processing carefully: use RocksDB-backed state stores for Kafka Streams and Flink checkpoints backed by S3/HDFS; configure incremental checkpointing in Flink, size state store memory and disk appropriately, and implement state migration strategies for topology changes.

10. Handle backpressure by configuring bounded buffers and applying flow control: in Flink, enable credit-based flow control; in Kafka Streams, tune `max.poll.records` and `max.poll.interval.ms`; monitor consumer lag as the primary health metric and set alerts when lag exceeds SLO thresholds.

11. Route failed messages to dead letter queues (DLQs) with full context: include the original message, error details, timestamp, retry count, and source topic; implement automated retry from DLQ with exponential backoff for transient errors, and provide a UI or CLI tool for manual inspection and replay of poison pills.

12. Build real-time analytics pipelines by streaming aggregated data from Kafka/Flink into a serving layer optimized for low-latency queries (Apache Druid, ClickHouse, or Pinot); pre-compute common aggregations in the stream processor, and use the serving layer for ad-hoc slicing and dicing.

13. Implement consumer group management best practices: use meaningful group IDs that indicate the consuming application, configure appropriate rebalancing strategies (cooperative sticky assignment to minimize disruption), handle partition revocation gracefully by flushing state, and monitor rebalance frequency as an operational health signal.

14. Design for multi-region streaming with MirrorMaker 2, Confluent Cluster Linking, or Pulsar geo-replication; handle topic naming conventions across clusters, implement conflict resolution for active-active writes, measure cross-region replication lag, and test failover procedures regularly.

15. Test streaming applications thoroughly: use embedded Kafka (Testcontainers) for integration tests, Flink's MiniCluster for pipeline testing, mock time sources for deterministic window tests, and inject synthetic late/out-of-order events to validate watermark and allowed-lateness behavior.
