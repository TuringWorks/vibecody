---
layout: page
title: "Demo 35: Compliance & Audit"
permalink: /demos/35-compliance/
---

# Demo 35: Compliance & Audit

## Overview

VibeCody includes built-in compliance tooling for SOC 2 readiness. The framework provides 15 default technical controls mapped to 5 Trust Service Criteria, audit logging with 10 action types, PII redaction with 4 strategies, compliance scoring, and Markdown report generation for auditors. Whether you are preparing for an audit or maintaining continuous compliance, these tools integrate directly into your development workflow.

## Prerequisites

- VibeCLI installed and on your PATH
- VibeCody running with audit logging enabled (`audit_logging = true` in `~/.vibecli/config.toml`)
- For VibeUI: the desktop app running with the **Compliance** panel visible

## SOC 2 Trust Service Criteria and Controls

VibeCody maps 15 technical controls across the 5 Trust Service Criteria:

| Trust Service Criteria     | Controls                                                        |
|----------------------------|-----------------------------------------------------------------|
| **Security**               | Access Control, Encryption at Rest, Encryption in Transit       |
| **Availability**           | Health Monitoring, Backup & Recovery, Redundancy                |
| **Processing Integrity**   | Input Validation, Output Verification, Error Handling           |
| **Confidentiality**        | PII Redaction, Data Classification, Key Rotation                |
| **Privacy**                | Consent Tracking, Data Retention, Right to Deletion             |

## Step-by-Step Walkthrough

### 1. Check Compliance Status

Run a compliance scan to evaluate all 15 controls and produce a score.

**CLI:**

```bash
vibecli compliance status
```

Example output:

```
Compliance Status
Framework: SOC 2 Type II
Controls: 15 evaluated

Trust Service Criteria         Controls   Pass   Warn   Fail
Security                       3          3      0      0
Availability                   3          2      1      0
Processing Integrity           3          3      0      0
Confidentiality                3          2      1      0
Privacy                        3          2      0      1

Overall Score: 80% (12/15 passing)

Warnings:
  [AVAIL-02] Backup & Recovery: no backup schedule configured
  [CONF-02] Data Classification: 3 files missing classification labels

Failures:
  [PRIV-03] Right to Deletion: deletion endpoint not implemented
```

**VibeUI:**

Open the **Compliance** panel. The main dashboard shows a compliance score gauge, per-criteria breakdowns, and a list of findings.

### 2. Review Audit Logs

VibeCody records 10 action types in the audit log. Each entry includes a timestamp, actor, action type, resource, and outcome.

**CLI:**

```bash
vibecli compliance audit-log --last 20
```

Example output:

```
Audit Log (last 20 entries):
  2026-03-13T14:22:01Z  user:alice   FileRead       src/main.rs          success
  2026-03-13T14:21:58Z  user:alice   FileWrite      src/routes/auth.rs   success
  2026-03-13T14:21:45Z  agent:batch  CodeGenerate   src/models/user.rs   success
  2026-03-13T14:20:12Z  user:bob     ConfigChange   config.toml          success
  2026-03-13T14:19:30Z  agent:review CodeReview     src/db/schema.rs     success
  2026-03-13T14:18:05Z  system       KeyRotation    api-key-anthropic    success
  2026-03-13T14:15:00Z  user:alice   Login          session:abc123       success
  2026-03-13T14:14:22Z  agent:batch  ToolExecution  bash:cargo-test      success
  2026-03-13T14:12:10Z  user:alice   PermChange     user:bob → admin     success
  2026-03-13T14:10:00Z  system       DataExport     report-march.csv     success
  ...
```

**10 Audit Action Types:**

| Action Type     | Description                                  |
|-----------------|----------------------------------------------|
| `FileRead`      | A file was read by a user or agent           |
| `FileWrite`     | A file was created or modified               |
| `CodeGenerate`  | AI-generated code was written to disk        |
| `CodeReview`    | An agent reviewed code                       |
| `ConfigChange`  | Configuration was modified                   |
| `KeyRotation`   | An API key or secret was rotated             |
| `Login`         | A user authenticated                         |
| `ToolExecution` | An agent executed a tool (bash, file, etc.)  |
| `PermChange`    | User permissions were modified               |
| `DataExport`    | Data was exported from the system            |

**VibeUI:**

The audit log is available in the Compliance panel under the **Audit Log** tab, with filters for action type, actor, date range, and resource.

### 3. Configure PII Redaction

PII redaction detects and sanitizes sensitive data in AI inputs and outputs. Four detection patterns and four redaction strategies are available.

**Detected PII Types:**

| PII Type   | Pattern Example                       |
|------------|----------------------------------------|
| Email      | `user@example.com`                     |
| API Key    | `sk-abc123...`, `AKIA...`              |
| IP Address | `192.168.1.100`, `2001:db8::1`         |
| Name       | Personal names detected via NER        |

**4 Redaction Strategies:**

| Strategy     | Example                                         |
|--------------|-------------------------------------------------|
| **Hash**     | `user@example.com` becomes `sha256:a1b2c3...`  |
| **Mask**     | `user@example.com` becomes `u***@e******.com`   |
| **Remove**   | `user@example.com` becomes `[REDACTED]`         |
| **Tokenize** | `user@example.com` becomes `PII_EMAIL_001`      |

