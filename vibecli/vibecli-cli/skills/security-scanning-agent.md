# Security Scanning in Agent Flow

Inline security analysis that scans code for vulnerabilities as the agent generates or edits.

## Triggers
- "security scan", "vulnerability check", "OWASP scan"
- "secret detection", "code security", "SAST"

## Usage
```
/scan file src/auth.rs            # Scan a file
/scan diff                        # Scan only changed lines
/scan summary                     # Show finding summary
/scan suppress finding-1          # Suppress a finding
/scan pattern "DANGER" critical   # Add custom pattern
```

## Features
- 13 vulnerability classes covering OWASP Top 10+: SQL injection, XSS, command injection, path traversal, hardcoded secrets, insecure deserialization, SSRF, open redirect, weak crypto, missing auth, buffer overflow, race condition
- 5 severity levels with scoring: Critical (10), High (8), Medium (5), Low (2), Info (0)
- 7 default scan patterns: eval(), exec(), password=, api_key=, innerHTML, md5(), SELECT * FROM
- Diff-aware scanning for incremental PR analysis
- Inline suppression via `// nosec` or `# nosec` comments
- Per-finding suppression
- Custom pattern support
- Summary with severity breakdown and suppressed count
- Blocking threshold (configurable minimum severity)
