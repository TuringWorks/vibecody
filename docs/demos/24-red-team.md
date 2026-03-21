---
layout: page
title: "Demo 24: Red Team Security"
permalink: /demos/red-team/
nav_order: 24
parent: Demos
---


## Overview

VibeCody's Red Team module provides automated security scanning that checks your codebase against the OWASP Top 10, common vulnerability patterns, dependency vulnerabilities, and custom security rules. The AI acts as an adversarial security auditor, probing for path traversal, SQL injection, XSS, insecure deserialization, and more. Results are presented as a prioritized security report with remediation suggestions that the AI can apply automatically.

**Time to complete:** ~15 minutes

## Prerequisites

- VibeCody installed and configured with at least one AI provider
- A project with source code to scan
- (Optional) `cargo-audit`, `npm audit`, or `pip-audit` for dependency scanning
- (Optional) VibeUI installed for the graphical Red Team panel

## Step-by-Step Walkthrough

### Step 1: Run a full security scan

Open the VibeCLI REPL and initiate a scan.

```bash
vibecli repl
```

```
/redteam scan
```

VibeCody scans the entire project, checking for common vulnerability patterns.

```
Red Team Security Scan
======================
Project: /path/to/your/project
Scanning 342 source files...

Phase 1/4: Static analysis.............. done (12.3s)
Phase 2/4: OWASP Top 10 checks......... done (8.7s)
Phase 3/4: Dependency audit............. done (5.2s)
Phase 4/4: AI adversarial analysis...... done (15.1s)

Scan complete in 41.3s

Results:
  CRITICAL:  1
  HIGH:      3
  MEDIUM:    7
  LOW:       12
  INFO:      5

Total findings: 28
Report saved to: .vibecli/security/scan-2026-03-13.json
```

### Step 2: Review findings by severity

```
/redteam report
```

```
=== CRITICAL (1) ===

[C-001] SQL Injection in user query
  File: src/db/queries.rs:45
  Type: OWASP A03:2021 - Injection
  Code:
    let query = format!("SELECT * FROM users WHERE id = '{}'", user_input);
  Risk: Attacker can execute arbitrary SQL via crafted user_input
  Fix:  Use parameterized queries instead of string interpolation

=== HIGH (3) ===

[H-001] Path Traversal in file download endpoint
  File: src/api/files.rs:78
  Type: OWASP A01:2021 - Broken Access Control
  Code:
    let path = format!("uploads/{}", request.filename);
    let content = fs::read_to_string(path)?;
  Risk: Attacker can read arbitrary files via "../../../etc/passwd"
  Fix:  Canonicalize path and verify it stays within uploads directory

[H-002] Missing authentication on admin endpoint
  File: src/api/admin.rs:12
  Type: OWASP A07:2021 - Identification and Authentication Failures
  Code:
    #[get("/admin/users")]
    async fn list_all_users(db: &State<Db>) -> Json<Vec<User>> {
  Risk: Any unauthenticated user can list all user records
  Fix:  Add authentication guard middleware

[H-003] Hardcoded secret key
  File: src/config.rs:23
  Type: OWASP A02:2021 - Cryptographic Failures
  Code:
    const JWT_SECRET: &str = "super-secret-key-change-me";
  Risk: Known secret enables token forgery
  Fix:  Load from environment variable or secrets manager

=== MEDIUM (7) ===

[M-001] Cross-Site Scripting (XSS) in template rendering
  File: src/templates/user_profile.rs:34
  Type: OWASP A03:2021 - Injection
  ...

[M-002] Insecure deserialization of user input
  File: src/api/webhooks.rs:56
  Type: OWASP A08:2021 - Software and Data Integrity Failures
  ...

(and 5 more medium findings)

=== LOW (12) ===
(summary of low-severity findings)

=== INFO (5) ===
(informational notes about security best practices)
```

### Step 3: Scan for specific vulnerability types

Focus on a single category.

```
/redteam scan --type injection
```

```
Scanning for injection vulnerabilities...
  Checked: SQL injection, NoSQL injection, Command injection, LDAP injection, XPath injection

Findings:
  [C-001] SQL Injection in src/db/queries.rs:45
  [M-001] XSS in src/templates/user_profile.rs:34
  [M-005] Command injection in src/utils/shell.rs:22

3 findings (1 critical, 2 medium)
```

Other scan types:

```
/redteam scan --type auth          # Authentication and authorization issues
/redteam scan --type crypto        # Cryptographic failures
/redteam scan --type dependencies  # Vulnerable dependencies
/redteam scan --type config        # Security misconfiguration
/redteam scan --type xss           # Cross-site scripting
/redteam scan --type path-traversal  # Path traversal attacks
```

### Step 4: Audit dependencies

```
/redteam audit --deps
```

