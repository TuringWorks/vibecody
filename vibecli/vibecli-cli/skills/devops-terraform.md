---
triggers: ["Terraform", "IaC", "infrastructure as code", "terraform module", "terraform state", "HCL"]
tools_allowed: ["read_file", "write_file", "bash"]
requires_bins: ["terraform"]
category: devops
---

# Terraform Infrastructure as Code

When managing infrastructure with Terraform:

1. Structure: `main.tf` (resources), `variables.tf` (inputs), `outputs.tf` (outputs), `providers.tf`
2. Use modules for reusable infrastructure: `module "vpc" { source = "./modules/vpc" }`
3. Store state remotely: S3 + DynamoDB lock (AWS), GCS (GCP) — never commit `terraform.tfstate`
4. Use `terraform plan` before `apply` — review changes, especially destroys
5. Use workspaces or separate directories for dev/staging/prod environments
6. Pin provider versions: `required_providers { aws = { version = "~> 5.0" } }`
7. Use `count` and `for_each` for creating multiple similar resources
8. Tag all resources: `tags = { Environment = var.env, Team = "platform", ManagedBy = "terraform" }`
9. Use data sources to reference existing infrastructure: `data "aws_vpc" "main" { ... }`
10. Use `lifecycle { prevent_destroy = true }` for critical resources (databases, S3 buckets)
11. Drift detection: run `terraform plan` in CI to detect manual changes
12. Use `terraform import` to bring existing resources under management — then write the config
