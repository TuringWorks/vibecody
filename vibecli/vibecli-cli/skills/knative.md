---
triggers: ["Knative", "knative", "knative serving", "knative eventing", "knative function", "scale to zero", "knative broker", "knative trigger"]
tools_allowed: ["read_file", "write_file", "bash"]
requires_bins: ["kubectl"]
category: devops
---

# Knative Serverless on Kubernetes

When working with Knative:

1. Install Knative Serving with `kubectl apply -f` the serving CRDs and core components, then install a networking layer (Kourier for simplicity, Istio for full mesh); verify with `kubectl get pods -n knative-serving` and configure DNS with `default-domain` or a real domain via `config-domain` ConfigMap.
2. Deploy services with a `kind: Service` resource specifying `spec.template.spec.containers[].image`; Knative automatically creates Routes, Configurations, and Revisions — each deployment produces an immutable Revision that can receive traffic.
3. Configure auto-scaling to zero by setting `autoscaling.knative.dev/min-scale: "0"` annotation on the Service; control scale-up responsiveness with `autoscaling.knative.dev/target` (concurrent requests per pod), `window`, and `panic-window` annotations.
4. Implement traffic splitting for canary deployments using `spec.traffic` blocks: `- revisionName: myapp-v1, percent: 90` and `- revisionName: myapp-v2, percent: 10`; use `tag` fields to create named preview URLs for each revision.
5. Tune cold start latency by setting `autoscaling.knative.dev/min-scale: "1"` for latency-sensitive services, using lightweight base images, and configuring `containerConcurrency` to match your app's thread model and avoid queuing delays.
6. Install Knative Eventing for event-driven architectures; deploy the eventing core and a channel implementation (InMemoryChannel for dev, KafkaChannel for production) to enable event routing between sources and sinks.
7. Create Brokers with `kind: Broker` as the central event mesh for a namespace; use Triggers with `spec.filter.attributes` to route CloudEvents to specific subscriber services based on event `type`, `source`, or custom extension attributes.
8. Configure event sources (`ApiServerSource`, `PingSource`, `KafkaSource`, `GitHubSource`) to ingest external events as CloudEvents into Brokers or directly to sinks; custom sources implement the `SinkBinding` pattern for any event-producing application.
9. Use Knative Functions CLI (`func create -l python -t http`) to scaffold function projects with built-in buildpacks; deploy with `func deploy` which builds the container, pushes to registry, and creates the Knative Service in one command.
10. Configure custom domains by editing the `config-domain` ConfigMap in `knative-serving` namespace; map domain suffixes to label selectors so different namespaces or services get distinct domain names with automatic TLS via `net-certmanager` or `net-http01`.
11. Set up dead-letter sinks on Triggers and Channels with `spec.delivery.deadLetterSink` to capture failed event deliveries; configure `retry` count and `backoffPolicy` (linear/exponential) in the delivery spec for transient failure handling.
12. Monitor Knative with the built-in metrics exported to Prometheus; track `revision_request_count`, `revision_request_latencies`, and `activator_request_count` — use `kubectl get ksvc` to check service readiness and `kubectl get revisions` to audit revision history and traffic allocation.