```
Dependency Vulnerability Audit
==============================

Rust dependencies (Cargo.lock):
  [HIGH] serde_yaml 0.8.23 - CVE-2024-XXXX: Arbitrary code execution
    Fix: Upgrade to serde_yaml 0.9.0+
  [MEDIUM] hyper 0.14.18 - CVE-2024-YYYY: Request smuggling
    Fix: Upgrade to hyper 0.14.27+
  [LOW] regex 1.5.4 - CVE-2023-ZZZZ: ReDoS via crafted pattern
    Fix: Upgrade to regex 1.7.0+

Node.js dependencies (package-lock.json):
  [HIGH] jsonwebtoken 8.5.1 - CVE-2024-AAAA: Algorithm confusion
    Fix: Upgrade to jsonwebtoken 9.0.0+
  [MEDIUM] express 4.17.1 - CVE-2024-BBBB: Open redirect
    Fix: Upgrade to express 4.18.2+

Total: 5 vulnerable dependencies (2 high, 2 medium, 1 low)
```

### Step 5: AI-powered adversarial analysis

The AI acts as a red team attacker, reasoning about attack vectors specific to your application.

```
/redteam scan --ai-deep
```

```
AI Adversarial Analysis
=======================

[AI-001] Authentication bypass via race condition
  File: src/middleware/auth.rs:67
  Attack: Two concurrent requests with an expiring token could both pass
         validation if the token check and the database lookup are not atomic.
  Impact: Temporary authentication bypass
  Recommendation: Use atomic check-and-load, or accept slight timing window
  Confidence: Medium

[AI-002] Information leakage via error messages
  File: src/api/handlers.rs:120
  Attack: Database errors are returned verbatim to the client, revealing
         table names, column names, and query structure.
  Impact: Aids attacker reconnaissance
  Recommendation: Map internal errors to generic user-facing messages
  Confidence: High

[AI-003] Denial of service via unbounded pagination
  File: src/api/list.rs:45
  Attack: GET /api/items?limit=999999999 returns the entire database
  Impact: Memory exhaustion, service disruption
  Recommendation: Enforce maximum page size (e.g., 100)
  Confidence: High
```

### Step 6: Auto-fix vulnerabilities

Ask the AI to fix a specific finding.

```
/redteam fix C-001
```

```
Fixing [C-001] SQL Injection in src/db/queries.rs:45

Before:
  let query = format!("SELECT * FROM users WHERE id = '{}'", user_input);
  let result = db.execute(&query)?;

After:
  let result = db.execute(
      "SELECT * FROM users WHERE id = $1",
      &[&user_input],
  )?;

Applied fix to src/db/queries.rs
Running tests... 142 passed, 0 failed
```

Fix all high and critical findings at once:

```
/redteam fix --severity critical,high
```

```
Fixing 4 findings...
  [C-001] SQL Injection       -> Fixed (parameterized query)
  [H-001] Path Traversal      -> Fixed (path canonicalization + boundary check)
  [H-002] Missing Auth         -> Fixed (added auth guard middleware)
  [H-003] Hardcoded Secret    -> Fixed (read from env var VIBECODY_JWT_SECRET)

4 fixes applied. Running tests... 142 passed, 0 failed
Re-scanning... 0 critical, 0 high remaining
```

### Step 7: Generate a security report

```
/redteam report --format markdown --output security-report.md
```

```
Security report generated: security-report.md
  Scan date: 2026-03-13
  Total findings: 28 (before fixes), 21 (after fixes)
  Fixed: 7 (1 critical, 3 high, 3 medium)
  Remaining: 21 (0 critical, 0 high, 4 medium, 12 low, 5 info)
  OWASP coverage: 10/10 categories checked
```

Other formats: `--format json`, `--format html`, `--format sarif` (for CI integration).

### Step 8: Using the Red Team panel in VibeUI

Open the **Red Team** panel from the sidebar.

1. **Scan Tab** -- Click "Full Scan" or select specific categories. Progress bar shows scan phases.
2. **Findings Tab** -- Sortable table of findings by severity, type, and file. Click a finding to jump to the affected line in the editor with the vulnerability highlighted.
3. **Dependencies Tab** -- List of vulnerable dependencies with upgrade buttons.
4. **Report Tab** -- Generate and download reports in markdown, JSON, HTML, or SARIF format.
5. **Fix Tab** -- Select findings and click "Auto-Fix". The AI applies fixes and re-runs tests.

The editor integration highlights vulnerable code with red underlines and shows the finding description in a hover tooltip.

## OWASP Top 10 Coverage

| OWASP Category | Checks Performed |
|----------------|-----------------|
| A01: Broken Access Control | Missing auth, path traversal, CORS misconfiguration |
| A02: Cryptographic Failures | Hardcoded secrets, weak algorithms, missing encryption |
| A03: Injection | SQL, NoSQL, command, XSS, LDAP, XPath |
| A04: Insecure Design | Business logic flaws, missing rate limiting |
| A05: Security Misconfiguration | Debug mode, default credentials, open endpoints |
| A06: Vulnerable Components | Dependency CVE audit |
| A07: Auth Failures | Weak passwords, missing MFA, session fixation |
| A08: Data Integrity | Insecure deserialization, unsigned updates |
| A09: Logging Failures | Missing audit logs, sensitive data in logs |
| A10: SSRF | URL scheme validation, internal network access |

