---
triggers: ["OCI", "oracle cloud", "autonomous database", "oracle oci", "oci compartment", "oci vcn", "oracle kubernetes", "oci functions"]
tools_allowed: ["read_file", "write_file", "bash"]
requires_bins: ["oci"]
category: cloud-oci
---

# Oracle Cloud Infrastructure (OCI)

When working with Oracle Cloud Infrastructure:

1. Organize resources into compartments for isolation and IAM policy scoping; create compartments with `oci iam compartment create --name dev --description "Dev environment" --compartment-id $TENANCY_OCID` and reference them in all resource commands via `--compartment-id`.
2. Set up VCN networking with public and private subnets using `oci network vcn create --cidr-block 10.0.0.0/16 --display-name main-vcn --compartment-id $CID`, then add subnets, internet gateways, route tables, and security lists for controlled ingress/egress.
3. Provision Autonomous Database with `oci db autonomous-database create --compartment-id $CID --db-name mydb --cpu-core-count 1 --data-storage-size-in-tbs 1 --admin-password $PWD --db-workload OLTP` and use wallet-based mTLS connections by downloading the wallet via `oci db autonomous-database generate-wallet`.
4. Deploy OCI Functions by creating an application context with `fn create app myapp --annotation oracle.com/oci/subnetIds='["$SUBNET_OCID"]'`, then `fn deploy --app myapp` to push container-based functions that scale to zero automatically.
5. Use OKE (Oracle Kubernetes Engine) for container orchestration: `oci ce cluster create --name prod-cluster --compartment-id $CID --vcn-id $VCN_OCID --kubernetes-version v1.28.2` and configure kubectl access with `oci ce cluster create-kubeconfig --cluster-id $CLUSTER_OCID`.
6. Manage Object Storage buckets with `oci os bucket create --name assets --compartment-id $CID` and use pre-authenticated requests for time-limited public access: `oci os preauth-request create --bucket-name assets --access-type ObjectRead --time-expires 2026-04-01T00:00:00Z`.
7. Write IAM policies using the OCI policy language: `Allow group developers to manage instances in compartment dev` and apply with `oci iam policy create --name dev-policy --compartment-id $CID --statements file://policy.json` to enforce least-privilege access.
8. Use the Terraform OCI provider by configuring `provider "oci" { tenancy_ocid, user_ocid, fingerprint, private_key_path, region }` and leverage `oci_core_instance`, `oci_database_autonomous_database`, and `oci_containerengine_cluster` resources for infrastructure as code.
9. Leverage Always Free Tier resources (2 AMD Compute VMs, 4 Arm Ampere A1 cores, 24GB RAM, 2 Autonomous Databases, 20GB Object Storage) for development and testing by selecting Always Free-eligible shapes like `VM.Standard.E2.1.Micro` or `VM.Standard.A1.Flex`.
10. Use the OCI Python SDK for programmatic access: `import oci; config = oci.config.from_file(); compute = oci.core.ComputeClient(config); instances = compute.list_instances(compartment_id).data` and handle pagination with `oci.pagination.list_call_get_all_results()`.
11. Enable cost management by setting budgets with `oci budgets budget create --compartment-id $CID --amount 100 --reset-period MONTHLY --target-type COMPARTMENT` and configure alert rules to notify when spending approaches thresholds.
12. Secure API access by storing OCI CLI config in `~/.oci/config` with API key authentication, rotate keys regularly with `oci iam user api-key upload`, and use instance principals (`oci.auth.signers.InstancePrincipalsSecurityTokenSigner()`) for workloads running on OCI compute to avoid embedding credentials.
