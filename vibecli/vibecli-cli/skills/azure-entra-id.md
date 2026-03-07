---
triggers: ["Entra ID", "Azure AD", "azure entra", "MSAL", "managed identity", "service principal", "azure authentication", "azure RBAC"]
tools_allowed: ["read_file", "write_file", "bash"]
requires_bins: ["az"]
category: cloud-azure
---

# Azure Entra ID (formerly Azure AD)

When working with Azure Entra ID:

1. Register applications via `az ad app create` with explicit `--sign-in-audience` (single-tenant, multi-tenant, or personal accounts); configure redirect URIs per platform (SPA, web, mobile) and never use wildcard redirect URIs — validate the full URI to prevent open redirect attacks.
2. Use MSAL SDK (`@azure/msal-browser`, `msal-python`, `Microsoft.Identity.Client`) for all authentication flows; call `acquireTokenSilent()` first and fall back to `acquireTokenInteractive()` only on `InteractionRequired` errors — MSAL handles token caching, refresh, and retry automatically.
3. Prefer managed identity over service principal secrets in Azure-hosted workloads; use `DefaultAzureCredential()` which chains ManagedIdentityCredential, EnvironmentCredential, AzureCliCredential — in production only system-assigned or user-assigned managed identity is used, zero secrets to manage.
4. Create service principals with `az ad sp create-for-rbac --role <role> --scopes <scope>` for CI/CD and external integrations; use certificate credentials (`--cert`) over client secrets, set short expiry (`--years 1`), and rotate via `az ad sp credential reset`.
5. Implement RBAC with least-privilege: assign roles at the narrowest scope (resource > resource group > subscription); use built-in roles before custom ones, check effective permissions with `az role assignment list`, and use `--condition` for ABAC (attribute-based access control) on storage.
6. Configure conditional access policies for zero-trust enforcement: require MFA for admin roles, block legacy authentication, enforce compliant devices for sensitive apps, and use sign-in risk policies with Identity Protection — test with report-only mode before enforcing.
7. Build Azure AD B2C user flows for consumer-facing apps: create custom policies (Identity Experience Framework) for complex journeys, configure identity providers (Google, Facebook, Apple), and use API connectors to call external APIs during sign-up for validation or enrichment.
8. Validate tokens in APIs by verifying the `iss` (issuer), `aud` (audience matches your app ID), `exp` (not expired), and `nbf` (not before) claims; use the JWKS endpoint (`https://login.microsoftonline.com/{tenant}/discovery/v2.0/keys`) for signature validation — never skip signature verification.
9. Use app roles and security groups for authorization: define `appRoles` in the app manifest, assign users/groups via `az ad app permission grant`, and check `roles` claim in the token; for fine-grained permissions use delegated scopes with `oauth2PermissionScopes`.
10. Implement the on-behalf-of (OBO) flow for middle-tier APIs that call downstream APIs: exchange the incoming token with `ConfidentialClientApplication.acquire_token_on_behalf_of(user_assertion, scopes)` — this maintains the user context across service boundaries.
11. Configure token lifetime and session management: use Continuous Access Evaluation (CAE) for near-real-time token revocation, set token lifetimes via Conditional Access (not legacy token lifetime policies), and use `login_hint` and `domain_hint` to streamline the sign-in experience.
12. Audit and monitor with Entra ID sign-in logs and audit logs routed to Log Analytics; set up alerts for risky sign-ins, application consent grants, and role assignment changes; use `az ad app credential list` to find expiring secrets and automate rotation with Key Vault and Event Grid.