**CLI:**

```bash
vibecli compliance pii-config --strategy mask --types email,api-key,ip,name
```

**CLI (test redaction):**

```bash
vibecli compliance pii-test "Contact alice@example.com or call 192.168.1.1"
```

Example output:

```
Original: Contact alice@example.com or call 192.168.1.1
Redacted: Contact a****@e******.com or call 1**.***.*.*
Detections: 2 (email, ip_address)
```

**VibeUI:**

In the Compliance panel, open the **PII Redaction** settings to select the strategy and toggle PII types.

### 4. Generate a Compliance Report

Produce a Markdown report suitable for sharing with auditors. The report includes the compliance score, per-control findings, evidence references, and remediation recommendations.

**CLI:**

```bash
vibecli compliance report --format markdown --output compliance-report-2026-03.md
```

Example output file excerpt:

```markdown
# SOC 2 Compliance Report
**Generated:** 2026-03-13
**Period:** 2026-01-01 to 2026-03-13
**Overall Score:** 80% (12/15 controls passing)

## Security (3/3 Passing)
### SEC-01: Access Control Yes
- RBAC enforced for all users and agents
- Evidence: audit log entries for PermChange actions

### SEC-02: Encryption at Rest Yes
- All data at rest encrypted with AES-256
...
```

**VibeUI:**

Click **Generate Report** in the Compliance panel toolbar. Choose the date range and format (Markdown or PDF). The report opens in a preview pane.

### 5. Data Retention Policies

Configure how long audit logs, session data, and generated artifacts are retained.

**CLI:**

```bash
vibecli compliance retention --audit-logs 365d --sessions 90d --artifacts 180d
```

Example output:

```
Data Retention Policies Updated:
  Audit logs:    365 days
  Sessions:       90 days
  Artifacts:     180 days
  Next purge:    2026-04-01 (28 records eligible)
```

### 6. RBAC and Key Rotation

Review role-based access control settings and trigger key rotation.

**CLI (view RBAC):**

```bash
vibecli compliance rbac list
```

Example output:

```
Roles:
  admin     Full access (users: alice)
  developer Read/write code, run agents (users: bob, carol)
  viewer    Read-only access (users: dave)
  auditor   Compliance panel + audit logs only (users: eve)
```

**CLI (rotate keys):**

```bash
vibecli compliance rotate-keys --provider anthropic
```

Example output:

```
Key Rotation:
  Provider: Anthropic
  Old key:  sk-ant-...XXXX (deactivated)
  New key:  sk-ant-...YYYY (active)
  Audit entry: KeyRotation logged at 2026-03-13T14:30:00Z
```

### 7. Continuous Compliance Monitoring

Run compliance checks on a schedule to catch regressions early.

**CLI:**

```bash
vibecli compliance watch --interval 1h
```

This runs a compliance scan every hour and alerts on any score changes.

## Demo Recording JSON

```json
{
  "demo_id": "35-compliance",
  "title": "Compliance & Audit",
  "version": "1.0.0",
  "steps": [
    {
      "action": "cli_command",
      "command": "vibecli compliance status",
      "description": "Run compliance scan and view score across 15 controls"
    },
    {
      "action": "cli_command",
      "command": "vibecli compliance audit-log --last 20",
      "description": "Review recent audit log entries"
    },
    {
      "action": "cli_command",
      "command": "vibecli compliance pii-config --strategy mask --types email,api-key,ip,name",
      "description": "Configure PII redaction strategy and types"
    },
    {
      "action": "cli_command",
      "command": "vibecli compliance pii-test \"Contact alice@example.com or call 192.168.1.1\"",
      "description": "Test PII redaction on sample text"
    },
    {
      "action": "cli_command",
      "command": "vibecli compliance report --format markdown --output compliance-report-2026-03.md",
      "description": "Generate a Markdown compliance report for auditors"
    },
    {
      "action": "cli_command",
      "command": "vibecli compliance retention --audit-logs 365d --sessions 90d --artifacts 180d",
      "description": "Configure data retention policies"
    },
    {
      "action": "cli_command",
      "command": "vibecli compliance rbac list",
      "description": "View RBAC roles and assignments"
    },
    {
      "action": "cli_command",
      "command": "vibecli compliance rotate-keys --provider anthropic",
      "description": "Rotate API keys with audit logging"
    },
    {
      "action": "vibeui_interaction",
      "panel": "Compliance",
      "tab": "Dashboard",
      "description": "View compliance score gauge and per-criteria breakdown"
    },
    {
      "action": "vibeui_interaction",
      "panel": "Compliance",
      "tab": "Audit Log",
      "description": "Browse and filter audit log entries"
    },
    {
      "action": "vibeui_interaction",
      "panel": "Compliance",
      "tab": "PII Redaction",
      "description": "Configure PII detection and redaction settings"
    }
  ]
}
```

## What's Next

- [Demo 31: Batch Builder](../31-batch-builder/) -- Run large batch jobs with full audit trails
- [Demo 34: Usage Metering](../34-usage-metering/) -- Track token usage for compliance reporting
- [Demo 32: Legacy Migration](../32-legacy-migration/) -- Ensure migrated code meets compliance standards
