---
triggers: ["security hardening", "threat model", "security audit", "hardening checklist", "security review"]
tools_allowed: ["read_file", "write_file", "bash"]
category: workflow
---

# Security Hardening Workflow

When hardening application security:

1. **Threat model**: identify assets, threats, entry points — STRIDE framework (Spoofing, Tampering, Repudiation, Information disclosure, Denial of service, Elevation of privilege)
2. **Dependencies**: run `npm audit` / `cargo audit` / `pip-audit` — fix critical/high CVEs
3. **Secrets scan**: use `gitleaks` or `trufflehog` — ensure no secrets in git history
4. **Headers**: add CSP, HSTS, X-Content-Type-Options, X-Frame-Options, Referrer-Policy
5. **Authentication**: enforce MFA, rate-limit login, use secure session management
6. **Authorization**: verify permissions server-side for every endpoint — never trust client
7. **Input validation**: validate all user input at boundaries — whitelist, not blacklist
8. **Database**: use parameterized queries only — never concatenate user input into SQL
9. **Encryption**: TLS everywhere, encrypt sensitive data at rest, rotate keys
10. **Logging**: log security events (auth, access, errors) — but never log secrets/PII
11. **Network**: minimize open ports, use firewalls, isolate internal services
12. **Review**: schedule regular security audits — automated scanning + manual review
