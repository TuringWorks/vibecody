---
triggers: ["Key Vault", "azure key vault", "azure secrets", "DefaultAzureCredential", "azure certificate", "azure encryption", "managed HSM"]
tools_allowed: ["read_file", "write_file", "bash"]
requires_bins: ["az"]
category: cloud-azure
---

# Azure Key Vault + Security Patterns

When working with Azure Key Vault:

1. Use `DefaultAzureCredential` from the Azure Identity SDK as the standard authentication chain: it tries Managed Identity, environment variables, Azure CLI, and Visual Studio credentials in order — `SecretClient(vault_url="https://myvault.vault.azure.net", credential=DefaultAzureCredential())` works seamlessly across local dev and production.
2. Choose RBAC over access policies for Key Vault authorization: assign `Key Vault Secrets User` for reading secrets, `Key Vault Crypto User` for cryptographic operations, and `Key Vault Certificates Officer` for certificate management; RBAC supports conditional access and is auditable through Entra ID — migrate existing vaults with `az keyvault update --enable-rbac-authorization`.
3. Manage secrets with versioning: `secret_client.set_secret("db-password", "value")` creates a new version automatically; retrieve specific versions with `get_secret("db-password", version="abc123")` or latest with `get_secret("db-password")`; list versions with `list_properties_of_secret_versions()` for audit trails.
4. Use Key Vault for cryptographic key operations without exposing key material: `crypto_client.encrypt(EncryptionAlgorithm.rsa_oaep_256, plaintext)` and `decrypt()` perform operations server-side; use `sign()` and `verify()` for digital signatures — keys never leave the vault boundary, meeting compliance requirements.
5. Manage TLS certificates with Key Vault: `certificate_client.begin_create_certificate("my-cert", CertificatePolicy.get_default())` generates self-signed certs, or configure issuers (DigiCert, GlobalSign) for CA-signed certificates; enable auto-renewal with `lifetime_actions` set to renew at 80% of validity period.
6. Implement secret rotation with Event Grid notifications: Key Vault fires `SecretNearExpiry` and `SecretExpired` events; subscribe with an Azure Function that generates a new secret, updates the vault, and reconfigures dependent services — set `--expires` on secrets to trigger the rotation lifecycle.
7. Use Managed HSM for FIPS 140-2 Level 3 compliance: `az keyvault create --hsm-name <name> --sku premium` provides single-tenant HSM instances; perform key operations with the same SDK but guaranteed hardware-backed isolation — required for financial, healthcare, and government workloads.
8. Reference Key Vault secrets in App Service and Azure Functions: set app settings to `@Microsoft.KeyVault(SecretUri=https://myvault.vault.azure.net/secrets/secret-name/)` or with version `@Microsoft.KeyVault(SecretUri=https://myvault.vault.azure.net/secrets/name/version)`; the platform resolves secrets at runtime using the app's managed identity.
9. Configure private endpoints to restrict Key Vault network access: `az keyvault update --default-action Deny` then create a private endpoint in your VNet; use `az keyvault network-rule add --ip-address <cidr>` for specific IP exceptions, and enable `--bypass AzureServices` to allow trusted Azure services.
10. Implement caching for secrets in applications: fetch secrets at startup and cache in memory with a TTL matching your rotation frequency; use `list_properties_of_secrets()` (lightweight, no values) to detect changes, and only call `get_secret()` when versions differ — avoids Key Vault throttling (1000 operations per 10 seconds per vault).
11. Use Key Vault in CI/CD pipelines: in Azure DevOps, link variable groups to Key Vault (`az pipelines variable-group create --authorize true`); in GitHub Actions, use `Azure/get-keyvault-secrets@v1` action with OIDC federated credentials; never export secrets to logs — both platforms mask secret values automatically.
12. Enable diagnostic logging and monitoring: `az monitor diagnostic-settings create --resource <vault-id> --logs '[{"category":"AuditEvent","enabled":true}]' --workspace <log-analytics-id>`; alert on `UnauthorizedAccess` events, monitor `ServiceApiLatency` and `Availability` metrics, and review access patterns in Log Analytics with `AzureDiagnostics | where ResourceProvider == "MICROSOFT.KEYVAULT"`.
