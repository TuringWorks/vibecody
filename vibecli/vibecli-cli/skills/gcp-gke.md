---
triggers: ["GKE", "gcp gke", "google kubernetes", "gke autopilot", "gke workload identity", "gke gateway", "gke cluster"]
tools_allowed: ["read_file", "write_file", "bash"]
requires_bins: ["gcloud", "kubectl"]
category: cloud-gcp
---

# GCP Google Kubernetes Engine

When working with GKE:

1. Prefer Autopilot clusters (`gcloud container clusters create-auto`) for most workloads; they enforce security best practices by default (no SSH, no privileged containers, no host network) and bill per-pod resource requests.
2. Enable Workload Identity on Standard clusters with `--workload-pool=PROJECT.svc.id.goog` and annotate Kubernetes service accounts with `iam.gke.io/gcp-service-account=GSA@PROJECT.iam.gserviceaccount.com` to eliminate node-level service account key exposure.
3. Use the Gateway API instead of legacy Ingress by deploying `GatewayClass` resources; GKE's `gke-l7-global-external-managed` class provides managed HTTPS load balancing with automatic TLS certificate provisioning via Certificate Manager.
4. Configure Config Sync with `gcloud beta container fleet config-management apply` to enable GitOps; structure your repo with `namespaces/` and `cluster/` directories and set `spec.sourceFormat: unstructured` for flexibility.
5. Enable node auto-provisioning (`--enable-autoprovisioning --max-cpu=100 --max-memory=400`) to let GKE create optimal node pools matching pod resource requests, reducing over-provisioning costs.
6. For multi-cluster deployments, register clusters to a fleet with `gcloud container fleet memberships register` and use Multi Cluster Ingress with `kind: MultiClusterIngress` for cross-cluster traffic management.
7. Set resource requests and limits on every container; use Vertical Pod Autoscaler in recommendation mode first (`updateMode: "Off"`) to right-size before switching to `Auto`, and configure Horizontal Pod Autoscaler on custom metrics via `external.metric.name`.
8. Enable GKE Backup (`gcloud beta container backup-restore backup-plans create`) with scheduled backups and test restores regularly; scope backup plans to specific namespaces for faster, targeted recovery.
9. Use Binary Authorization (`--binauthz-evaluation-mode=PROJECT_SINGLETON_POLICY_ENFORCE`) to require signed container images; create attestors linked to Cloud KMS keys and integrate signing into your CI pipeline.
10. Optimize costs by using Spot VMs for fault-tolerant workloads with `--spot` node pool flag, setting Pod Disruption Budgets, and configuring cluster autoscaler with `--autoscaling-profile=optimize-utilization`.
11. Secure clusters with private nodes (`--enable-private-nodes --master-ipv4-cidr=172.16.0.0/28`), authorized networks for API server access, and network policies using Calico (`--enable-network-policy`) to restrict pod-to-pod traffic.
12. Monitor with GKE-integrated Cloud Monitoring; enable managed Prometheus collection with `--enable-managed-prometheus` and deploy `PodMonitoring` custom resources to scrape application metrics without managing a Prometheus server.
