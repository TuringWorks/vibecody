---
triggers: ["system design", "load balancing", "sharding", "CAP theorem", "scalability", "high availability", "distributed system"]
tools_allowed: ["read_file", "write_file", "bash"]
category: architecture
---

# System Design

When designing scalable systems:

1. Start with requirements: clarify functional + non-functional (latency, throughput, availability)
2. Back-of-envelope estimation: users, QPS, storage, bandwidth — know your numbers
3. Load balancing: L4 (TCP) for raw performance, L7 (HTTP) for routing rules — use round-robin or least-connections
4. CAP theorem: choose CP (consistency) for financial systems, AP (availability) for social feeds
5. Horizontal scaling: stateless services behind load balancer — scale by adding instances
6. Database sharding: partition by user ID or region — handle cross-shard queries carefully
7. Caching: CDN → application cache (Redis) → database query cache — multiple levels
8. Read replicas: scale reads independently — use eventual consistency where acceptable
9. Message queues: decouple producers from consumers — absorb traffic spikes
10. Rate limiting: token bucket or sliding window — protect against abuse and cascading failures
11. Data partitioning: time-based for logs, hash-based for users, range-based for geospatial
12. Failover: active-passive for databases, active-active for stateless services — test regularly
