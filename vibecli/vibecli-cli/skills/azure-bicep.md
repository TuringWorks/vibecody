---
triggers: ["Bicep", "azure bicep", "bicep module", "bicep template", "azure infrastructure as code", "arm template", "bicep deploy"]
tools_allowed: ["read_file", "write_file", "bash"]
requires_bins: ["az"]
category: cloud-azure
---

# Azure Bicep Infrastructure as Code

When working with Azure Bicep:

1. Structure projects with modules: split `main.bicep` into reusable modules per resource group or logical component (`modules/storage.bicep`, `modules/network.bicep`); pass parameters explicitly and use `module storageModule 'modules/storage.bicep' = { params: { ... } }` — modules enforce encapsulation and testability.
2. Define parameters with `@allowed`, `@minLength`, `@maxLength`, `@minValue`, `@maxValue` decorators for input validation; use `@secure()` for secrets, `@description()` for documentation, and provide sensible `param location string = resourceGroup().location` defaults to reduce required inputs.
3. Use variables for computed values and string interpolation: `var storageAccountName = 'st${uniqueString(resourceGroup().id)}'`; keep naming conventions consistent with variables, and use `az bicep build` to inspect the generated ARM JSON for debugging.
4. Deploy conditional resources with `if` expressions: `resource lock 'Microsoft.Authorization/locks@2020-05-01' = if (enableLock) { ... }`; combine with ternary operators in property values (`property: isProd ? 'Premium' : 'Standard'`) to create environment-aware templates.
5. Use loops with `for` for creating multiple resources: `resource nsg 'Microsoft.Network/networkSecurityGroups@2023-09-01' = [for subnet in subnets: { ... }]`; access loop index with `for (item, index) in items`, and use `@batchSize(n)` decorator for serial deployment of dependent resources.
6. Leverage deployment stacks (`az stack group create`) to manage resource lifecycle: stacks track all resources deployed by a template and can `--deny-settings-mode denyDelete` to prevent accidental removal; use `--action-on-unmanage deleteAll` to clean up orphaned resources.
7. Run what-if analysis before every deployment: `az deployment group what-if --template-file main.bicep --parameters params.bicepparam` shows create, modify, delete, and no-change operations; integrate what-if into PR pipelines for infrastructure change review.
8. Publish modules to a Bicep registry (ACR): `az bicep publish --file module.bicep --target br:myregistry.azurecr.io/bicep/modules/storage:v1`; reference published modules with `module stg 'br:myregistry.azurecr.io/bicep/modules/storage:v1' = { ... }` for versioned, org-wide reuse.
9. Use `bicepconfig.json` to configure linting rules, module aliases, and credential providers; enable `no-unused-params`, `no-hardcoded-env-url`, `secure-parameter-default` rules, and set `"moduleAliases"` to shorten registry paths in module references.
10. Use `.bicepparam` parameter files (Bicep-native) over JSON parameter files: `using 'main.bicep'` with `param env = 'prod'` syntax; reference Key Vault secrets with `param sqlPassword = az.getSecret('<subscriptionId>', '<rgName>', '<vaultName>', '<secretName>')` for secure parameter injection.
11. Use `existing` keyword to reference pre-existing resources: `resource vnet 'Microsoft.Network/virtualNetworks@2023-09-01' existing = { name: vnetName }` then access `vnet.id` and `vnet.properties`; use `scope: resourceGroup(otherRg)` for cross-resource-group references.
12. Integrate Bicep into CI/CD with `az deployment group create --template-file main.bicep --mode Incremental`; never use `Complete` mode in production without careful review as it deletes unlisted resources; use deployment names with build IDs for traceability and rollback via `az deployment group cancel`.
