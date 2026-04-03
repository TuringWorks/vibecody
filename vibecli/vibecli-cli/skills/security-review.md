---
name: Security Review
description: Perform a security review of code changes
triggers: ["security review", "security audit", "vulnerability", "owasp", "pen test"]
---

When performing a security review:
1. Read all changed files carefully
2. Check for OWASP Top 10 vulnerabilities:
   - SQL injection (parameterize queries, never concatenate user input)
   - XSS (sanitize output, use template engines with auto-escaping)
   - Command injection (avoid shell execution with user input)
   - Path traversal (validate and sanitize file paths)
   - Authentication/authorization flaws
   - Sensitive data exposure (hardcoded secrets, verbose errors)
   - CSRF (verify anti-CSRF tokens)
   - Insecure deserialization
   - Security misconfiguration
   - Insufficient logging
3. Check for hardcoded credentials, API keys, tokens
4. Verify input validation at system boundaries
5. Check dependency versions for known CVEs
6. Report findings with severity (critical/high/medium/low) and remediation steps