## CLI Command Reference

| Command | Description |
|---------|-------------|
| `/redteam scan` | Full security scan of the project |
| `/redteam scan --type <category>` | Scan for a specific vulnerability type |
| `/redteam scan --ai-deep` | AI adversarial analysis for complex attack vectors |
| `/redteam audit --deps` | Audit dependencies for known CVEs |
| `/redteam report` | Display the latest scan report |
| `/redteam report --format <fmt> --output <file>` | Export report (markdown, json, html, sarif) |
| `/redteam fix <finding-id>` | Auto-fix a specific finding |
| `/redteam fix --severity <levels>` | Auto-fix all findings at given severity levels |

## Demo Recording

```json
{
  "demoRecording": {
    "version": "1.0",
    "title": "Red Team Security Demo",
    "description": "Run OWASP security scans, review findings, audit dependencies, and auto-fix vulnerabilities",
    "duration_seconds": 300,
    "steps": [
      {
        "timestamp": 0,
        "action": "repl_command",
        "command": "/redteam scan",
        "output": "Scanning 342 source files...\nPhase 1/4: Static analysis... done\nPhase 2/4: OWASP Top 10... done\nPhase 3/4: Dependency audit... done\nPhase 4/4: AI adversarial... done\n\nResults: 1 critical, 3 high, 7 medium, 12 low, 5 info",
        "narration": "Run a full security scan across the project"
      },
      {
        "timestamp": 35,
        "action": "repl_command",
        "command": "/redteam report",
        "output": "[C-001] SQL Injection in src/db/queries.rs:45\n[H-001] Path Traversal in src/api/files.rs:78\n[H-002] Missing auth on admin endpoint\n[H-003] Hardcoded secret key\n...",
        "narration": "Review findings sorted by severity"
      },
      {
        "timestamp": 65,
        "action": "repl_command",
        "command": "/redteam scan --type injection",
        "output": "Findings:\n  [C-001] SQL Injection\n  [M-001] XSS\n  [M-005] Command injection\n\n3 findings",
        "narration": "Scan for injection vulnerabilities only"
      },
      {
        "timestamp": 90,
        "action": "repl_command",
        "command": "/redteam audit --deps",
        "output": "Rust: 3 vulnerable packages\nNode.js: 2 vulnerable packages\nTotal: 5 (2 high, 2 medium, 1 low)",
        "narration": "Audit dependencies for known CVEs"
      },
      {
        "timestamp": 120,
        "action": "repl_command",
        "command": "/redteam scan --ai-deep",
        "output": "[AI-001] Race condition in auth middleware\n[AI-002] Information leakage via error messages\n[AI-003] DoS via unbounded pagination",
        "narration": "AI adversarial analysis finds subtle vulnerabilities"
      },
      {
        "timestamp": 160,
        "action": "repl_command",
        "command": "/redteam fix C-001",
        "output": "Fixing SQL Injection in src/db/queries.rs:45\nBefore: format!(\"SELECT...\")\nAfter: parameterized query\nTests: 142 passed, 0 failed",
        "narration": "Auto-fix the critical SQL injection finding"
      },
      {
        "timestamp": 190,
        "action": "repl_command",
        "command": "/redteam fix --severity critical,high",
        "output": "Fixing 4 findings...\n  C-001: Fixed\n  H-001: Fixed\n  H-002: Fixed\n  H-003: Fixed\nRe-scanning: 0 critical, 0 high remaining",
        "narration": "Auto-fix all critical and high severity findings at once"
      },
      {
        "timestamp": 225,
        "action": "repl_command",
        "command": "/redteam report --format markdown --output security-report.md",
        "output": "Security report generated: security-report.md\n  Before: 28 findings | After: 21 findings | Fixed: 7",
        "narration": "Export the security report as markdown"
      },
      {
        "timestamp": 255,
        "action": "ui_interaction",
        "panel": "RedTeam",
        "tab": "Findings",
        "action_detail": "click_finding_C001",
        "details": "Editor jumps to src/db/queries.rs:45 with red underline on vulnerable code",
        "narration": "Click a finding in VibeUI to jump to the affected code"
      },
      {
        "timestamp": 275,
        "action": "ui_interaction",
        "panel": "RedTeam",
        "tab": "Dependencies",
        "action_detail": "view_cve_list",
        "narration": "Review vulnerable dependencies with CVE details and upgrade paths"
      },
      {
        "timestamp": 290,
        "action": "ui_interaction",
        "panel": "RedTeam",
        "tab": "Report",
        "action_detail": "download_sarif",
        "narration": "Download SARIF report for CI pipeline integration"
      }
    ]
  }
}
```

## What's Next

- Integrate red team scans into your CI pipeline using the SARIF output format
- Combine red team scanning with agent teams to have a SecurityAuditor agent run scans continuously
- Use context bundles to pin security policies so the AI follows your organization's security standards
- [Demo 19: Context Bundles](../context-bundles/) -- Package project context for consistent AI interactions
- [Demo 20: Agent Teams](../agent-teams/) -- Coordinate multiple AI agents with specialized roles
