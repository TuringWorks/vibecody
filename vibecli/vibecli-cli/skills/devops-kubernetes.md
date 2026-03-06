---
triggers: ["Kubernetes", "k8s", "pod", "deployment", "service k8s", "helm", "HPA", "kubectl"]
tools_allowed: ["read_file", "write_file", "bash"]
requires_bins: ["kubectl"]
category: devops
---

# Kubernetes

When deploying to Kubernetes:

1. Use Deployments for stateless apps, StatefulSets for databases, DaemonSets for node-level agents
2. Always set resource requests AND limits: `requests` for scheduling, `limits` for protection
3. Use liveness probes (restart on failure) and readiness probes (route traffic when ready)
4. Use ConfigMaps for config, Secrets for credentials — mount as volumes or env vars
5. Use Namespaces to isolate environments and teams — apply ResourceQuotas per namespace
6. HPA (Horizontal Pod Autoscaler): scale on CPU/memory or custom metrics
7. Use Services for internal discovery; Ingress for external HTTP routing with TLS
8. Rolling updates: set `maxSurge` and `maxUnavailable` for zero-downtime deployments
9. Use Helm charts for templating and packaging — `values.yaml` for per-environment overrides
10. Use PodDisruptionBudgets to ensure availability during node maintenance
11. Use `kubectl describe pod` and `kubectl logs` for debugging — check events section
12. Network Policies: deny all by default, explicitly allow required traffic flows
