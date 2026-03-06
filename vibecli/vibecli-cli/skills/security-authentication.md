---
triggers: ["OAuth2", "JWT", "session management", "MFA", "PKCE", "authentication", "login security", "access token"]
tools_allowed: ["read_file", "write_file", "bash"]
category: security
---

# Authentication & Authorization Security

When implementing authentication:

1. Use OAuth 2.0 with PKCE for public clients (SPAs, mobile) — never implicit flow
2. Store JWTs in `httpOnly`, `secure`, `sameSite=strict` cookies — not localStorage
3. JWT: use short-lived access tokens (15min) + long-lived refresh tokens (7d) with rotation
4. Hash passwords with `bcrypt` (cost 12+) or `argon2id` — never MD5/SHA without salt
5. Implement rate limiting on login endpoints — exponential backoff after 5 failed attempts
6. Session management: regenerate session ID after login, invalidate on logout
7. MFA: prefer TOTP (authenticator apps) over SMS — use backup codes as fallback
8. Validate redirect URIs exactly — open redirect is a common OAuth vulnerability
9. Use `state` parameter in OAuth flows to prevent CSRF attacks
10. API keys: generate 256-bit random tokens, hash before storage, support key rotation
11. Authorization: implement RBAC or ABAC — check permissions at the service layer, not just UI
12. Always verify token signature, expiration, issuer, and audience claims
