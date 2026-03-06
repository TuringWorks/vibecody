---
triggers: ["message queue", "CQRS", "event driven", "idempotency", "Kafka", "RabbitMQ", "pub sub architecture"]
tools_allowed: ["read_file", "write_file", "bash"]
category: architecture
---

# Event-Driven Architecture

When building event-driven systems:

1. Events are immutable facts: "OrderPlaced", "PaymentReceived" — past tense naming
2. Use message brokers: Kafka for high-throughput streams, RabbitMQ for task queues
3. CQRS: separate read models (optimized for queries) from write models (optimized for commands)
4. Idempotent consumers: use event ID + deduplication table to handle redeliveries
5. Dead letter queues (DLQ): route failed messages for investigation — don't lose events
6. Event schema evolution: add fields (safe), never remove/rename — use schema registry
7. Eventual consistency: accept that read models may lag — design UIs accordingly
8. Use outbox pattern: write event to DB table + business data in same transaction, relay async
9. Consumer groups: distribute partitions across instances for parallel processing
10. Ordering guarantees: use partition keys (e.g., user ID) for per-entity ordering
11. Backpressure: consumers must signal when overwhelmed — prefetch limits, batch processing
12. Event replay: design consumers to handle re-processing from any point in the stream
