---
triggers: ["Dapr", "dapr", "dapr sidecar", "dapr component", "dapr pub sub", "dapr state store", "dapr binding", "dapr workflow", "dapr actor"]
tools_allowed: ["read_file", "write_file", "bash"]
category: devops
---

# Dapr Distributed Application Runtime

When working with Dapr:

1. Initialize Dapr locally with `dapr init` which installs the sidecar binary, Redis (state/pubsub), and Zipkin; run applications with `dapr run --app-id myapp --app-port 3000 -- node app.js` to inject the sidecar alongside your process.
2. Use service invocation via `http://localhost:3500/v1.0/invoke/<app-id>/method/<endpoint>` for synchronous service-to-service calls; the sidecar handles service discovery, mTLS, retries, and observability automatically without SDK dependencies.
3. Configure state stores by placing component YAML files in `~/.dapr/components/` (local) or the Kubernetes `dapr-system` namespace; specify `metadata` with connection strings and set `keyPrefix` to `appid` for app-scoped state isolation.
4. Implement pub/sub messaging by defining a pub/sub component (Redis Streams, Kafka, RabbitMQ) and publishing with `POST /v1.0/publish/<pubsubname>/<topic>`; subscribe by exposing a `POST /topic` endpoint and registering it via `/dapr/subscribe` or declarative subscription YAML.
5. Use input/output bindings for event-driven integrations with external systems (cron, queues, blob storage, SMTP); output bindings invoke via `POST /v1.0/bindings/<name>` and input bindings push events to your app's configured endpoint.
6. Leverage Dapr Actors for stateful, single-threaded virtual actor patterns; define actor types in component config, implement the actor interface in your app, and use turn-based concurrency — the runtime manages activation, deactivation, and timer/reminder persistence.
7. Store and retrieve secrets via the secrets building block using `GET /v1.0/secrets/<store-name>/<key>`; configure secret store components (Kubernetes Secrets, HashiCorp Vault, Azure Key Vault) and reference secrets in other component definitions with `secretKeyRef`.
8. Implement distributed workflows using the Dapr Workflow building block; define workflow activities as functions, compose them with sequential/parallel/fan-out patterns via the workflow SDK, and monitor execution with `dapr workflow get <instance-id>`.
9. Deploy to Kubernetes by adding `dapr.io/enabled: "true"`, `dapr.io/app-id`, and `dapr.io/app-port` annotations to pod specs; the Dapr sidecar injector automatically provisions the daprd container with configured components from the cluster.
10. Configure resiliency policies in `resiliency.yaml` specifying retry strategies (`constant`, `exponential`), timeouts, and circuit breakers per target; apply policies to specific app IDs, components, or actors to handle transient failures gracefully.
11. Use the distributed lock building block (`/v1.0/lock` and `/v1.0/unlock`) with a supported lock store for cross-service mutual exclusion; set appropriate lease TTLs and implement lock renewal for long-running critical sections.
12. Enable observability by configuring the tracing spec in `dapr-config.yaml` with `samplingRate: "1"` and a Zipkin/OTLP collector endpoint; Dapr automatically propagates trace context headers across service invocations, pub/sub, and bindings for end-to-end distributed tracing.
