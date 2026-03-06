---
triggers: ["input validation", "sanitize", "XSS", "SQL injection", "OWASP", "security"]
tools_allowed: ["read_file", "write_file", "bash"]
category: security
---

# Input Validation & Security

1. Validate ALL user input at system boundaries (API endpoints, CLI args, form data)
2. Use parameterized queries — NEVER string-concatenate SQL
3. HTML-encode output to prevent XSS: `&`, `<`, `>`, `"`, `'`
4. Use allowlists over denylists for input validation
5. Validate file paths to prevent traversal: reject `..`, resolve and check prefix
6. Limit request body size and rate-limit API endpoints
7. Use constant-time comparison for secrets (`hmac::equal()`, `crypto.timingSafeEqual()`)
8. Never log secrets, tokens, or passwords — redact before logging
9. Use CSRF tokens for state-changing operations
10. Set security headers: CSP, X-Frame-Options, X-Content-Type-Options
