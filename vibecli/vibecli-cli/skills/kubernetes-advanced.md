---
triggers: ["k8s operator", "custom resource", "CRD", "k8s networking", "service mesh", "istio", "k8s RBAC", "kustomize", "k8s admission webhook", "k8s scaling"]
tools_allowed: ["read_file", "write_file", "bash"]
requires_bins: ["kubectl"]
category: devops
---

# Advanced Kubernetes Operations

When working with advanced Kubernetes operations:

1. Define CRDs with structural schemas and validation using OpenAPI v3 — always set `preserveUnknownFields: false` and add `additionalPrinterColumns` for useful `kubectl get` output.

2. Configure NetworkPolicies as default-deny ingress/egress per namespace, then whitelist specific pod-to-pod traffic using label selectors: `kubectl apply -f - <<EOF
apiVersion: networking.k8s.io/v1
kind: NetworkPolicy
metadata:
  name: default-deny-all
spec:
  podSelector: {}
  policyTypes: [Ingress, Egress]
EOF`

3. Set up Istio service mesh with strict mTLS peer authentication at the namespace level and use `DestinationRule` to configure circuit breaking with `connectionPool` and `outlierDetection` settings.

4. Implement RBAC with least-privilege Roles scoped to namespaces — use `ClusterRoles` only for cluster-wide resources. Bind service accounts explicitly and never use the `default` service account for workloads.

5. Use Kustomize overlays for environment-specific configuration: base manifests in `base/`, per-env patches in `overlays/dev/`, `overlays/prod/` — prefer strategic merge patches over JSON patches for readability.

6. Build admission webhooks as `ValidatingWebhookConfiguration` or `MutatingWebhookConfiguration` with `failurePolicy: Fail` for security-critical checks and `Ignore` for non-critical enrichment. Always set `timeoutSeconds` to 5 or less.

7. Configure Horizontal Pod Autoscaler v2 with custom metrics from Prometheus using the `metrics-server` and `prometheus-adapter`: `metrics: [{type: Pods, pods: {metric: {name: http_requests_per_second}, target: {type: AverageValue, averageValue: "100"}}}]`.

8. Use PodDisruptionBudgets on every production Deployment to guarantee availability during node drains: `minAvailable: "50%"` or `maxUnavailable: 1` depending on workload criticality.

9. Apply resource quotas and limit ranges per namespace to prevent noisy-neighbor issues: set default CPU/memory requests and limits via `LimitRange`, enforce namespace totals via `ResourceQuota`.

10. Use `topologySpreadConstraints` instead of pod anti-affinity for even distribution across zones and nodes: `maxSkew: 1, topologyKey: topology.kubernetes.io/zone, whenUnsatisfiable: DoNotSchedule`.

11. Implement Vertical Pod Autoscaler in recommendation mode first (`updateMode: "Off"`) to observe suggested resource values before enabling automatic right-sizing with `updateMode: "Auto"`.

12. Debug networking issues systematically: check `kubectl get endpoints`, verify Service selector matches pod labels, inspect kube-proxy iptables rules with `iptables-save | grep <service-cluster-ip>`, and use ephemeral debug containers with `kubectl debug -it <pod> --image=nicolaka/netshoot`.
