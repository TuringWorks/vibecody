---
triggers: ["OWASP", "SSRF", "XXE", "deserialization", "CSRF", "IDOR", "injection", "XSS", "broken access"]
tools_allowed: ["read_file", "write_file", "bash"]
category: security
---

# OWASP Top 10 Security

When protecting against OWASP Top 10 vulnerabilities:

1. **Injection** (A03): Use parameterized queries/prepared statements — never concatenate user input into SQL
2. **XSS** (A03): Escape all user-generated content in HTML output; use CSP headers
3. **SSRF** (A10): Validate and whitelist URLs — block private IP ranges (10.x, 172.16-31.x, 192.168.x)
4. **XXE** (A05): Disable external entity processing in XML parsers — use JSON when possible
5. **Broken Access Control** (A01): Check authorization server-side for every request — never trust client
6. **IDOR**: Use indirect references (UUIDs) not sequential IDs; verify object ownership
7. **CSRF**: Use anti-CSRF tokens for state-changing requests; `SameSite=Strict` cookies
8. **Insecure Deserialization** (A08): Validate and sanitize serialized data; prefer JSON over native serialization
9. **Security Misconfiguration** (A05): Disable debug modes, remove default credentials, minimize attack surface
10. **Cryptographic Failures** (A02): Encrypt sensitive data at rest and in transit; rotate keys
11. **Vulnerable Components** (A06): Keep dependencies updated; audit with automated tools
12. **Insufficient Logging** (A09): Log auth events, access failures, input validation failures — but never log secrets
