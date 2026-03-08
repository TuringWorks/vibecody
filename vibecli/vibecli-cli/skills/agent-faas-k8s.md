---
triggers: ["agent kubernetes", "agent faas", "agent k8s", "agent serverless", "agent deployment", "agent scaling", "agent orchestration kubernetes", "keda agent"]
tools_allowed: ["read_file", "write_file", "bash"]
requires_bins: ["kubectl"]
category: ai
---

# AI Agent Deployment on FaaS + Kubernetes

When deploying AI agents on serverless and Kubernetes infrastructure:

1. **Agent-per-Function Pattern** — Deploy each agent capability as a discrete serverless function (AWS Lambda, Cloud Functions, Azure Functions) with a single responsibility. Map tool calls to function invocations, use cold-start-optimized runtimes (Go, Rust, or Python with slim containers), and keep function packages under 50MB for sub-second cold starts.
2. **Event-Driven Agent Activation** — Wire agents to event sources (SQS, Pub/Sub, EventBridge, Kafka) instead of synchronous HTTP. Use event filters to route only relevant events to each agent, implement fan-out for parallel agent execution, and leverage dead-letter queues for events that fail after retries.
3. **K8s Job-Based Agents** — Run long-running agent tasks as Kubernetes Jobs or CronJobs rather than keeping pods alive. Set `activeDeadlineSeconds` and `backoffLimit` to prevent runaway agents, use `ttlSecondsAfterFinished` for automatic cleanup, and mount agent state from PVCs or object storage for resumability.
4. **KEDA-Driven Agent Scaling** — Use KEDA (Kubernetes Event-Driven Autoscaler) to scale agent deployments based on queue depth, HTTP request rate, or custom metrics. Configure `ScaledObject` with appropriate `pollingInterval`, `cooldownPeriod`, and `minReplicaCount` (zero for cost savings, one for latency-sensitive agents).
5. **Agent State in External Stores** — Persist agent conversation history, tool results, and execution state in Redis (for fast access) or DynamoDB/Firestore (for durability). Use TTLs to auto-expire stale sessions, partition state by agent-run-id, and never store state in the pod filesystem or function /tmp beyond a single invocation.
6. **Step Functions / Durable Functions Orchestration** — Model multi-step agent workflows as state machines using AWS Step Functions, Azure Durable Functions, or Temporal. Each state invokes an agent step, handles retries with exponential backoff, supports human approval gates, and provides built-in execution history for debugging.
7. **Cost-Efficient Agent Hosting** — Use spot/preemptible instances for non-latency-critical agent workloads with proper checkpointing. Set resource requests and limits on agent pods (typically 256Mi-1Gi RAM, 0.25-1 CPU for orchestration; more for local inference). Use Karpenter or Cluster Autoscaler to right-size node pools based on actual agent demand.
8. **Agent Gateway with Rate Limiting** — Deploy an API gateway (Kong, Envoy, or cloud-native) in front of agent endpoints with per-user rate limiting, request size limits, and authentication. Enforce token budget caps per request to prevent runaway LLM costs, and use circuit breakers to fail fast when downstream LLM providers are degraded.
9. **Sidecar Pattern for Agent Tooling** — Package agent tool dependencies (browser automation, code execution sandbox, database clients) as sidecar containers in the same pod. This isolates tool execution from agent logic, enables independent scaling and updates, and provides security boundaries via network policies between containers.
10. **Agent Observability Stack** — Instrument agents with OpenTelemetry traces spanning the full request lifecycle: trigger, LLM call, tool execution, and response. Export to Jaeger/Tempo for distributed tracing, emit custom metrics (tokens used, tool call count, latency per step) to Prometheus, and aggregate structured logs in Loki or CloudWatch.
11. **Multi-Cluster Agent Distribution** — Deploy agents across multiple clusters or regions for resilience and data locality. Use Kubernetes federation or a service mesh to route agent requests to the nearest healthy cluster. Store agent state in globally replicated databases (CockroachDB, Spanner) to enable seamless failover.
12. **Namespace Isolation and RBAC** — Isolate agent workloads in dedicated Kubernetes namespaces with ResourceQuotas and LimitRanges. Create service accounts per agent type with minimal RBAC permissions, use NetworkPolicies to restrict agent-to-agent communication, and enforce PodSecurity Standards at the namespace level to prevent privilege escalation.
