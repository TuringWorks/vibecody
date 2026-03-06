---
triggers: ["microservices", "service boundary", "saga pattern", "event sourcing", "service mesh", "domain driven"]
tools_allowed: ["read_file", "write_file", "bash"]
category: architecture
---

# Microservices Architecture

When designing microservices:

1. Define service boundaries around business domains (DDD bounded contexts) — not technical layers
2. Each service owns its data — no shared databases between services
3. Use async communication (events/messages) between services — avoid synchronous chains
4. Saga pattern for distributed transactions: choreography (events) or orchestration (coordinator)
5. Event sourcing: store events, not state — rebuild state by replaying events
6. API Gateway: single entry point for clients — handles routing, auth, rate limiting
7. Circuit breaker: prevent cascade failures when downstream services are unavailable
8. Service discovery: use DNS-based (Kubernetes Service) or registry-based (Consul)
9. Idempotency: all operations must be safely retryable — use idempotency keys
10. Health checks: each service exposes `/health` and `/ready` endpoints
11. Correlation IDs: propagate request ID across all service calls for distributed tracing
12. Start monolith, extract services when boundaries become clear — avoid premature decomposition
