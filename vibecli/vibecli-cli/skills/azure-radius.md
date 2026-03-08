---
triggers: ["Radius", "radius project", "radius application", "rad cli", "radius recipe", "radius environment", "application graph"]
tools_allowed: ["read_file", "write_file", "bash"]
category: cloud-azure
---

# Radius Application Platform

When working with Radius:

1. Initialize a Radius workspace with `rad init` which scaffolds the environment, resource group, and connects to your Kubernetes cluster; verify setup with `rad env list` and `rad group list` before deploying applications.
2. Define applications using Bicep files with `resource app 'Applications.Core/applications@2023-10-01-preview'` as the root; nest containers, gateways, and connections as child resources referencing the application scope.
3. Use portable resources (`Applications.Datastores/redisCaches`, `Applications.Datastores/sqlDatabases`, `Applications.Messaging/rabbitMQQueues`) to abstract infrastructure; these resolve to cloud-managed or containerized instances depending on the environment's recipes.
4. Configure environments per stage (dev, staging, prod) with `rad env create` and attach recipes that map portable resources to specific infrastructure; dev environments can use containerized local instances while prod uses managed cloud services.
5. Author Bicep recipes for portable resources using `resource recipe 'Applications.Datastores/redisCaches@2023-10-01-preview'` that provision the underlying infrastructure; register recipes with `rad recipe register` against specific environments.
6. Create Terraform recipes as an alternative to Bicep by pointing `rad recipe register --template-kind terraform --template-path <module>` to a Terraform module; Radius manages the Terraform state and lifecycle automatically.
7. Model connections between containers and resources explicitly using `connections: { db: { source: sqlDb.id } }` in Bicep; Radius injects connection strings and credentials as environment variables following a standard naming convention.
8. Leverage the application graph with `rad app graph` to visualize all resources, their connections, and dependencies; use this to validate that portable resources are correctly wired before deployment.
9. Deploy applications with `rad deploy app.bicep` which processes the Bicep template, provisions infrastructure via recipes, and deploys containers to Kubernetes; use `--parameters` for environment-specific overrides.
10. Organize resources into Radius resource groups (`rad group create`) for logical isolation and access control; map resource groups to teams or service boundaries for multi-team environments.
11. Configure container resources with `properties: { container: { image, ports, env, volumes } }` and set resource limits, liveness probes, and readiness probes; Radius translates these into Kubernetes pod specs.
12. Enable multi-cloud deployment by creating environments targeting different cloud providers and registering provider-specific recipes; the same application Bicep deploys to AWS, Azure, or GCP by swapping the environment parameter.
