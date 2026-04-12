---
triggers: ["MCP audit", "MCP enterprise", "SSO MCP", "gateway policy", "MCP config portability"]
tools_allowed: ["read_file", "write_file", "bash"]
category: protocols
---

# Enterprise MCP Governance

When deploying MCP (Model Context Protocol) in enterprise environments:

1. **Audit-Log All Tool Invocations** — Every MCP tool call must produce an immutable audit log entry containing: timestamp (ISO 8601 UTC), session ID, user identity (resolved from SSO token), tool name, input parameters (redacted for PII/secrets), output hash, and latency. Store audit logs in an append-only sink (S3, CloudWatch Logs, or SIEM). Audit logging is non-negotiable — never deploy MCP without it enabled.
2. **OIDC vs SAML Selection** — Use OIDC when: the MCP client is a web app or CLI (short-lived access tokens, refresh token flow). Use SAML when: the enterprise has an existing SAML-only IdP (ADFS, legacy Okta) and cannot issue OIDC tokens. Prefer OIDC for all greenfield deployments due to simpler token validation and native JWT support. Never implement both simultaneously for the same MCP gateway — pick one and standardize.
3. **Gateway Policy: Default-Deny Model** — The MCP gateway must start from a default-deny policy: all tool invocations are blocked unless explicitly permitted by a policy rule. Policy rules must specify: allowed tool name patterns (glob), allowed user groups, allowed time windows (optional), and allowed resource scopes. Policy changes require review and must be version-controlled. Never use a default-allow policy in production.
4. **Config Portability Versioning** — MCP server configurations must carry a `config_version` field (semver). When a client connects with an older config version, the gateway must either auto-migrate to the current schema or reject with a clear migration error. Store config migration scripts in version control alongside the config schema. Never silently ignore unknown config fields — fail with a validation error.
5. **Scope Isolation Per Session** — Each MCP session must operate within a declared scope (e.g., workspace path, project ID, allowed data sources). The gateway enforces scope boundaries: tool calls attempting to access resources outside the declared scope are blocked and logged. Scope is set at session initiation and cannot be expanded mid-session without re-authentication.
6. **Rate Limiting and Quota Enforcement** — Apply per-user and per-tool rate limits at the gateway layer. Defaults: 60 tool calls per minute per user, 1000 per day per user, 10 concurrent sessions per user. Quota exceeded responses must include a `Retry-After` header and a quota-reset timestamp. Quota limits should be configurable per role or group via policy.
7. **Secret and Credential Handling** — MCP tool parameters must never contain raw API keys, passwords, or tokens. Enforce this at the gateway with a secrets-detection scanner on inbound parameters. Tools requiring credentials must reference a secret store (e.g., AWS Secrets Manager, Vault) by path, not by value. The gateway resolves secret references server-side and never echoes resolved values in audit logs.
8. **Incident Response Integration** — Configure the MCP gateway to emit security events to the enterprise SIEM on: repeated auth failures (>5 in 60s), policy violations, anomalous tool usage patterns (sudden spike in file reads), and tool calls from new geographic locations. Include playbook links in SIEM alerts so on-call engineers can act without context-building delay.
