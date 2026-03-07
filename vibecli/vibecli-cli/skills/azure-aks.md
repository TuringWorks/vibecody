---
triggers: ["AKS", "azure aks", "azure kubernetes", "aks node pool", "aks workload identity", "aks ingress", "azure container"]
tools_allowed: ["read_file", "write_file", "bash"]
requires_bins: ["az", "kubectl"]
category: cloud-azure
---

# Azure Kubernetes Service (AKS)

When working with Azure AKS:

1. Provision clusters with `az aks create` using `--enable-managed-identity` and `--network-plugin azure` (Azure CNI) for VNet-native pod IPs; choose `--network-plugin-mode overlay` for large clusters to avoid IP exhaustion, and set `--node-count`, `--min-count`, `--max-count` with `--enable-cluster-autoscaler`.
2. Use multiple node pools for workload isolation: create a system pool (`--mode System`, tainted with `CriticalAddonsOnly`) for kube-system pods and separate user pools with specific VM sizes, labels, and taints for application workloads — scale pools independently with `az aks nodepool scale`.
3. Enable workload identity (`--enable-oidc-issuer --enable-workload-identity`) instead of pod identity; create a Kubernetes ServiceAccount annotated with `azure.workload.identity/client-id`, bind it to a managed identity via federated credential, and use `DefaultAzureCredential` in your app — no secrets needed.
4. Configure KEDA autoscaling by installing via `az aks update --enable-keda`; define `ScaledObject` resources that reference Azure triggers (Service Bus queue length, Cosmos DB change feed lag, Storage Queue count) to scale deployments to zero and back based on event load.
5. Set up ingress with the managed NGINX ingress controller (`--enable-addons ingress-appgw` for App Gateway or `az aks approuting enable` for managed NGINX); configure TLS with `cert-manager` and Let's Encrypt or reference Key Vault certificates via the CSI driver.
6. Integrate with ACR using `az aks update --attach-acr <acrName>` which grants `AcrPull` role to the kubelet identity; use `az acr build` for cloud-native image builds and configure ACR tasks for automated image rebuilds on base image updates.
7. Implement GitOps with Flux v2 via `az k8s-configuration flux create`; point to a Git repo containing Kustomize overlays or Helm releases, and Flux reconciles cluster state automatically — use `--interval` to control sync frequency and `--prune` for garbage collection.
8. Configure pod disruption budgets (`PodDisruptionBudget`) and resource requests/limits for every deployment; use `LimitRange` and `ResourceQuota` per namespace, and enable the vertical pod autoscaler (`--enable-vpa`) for right-sizing recommendations.
9. Enable Azure Monitor for containers (`--enable-addons monitoring`) and configure Container Insights with `--enable-syslog` for log collection; use Prometheus-based monitoring with `az aks update --enable-azure-monitor-metrics` and query via managed Grafana dashboards.
10. Secure clusters with Azure Policy (`--enable-addons azure-policy`) to enforce pod security standards; use `--enable-defender` for runtime threat detection, disable local accounts with `--disable-local-accounts`, and enforce `--api-server-authorized-ip-ranges` for control plane access.
11. Use maintenance windows (`az aks maintenancewindow add`) to control upgrade timing; enable automatic channel upgrades (`--auto-upgrade-channel stable`) but pin critical workloads with PDBs, and test upgrades in a staging cluster before production rollout.
12. Optimize costs with spot node pools (`--priority Spot --eviction-policy Delete --spot-max-price -1`) for fault-tolerant workloads; use the stop/start feature (`az aks stop`) for dev/test clusters, and right-size VMs using Container Insights CPU/memory utilization data.
