---
triggers: ["Apache Kafka", "Kafka Streams", "Kafka Connect", "event streaming", "message broker Kafka"]
tools_allowed: ["read_file", "write_file", "bash"]
category: data-engineering
---

# Apache Kafka Best Practices

When working with Apache Kafka:

1. Design topic partitioning around access patterns and ordering requirements — choose partition keys that distribute load evenly (e.g., user ID, device ID) while guaranteeing ordering within a partition; avoid high-cardinality keys that create hot partitions and plan partition counts for future throughput (partitions are easy to add but hard to reduce).
2. Configure producer reliability with `acks=all` for durability-critical data, enable idempotence (`enable.idempotence=true`) to prevent duplicate writes on retries, and set `retries` to a high value with `delivery.timeout.ms` as the overall bound — use `max.in.flight.requests.per.connection=5` (safe with idempotence enabled).
3. Manage consumer groups carefully — size the number of consumers to match partition count (consumers beyond partition count sit idle), handle rebalancing gracefully with cooperative sticky assignor (`partition.assignment.strategy=CooperativeStickyAssignor`), and commit offsets after processing to avoid data loss.
4. Implement exactly-once semantics (EOS) using transactional producers (`transactional.id`) combined with `read_committed` isolation on consumers, or leverage Kafka Streams' built-in EOS support (`processing.guarantee=exactly_once_v2`) for stream processing workloads.
5. Deploy Kafka Connect for change data capture (CDC) using Debezium connectors — configure snapshot modes, tombstone handling, and schema history topics; use single message transforms (SMTs) for lightweight field mapping and routing before data lands in sink systems.
6. Use Kafka Streams for stateful stream processing — leverage KTables for materialized views, windowed aggregations for time-based analytics, and the Processor API for custom logic; back state stores with RocksDB and configure changelog topics for fault tolerance.
7. Enforce schema governance with Schema Registry using Avro or Protobuf serialization — set compatibility modes (BACKWARD, FORWARD, FULL) per subject, validate schemas in CI pipelines before deployment, and use schema references for complex nested types.
8. Configure retention policies based on use case — time-based retention (`retention.ms`) for event logs, size-based retention (`retention.bytes`) for capacity-constrained environments, and compacted topics (`cleanup.policy=compact`) for changelog/snapshot patterns where only the latest value per key matters.
9. Monitor Kafka clusters with JMX metrics exported to Prometheus — track under-replicated partitions, consumer lag (via Burrow or built-in metrics), request latency percentiles, disk usage per broker, and ISR shrink/expand rates; alert on consumer lag growth and under-replicated partitions immediately.
10. Secure Kafka deployments with SASL/SCRAM or mTLS for authentication, TLS encryption for data in transit, and ACLs for fine-grained topic-level authorization — use separate credentials per application and rotate regularly; enable audit logging for compliance.
11. Use compacted topics for maintaining the latest state per key — ideal for user profiles, configuration, and materialized views; set `min.cleanable.dirty.ratio` and `segment.ms` to control compaction frequency, and always include tombstone records (null values) for deletions.
12. Deploy MirrorMaker 2 for multi-datacenter replication — configure active-active or active-passive topologies, use topic renaming with `replication.policy.class` to avoid circular replication, monitor replication lag between clusters, and test failover procedures regularly.
