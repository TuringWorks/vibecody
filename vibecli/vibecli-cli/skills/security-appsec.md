---
triggers: ["application security", "OWASP", "SAST", "DAST", "penetration testing", "vulnerability scanning", "secure coding", "threat modeling", "security review", "CVE"]
tools_allowed: ["read_file", "write_file", "bash"]
category: security
---

# Application Security

When working with application security, vulnerability management, and secure development:

1. Prevent injection attacks (OWASP A03) by using parameterized queries or ORM methods for all database access, never concatenating user input into SQL/NoSQL/LDAP queries; apply the same principle to OS command execution by using array-based APIs instead of shell interpolation.

2. Integrate SAST (Static Application Security Testing) into CI pipelines using tools like Semgrep, CodeQL, or SonarQube; configure custom rules for your framework, fail builds on high/critical findings, and suppress false positives with inline annotations that require security team approval.

3. Run DAST (Dynamic Application Security Testing) against staging environments using OWASP ZAP or Burp Suite in CI; configure authenticated scanning with valid session tokens, target all API endpoints from the OpenAPI spec, and triage findings by CVSS score before promotion to production.

4. Conduct threat modeling using STRIDE (Spoofing, Tampering, Repudiation, Information Disclosure, Denial of Service, Elevation of Privilege) for every new feature or architecture change; document trust boundaries, data flows, and entry points, then derive security requirements from identified threats.

5. Scan dependencies for known vulnerabilities continuously using Dependabot, Snyk, or Trivy; pin dependency versions, set up automated PRs for security patches, block deployments when critical CVEs are present, and maintain a Software Bill of Materials (SBOM) in CycloneDX or SPDX format.

6. Manage secrets with a dedicated vault (HashiCorp Vault, AWS Secrets Manager, or 1Password Connect); never commit secrets to version control, rotate credentials on a schedule, use short-lived tokens where possible, and scan git history with tools like gitleaks or truffleHog.

7. Configure security headers on all HTTP responses: `Strict-Transport-Security` (HSTS with includeSubDomains and preload), `Content-Security-Policy` (restrictive default-src), `X-Content-Type-Options: nosniff`, `X-Frame-Options: DENY`, `Referrer-Policy: strict-origin-when-cross-origin`, and `Permissions-Policy` to disable unused browser features.

8. Implement Content Security Policy (CSP) with a strict approach: use nonce-based or hash-based script allowlisting instead of `unsafe-inline`, avoid `unsafe-eval`, report violations to a CSP reporting endpoint, and iterate on the policy using report-only mode before enforcement.

9. Secure authentication with modern standards: implement FIDO2/WebAuthn passkeys as the primary factor, support TOTP as a fallback, enforce minimum password complexity with bcrypt/argon2 hashing (cost factor >= 12), implement account lockout with exponential backoff, and protect against credential stuffing with rate limiting.

10. Secure APIs with OAuth 2.0 using PKCE for public clients, validate JWTs on every request (check signature, issuer, audience, expiry), implement token rotation with short-lived access tokens (5-15 minutes) and longer refresh tokens with revocation support, and enforce scope-based authorization.

11. Implement comprehensive security logging: log authentication events, authorization failures, input validation failures, and administrative actions with timestamps, user IDs, source IPs, and request IDs; ship logs to a SIEM (Splunk, Elastic Security) and set up alerting rules for anomalous patterns.

12. Automate incident response with runbooks: detect anomalies via monitoring alerts, automatically isolate compromised services, revoke affected credentials, capture forensic artifacts (memory dumps, logs), notify stakeholders per the communication plan, and conduct blameless post-mortems.

13. Implement rate limiting and abuse prevention at multiple layers: reverse proxy (Nginx/Envoy) for global rate limits, application-level per-user/per-endpoint limits, CAPTCHA for bot-susceptible flows, and progressive penalties (temporary bans) for repeated abuse.

14. Validate all input on the server side regardless of client-side validation; use allowlist validation (expected patterns) rather than denylist (known-bad patterns), enforce maximum lengths, validate content types, and sanitize output contextually (HTML encoding for web, parameterization for SQL).
