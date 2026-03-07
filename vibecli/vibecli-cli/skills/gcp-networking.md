---
triggers: ["GCP networking", "gcp vpc", "cloud load balancer", "cloud armor", "cloud nat", "cloud cdn", "gcp firewall rules", "private google access"]
tools_allowed: ["read_file", "write_file", "bash"]
requires_bins: ["gcloud"]
category: cloud-gcp
---

# GCP Networking

When working with GCP networking:

1. Use custom-mode VPCs (`gcloud compute networks create VPC --subnet-mode=custom`) with explicitly defined subnets and non-overlapping CIDR ranges; avoid default networks and plan IP allocation upfront to support future VPC peering and Shared VPC architectures.
2. Enable Private Google Access on subnets with `gcloud compute networks subnets update SUBNET --enable-private-ip-google-access` so VMs without external IPs can reach Google APIs (GCS, BigQuery, Pub/Sub) via internal routes without a NAT gateway.
3. Configure Cloud NAT with `gcloud compute routers nats create NAT --router=ROUTER --auto-allocate-nat-external-ips --nat-all-subnet-ip-ranges` to provide outbound internet access for private VMs; set `--min-ports-per-vm=64` and enable endpoint-independent mapping for connection-heavy workloads.
4. Deploy Cloud Load Balancing with backend services: use external Application Load Balancer (`gcloud compute url-maps create`) for HTTP/S, proxy Network Load Balancer for TCP/UDP, and internal passthrough for private services; always enable connection draining with `--connection-draining-timeout=300`.
5. Configure Cloud CDN on your load balancer backend with `gcloud compute backend-services update BE --enable-cdn --cache-mode=CACHE_ALL_STATIC` and set custom cache keys and signed URLs for authenticated content delivery at the edge.
6. Deploy Cloud Armor WAF policies with `gcloud compute security-policies create POLICY` and add preconfigured rules: `--expression='evaluatePreconfiguredExpr("sqli-v33-stable")'` for SQLi protection, rate limiting with `--rate-limit-threshold-count=100`, and geo-blocking for compliance.
7. Use hierarchical firewall policies at the organization or folder level (`gcloud compute firewall-policies create`) for baseline rules, and VPC firewall rules for workload-specific policies; tag resources with network tags and use service accounts as source/target for identity-based rules.
8. Set up Shared VPC with `gcloud compute shared-vpc enable HOST_PROJECT` and attach service projects to centralize network management; grant `roles/compute.networkUser` at the subnet level to service project service accounts for least-privilege.
9. Configure Cloud Interconnect (Dedicated or Partner) with `gcloud compute interconnects create` for hybrid connectivity with 99.99% SLA; use VLAN attachments on Cloud Routers with BGP for dynamic routing and set MED values for traffic engineering.
10. Use Cloud DNS with private zones (`gcloud dns managed-zones create ZONE --visibility=private --networks=VPC`) for internal service discovery; enable DNS response policies for custom resolution rules and forward queries to on-premises DNS with DNS forwarding zones.
11. Implement VPC Flow Logs on subnets with `--enable-flow-logs --logging-aggregation-interval=INTERVAL_5_SEC --logging-filter-expr='src_ip!="10.0.0.1"'` to capture network telemetry; export to BigQuery for security analysis and cost allocation across teams.
12. Use Private Service Connect (`gcloud compute forwarding-rules create --target-service-attachment=SA`) to access Google APIs and third-party services via private endpoints inside your VPC; this provides a cleaner alternative to VPC peering with no transitive routing or IP range conflicts.
