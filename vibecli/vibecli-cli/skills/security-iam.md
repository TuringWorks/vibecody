---
triggers: ["IAM", "identity management", "OAuth", "OIDC", "SAML", "SSO", "RBAC", "ABAC", "zero trust", "MFA", "SCIM", "directory service"]
tools_allowed: ["read_file", "write_file", "bash"]
category: security
---

# Identity and Access Management

When working with IAM, authentication protocols, and access control systems:

1. Implement OAuth 2.0 Authorization Code flow with PKCE for all client types (web, mobile, SPA, CLI); generate a cryptographically random code_verifier (43-128 characters), derive code_challenge with S256, and validate on the authorization server to prevent authorization code interception.

2. Configure OpenID Connect (OIDC) for user identity by requesting the `openid`, `profile`, and `email` scopes; validate the ID token's signature against the provider's JWKS endpoint, verify `iss`, `aud`, `exp`, `nonce`, and `at_hash` claims before establishing a session.

3. Implement SAML 2.0 federation for enterprise SSO by configuring your service as a SAML SP; validate the assertion's XML signature (use canonicalization-aware libraries to prevent XML signature wrapping attacks), check `NotBefore`/`NotOnOrAfter` conditions, and map SAML attributes to local user profiles.

4. Design SSO architecture with a centralized identity provider (Keycloak, Auth0, Okta, or Azure AD); implement session synchronization across services via shared session tokens or back-channel logout (OIDC Back-Channel Logout spec), and handle IdP-initiated vs SP-initiated flows.

5. Choose between RBAC and ABAC based on authorization complexity: use RBAC (role-based) for straightforward hierarchical permissions with role inheritance, switch to ABAC (attribute-based) when access decisions depend on resource attributes, environmental context (time, location), or complex policies expressible in XACML or OPA/Rego.

6. Implement MFA with WebAuthn/FIDO2 as the strongest factor (phishing-resistant, hardware-bound credentials); support TOTP (RFC 6238) as a fallback with 6-digit codes and 30-second windows, and avoid SMS-based OTP due to SIM-swap vulnerability unless required for accessibility.

7. Automate user provisioning and deprovisioning with SCIM 2.0 (RFC 7644); implement the `/Users` and `/Groups` endpoints with proper filtering (`filter=userName eq "john"`), support PATCH for incremental updates, and ensure deprovisioning immediately revokes all active sessions and tokens.

8. Follow JWT best practices: use asymmetric signing (RS256 or ES256) for tokens consumed by multiple services, set short expiration times (5-15 minutes for access tokens), include only necessary claims to minimize token size, never store sensitive data in JWT payloads, and validate all registered claims on every request.

9. Implement secure session management with server-side session stores (Redis with encryption-at-rest); set cookies with `Secure`, `HttpOnly`, `SameSite=Lax` (or `Strict`), and `__Host-` prefix; regenerate session IDs after authentication, and implement absolute and idle timeouts.

10. Design zero trust architecture by authenticating and authorizing every request regardless of network location; use mutual TLS (mTLS) for service-to-service communication, implement device trust verification, enforce least-privilege access policies, and continuously evaluate trust signals (device posture, user behavior, risk score).

11. Integrate with directory services (LDAP/Active Directory) using connection pooling and read replicas for performance; bind with a service account (never anonymous bind in production), use LDAPS or StartTLS for encryption, implement group-based access mapping, and cache directory lookups with short TTLs.

12. Implement token refresh strategies that balance security and user experience: use rotating refresh tokens (one-time use with automatic revocation of the token family on reuse detection), store refresh tokens server-side or in secure HTTP-only cookies, and implement a maximum refresh token lifetime.

13. Build consent management for OAuth scopes that records explicit user consent per client and scope combination; allow users to review and revoke granted permissions, implement incremental consent (request new scopes only when needed), and comply with GDPR consent requirements.

14. Implement centralized audit logging for all IAM events: login success/failure, MFA enrollment/usage, role assignments, permission changes, token issuance/revocation, and SCIM provisioning actions; retain logs per compliance requirements (SOC 2, ISO 27001) and alert on anomalous patterns (impossible travel, brute force).

15. Design for identity federation across organizational boundaries using OIDC Federation or SAML metadata exchange; implement just-in-time (JIT) provisioning for federated users, map external identity attributes to internal roles, and establish trust relationships with certificate pinning or metadata signature validation.
