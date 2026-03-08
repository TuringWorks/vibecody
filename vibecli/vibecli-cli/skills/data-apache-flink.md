---
triggers: ["Apache Flink", "Flink SQL", "stream processing Flink", "CEP Flink", "stateful streaming"]
tools_allowed: ["read_file", "write_file", "bash"]
category: data-engineering
---

# Apache Flink Best Practices

When working with Apache Flink:

1. Choose between DataStream API and Table/SQL API based on complexity — use the Table API and Flink SQL for declarative relational queries, aggregations, and joins where the optimizer can plan execution; use the DataStream API for custom stateful logic, side outputs, and fine-grained control over event processing.
2. Distinguish event time from processing time and default to event time for reproducible results — assign timestamps with `WatermarkStrategy.forBoundedOutOfOrderness()` for most use cases, and use processing time only when latency matters more than correctness (e.g., monitoring dashboards).
3. Configure watermarks to handle late data appropriately — set bounded-out-of-orderness duration based on observed data latency, use allowed lateness on windows for additional tolerance, and direct late events to side outputs for reprocessing rather than silently dropping them.
4. Enable checkpointing with appropriate intervals (typically 1-5 minutes) and use incremental checkpoints with RocksDB state backend for large state — configure `execution.checkpointing.mode=EXACTLY_ONCE`, set `min-pause-between-checkpoints` to avoid checkpoint storms, and store checkpoints on a distributed filesystem (S3, HDFS) for recovery.
5. Use savepoints for planned maintenance, application upgrades, and schema evolution — always take a savepoint before stopping a job, assign unique UIDs to all operators to ensure state compatibility across code changes, and validate restored state after redeployment.
6. Select the state backend based on state size — use HashMapStateBackend (heap) for small state with fast access, and RocksDBStateBackend for large state that exceeds available heap memory; tune RocksDB with block cache size, write buffer count, and compaction settings for heavy-state workloads.
7. Leverage Flink CEP (Complex Event Processing) for pattern detection over event streams — define patterns with `begin()`, `followedBy()`, `within()` for time-bounded sequences; use iterative conditions for flexible matching and handle timed-out partial patterns to avoid silent data loss.
8. Write streaming SQL queries with Flink SQL for windowed aggregations, temporal joins, and CDC ingestion — use `TUMBLE`, `HOP`, and `CUMULATE` window functions, connect to Kafka and database sources with built-in connectors, and leverage Flink SQL hints for join strategy control.
9. Implement exactly-once delivery to external systems using the two-phase commit protocol via `TwoPhaseCommitSinkFunction` — pre-commit on checkpoint barriers, commit on checkpoint completion; for Kafka sinks, use the built-in transactional producer; for databases, use idempotent upserts as a simpler alternative.
10. Use side outputs to split streams based on business logic without duplicating processing — tag late events, validation failures, and routing decisions into separate `OutputTag` streams for independent downstream handling, monitoring, or dead-letter processing.
11. Apply async I/O (`AsyncDataStream.unorderedWait()` or `orderedWait()`) for enrichment lookups against external databases or APIs — configure timeout and capacity to bound concurrent requests, use caching (Guava, Caffeine) to reduce external call volume, and prefer unordered mode when event ordering is not required for higher throughput.
12. Handle backpressure by monitoring Flink's built-in backpressure metrics in the web UI — identify bottleneck operators, scale parallelism of slow operators independently, optimize serialization (use Flink's native serializers over Kryo), and consider network buffer tuning (`taskmanager.network.memory.fraction`) for high-throughput pipelines; deploy on Kubernetes with native mode or YARN for elastic scaling.
