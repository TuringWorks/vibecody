---
triggers: ["infrastructure scanning", "tfsec", "Checkov", "kube-bench", "kubescape", "Prowler", "ScoutSuite", "CIS benchmark", "cloud security posture", "CSPM"]
tools_allowed: ["read_file", "write_file", "bash"]
category: security
---

# Infrastructure Security Scanning

When working with infrastructure scanning:

1. Scan Terraform code for misconfigurations before apply using tfsec: `tfsec . --format json > tfsec-results.json` detects security issues like unencrypted S3 buckets, open security groups, and missing logging; use `tfsec . --minimum-severity HIGH --soft-fail` in CI to warn without blocking, or `--hard-fail` to enforce as a gate.

2. Run Checkov as a comprehensive IaC scanner supporting Terraform, CloudFormation, Kubernetes, Helm, and Dockerfiles: `checkov -d . --framework terraform --output json > checkov-results.json` and `checkov -f Dockerfile --framework dockerfile`; use `--check CKV_AWS_18,CKV_AWS_19` to run specific checks or `--skip-check CKV_AWS_41` to suppress with documented rationale.

3. Assess Kubernetes cluster security against CIS benchmarks using kube-bench: `kube-bench run --targets master,node,etcd,policies --json > kube-bench.json` checks control plane configuration, node security, etcd encryption, and network policies; run as a Kubernetes Job with `kubectl apply -f https://raw.githubusercontent.com/aquasecurity/kube-bench/main/job.yaml`.

4. Scan Kubernetes manifests and running clusters with Kubescape: `kubescape scan framework nsa,mitre --format json --output kubescape.json` applies NSA/CISA and MITRE ATT&CK frameworks, `kubescape scan control C-0034` checks specific controls, and `kubescape scan . --include-namespaces production` focuses on critical namespaces.

5. Audit AWS accounts with Prowler for comprehensive cloud security posture management: `prowler aws --severity critical high --output-formats json-ocsf --output-directory ./prowler-results` checks 300+ controls across IAM, S3, EC2, RDS, CloudTrail, and more; filter by compliance framework with `prowler aws --compliance cis_2.0_aws`.

6. Scan multi-cloud environments with ScoutSuite: `scout aws --report-dir ./scout-results` for AWS, `scout azure` for Azure, `scout gcp` for GCP; generates HTML reports with severity-ranked findings across identity, storage, compute, networking, and logging services with direct links to remediation documentation.

7. Implement CIS benchmark compliance across cloud providers: use Prowler's `--compliance cis_2.0_aws` or Checkov's `--framework-mapping cis_aws` to map findings to specific CIS controls, generate evidence reports with `prowler aws --compliance cis_2.0_aws --output-formats csv` for auditor-ready documentation.

8. Configure AWS Config rules for continuous compliance monitoring: deploy managed rules like `s3-bucket-public-read-prohibited`, `encrypted-volumes`, `iam-root-access-key-check` via Terraform `aws_config_config_rule` resources; aggregate across accounts with AWS Config Aggregator and trigger auto-remediation via Lambda on non-compliant resources.

9. Enforce Azure Policy for infrastructure guardrails: assign built-in policy definitions like `Audit VMs that do not use managed disks` and `Storage accounts should use private link`; use `az policy assignment create --policy $POLICY_ID --scope /subscriptions/$SUB_ID` and check compliance with `az policy state summarize --filter "complianceState eq 'NonCompliant'"`.

10. Implement compliance-as-code by versioning policy definitions alongside infrastructure: store Checkov custom policies in `.checkov/`, Rego policies for OPA in `policies/`, and tfsec custom rules in `.tfsec/` within the same repository; run policy checks in CI with `checkov -d . --external-checks-dir .checkov/` to ensure policies evolve with the codebase.

11. Detect infrastructure drift between declared state and actual state: `terraform plan -detailed-exitcode` returns exit code 2 when drift exists, `driftctl scan --from tfstate://terraform.tfstate --output json://drift.json` identifies unmanaged resources, and schedule weekly drift checks to catch manual console changes that bypass IaC workflows.

12. Aggregate infrastructure scan results across tools and accounts: normalize outputs to OCSF or SARIF format, import into DefectDojo with `curl -X POST "$DEFECTDOJO_URL/api/v2/import-scan/" -F "scan_type=Checkov Scan" -F "file=@checkov-results.json"`, and build dashboards tracking compliance score trends, top failing controls, and mean-time-to-remediate across the infrastructure portfolio.
