---
triggers: ["Drasi", "drasi", "drasi source", "drasi reaction", "continuous query", "drasi change detection", "real-time event processing"]
tools_allowed: ["read_file", "write_file", "bash"]
category: cloud-azure
---

# Drasi Real-Time Event Processing

When working with Drasi:

1. Install Drasi on your Kubernetes cluster using `drasi init` which deploys the control plane components; verify readiness with `drasi status` and ensure the API server, query host, and source/reaction controllers are running.
2. Define sources by creating YAML manifests with `kind: Source` specifying the source type (`PostgreSQL`, `CosmosDB`, `Dapr`) and connection properties; apply with `drasi apply -f source.yaml` to start change data capture from the backing store.
3. Configure PostgreSQL sources with logical replication enabled (`wal_level = logical`) and provide the connection string, publication name, and tables to monitor; Drasi uses the write-ahead log for low-latency change detection without polling.
4. Set up Cosmos DB sources using the change feed processor with `databaseId`, `containerId`, and connection credentials; partition key-aware processing ensures ordered delivery within each logical partition.
5. Write continuous queries using Cypher-like syntax in `kind: ContinuousQuery` resources; define the `sources` array to join across multiple data sources, and specify the query in `properties.query` to express the change detection logic.
6. Design queries to detect meaningful state transitions rather than raw mutations; use temporal operators and aggregations in continuous queries to filter noise and emit only business-relevant events (e.g., "order total exceeded threshold").
7. Create reactions with `kind: Reaction` manifests that specify how to act on query results; built-in reaction types include `Webhook`, `SignalR`, `Dapr`, and `Debug` — each configured with endpoint URLs and authentication.
8. Use the Debug reaction (`kind: Reaction`, `type: Debug`) during development to log all query result changes to stdout; inspect reaction pod logs with `kubectl logs` to validate that your continuous queries emit expected results.
9. Integrate with Dapr by using Dapr sources to monitor state store changes and Dapr reactions to publish events to pub/sub topics; this enables Drasi to fit into existing Dapr-based microservice architectures seamlessly.
10. Handle reaction failures by configuring retry policies and dead-letter destinations in the reaction manifest; idempotent reaction handlers prevent duplicate processing when Drasi redelivers events after transient failures.
11. Monitor Drasi health by checking source sync status with `drasi list sources` and query status with `drasi list queries`; sources report their replication lag and queries report their result set size for operational visibility.
12. Scale continuous query processing by deploying multiple query host replicas and partitioning queries across them; resource-intensive queries with large result sets benefit from dedicated query hosts with increased memory limits.
