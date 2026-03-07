---
triggers: ["Azure DevOps", "azure pipelines", "azure pipeline yaml", "ado pipeline", "azure devops template", "azure artifacts"]
tools_allowed: ["read_file", "write_file", "bash"]
requires_bins: ["az"]
category: cloud-azure
---

# Azure DevOps Pipelines

When working with Azure DevOps Pipelines:

1. Structure YAML pipelines with clear stage-job-step hierarchy: use `stages` for environment gates (Build, Test, Deploy-Staging, Deploy-Prod), `jobs` for parallel workstreams within a stage, and `steps` for individual tasks; set `dependsOn` and `condition` for orchestration control flow.
2. Use templates for reusable pipeline components: create `templates/build.yml` with `parameters` and reference with `- template: templates/build.yml` passing `parameters: { ... }`; use `extends` templates to enforce organizational standards and security checks that teams cannot bypass.
3. Manage secrets with variable groups linked to Key Vault: `az pipelines variable-group create --authorize --variables` and link via `- group: my-keyvault-group` in YAML; mark sensitive variables with `isSecret: true` and access as `$(secretName)` — secrets are masked in logs automatically.
4. Define environments (`Environments` in project settings) with approval gates: configure pre-deployment approvals, branch control, and business hours checks; reference in deploy jobs with `environment: production` which creates a deployment history and enables rollback tracking.
5. Use caching to speed up builds: `- task: Cache@2` with `key` based on lock files (`**/package-lock.json` or `**/Cargo.lock`) and `path` pointing to the cache directory (`$(Pipeline.Workspace)/.cargo`, `node_modules`); cache hits skip dependency downloads entirely.
6. Configure service connections (`az devops service-endpoint create`) for Azure, Docker, Kubernetes, and GitHub integrations; use workload identity federation over secrets for Azure service connections, and scope connections to specific pipelines to limit blast radius.
7. Publish and consume artifacts: use `- publish: $(Build.ArtifactStagingDirectory)` and `- download: current` between stages; for package management, push to Azure Artifacts feeds (`- task: NuGetCommand@2` or `npm publish`) with versioning using `$(Build.BuildNumber)` or semantic versioning.
8. Implement matrix strategies for multi-platform builds: `strategy: matrix: { linux: { vmImage: ubuntu-latest }, windows: { vmImage: windows-latest } }` with `maxParallel` to control concurrency; combine with `each` expressions for dynamic matrix generation from parameters.
9. Use path triggers and branch filters to avoid unnecessary builds: `trigger: branches: include: [main] paths: include: [src/*, tests/*] exclude: [docs/*]`; configure `pr` triggers separately and use `schedules` for nightly builds with `- cron: "0 2 * * *"`.
10. Add quality gates with built-in tasks: `PublishTestResults@2` for test reporting, `PublishCodeCoverageResults@2` for coverage, and custom conditions like `condition: and(succeeded(), ge(variables['coverage'], 80))` to fail pipelines that do not meet thresholds.
11. Use deployment strategies in deploy jobs: `strategy: runOnce` for simple deploys, `rolling` with `maxParallel` for gradual rollout, or `canary` with `increments: [10, 20]` for progressive exposure; implement `on: failure: steps:` for automatic rollback on deployment failure.
12. Optimize pipeline security: enable `settableVariables` restrictions to prevent injection, use `resources: repositories` to pin external template repos to specific refs, run on self-hosted agents in private VNets for network-isolated builds, and audit pipeline changes via the REST API.
