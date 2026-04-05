---
layout: page
title: "Demo 54: Blue Team & Purple Team Security"
permalink: /demos/54-blue-purple-team/
nav_order: 54
parent: Demos
---


## Overview

VibeCody integrates defensive security (Blue Team) and adversarial validation (Purple Team) directly into your development workflow. The Blue Team module provides incident management with P1-P4 severity, IOC tracking across 9 indicator types, SIEM integration for 8 platforms (Splunk, Sentinel, Elastic, QRadar, CrowdStrike, Wazuh, Datadog, SumoLogic), forensic case management, and detection rule authoring with platform-specific query generation. The Purple Team module maps to the MITRE ATT&CK framework with 14 tactics and 20 pre-loaded techniques, enabling attack simulation, detection validation, coverage gap analysis, and heatmap generation.

**Time to complete:** ~15 minutes

## Prerequisites

- VibeCLI v0.5.1 installed and on your PATH
- At least one AI provider configured
- (Optional) VibeUI running with the **BlueTeam** and **PurpleTeam** panels visible
- (Optional) A SIEM platform for export testing (Splunk, Elastic, etc.)

## Step-by-Step Walkthrough

### Step 1: Create a Security Incident

Open the VibeCLI REPL and create a new incident.

```bash
vibecli
```

```
/blueteam incident create "Suspicious login from unknown IP"
```

Expected output:

```
Incident Created
  ID:        INC-2026-0042
  Title:     Suspicious login from unknown IP
  Severity:  P3 (default, use --severity to override)
  Status:    Open
  Created:   2026-03-29T10:15:00Z
  Assignee:  unassigned

Tip: Add IOCs with /blueteam ioc add --incident INC-2026-0042
```

You can override the severity at creation time:

```
/blueteam incident create "Ransomware detected on build server" --severity P1
```

```
Incident Created
  ID:        INC-2026-0043
  Title:     Ransomware detected on build server
  Severity:  P1 (Critical)
  Status:    Open
  Created:   2026-03-29T10:16:00Z
  Assignee:  unassigned
```

### Step 2: Add Indicators of Compromise

Attach IOCs to an incident for tracking. VibeCody supports 9 IOC types: IP, Domain, URL, Hash, Email, File, Registry, Process, and Certificate.

```
/blueteam ioc add --incident INC-2026-0042 --type ip --value "203.0.113.45"
```

```
IOC Added
  Incident:  INC-2026-0042
  Type:      IP Address
  Value:     203.0.113.45
  Severity:  Medium
  Tags:      [suspicious-login]
  Total IOCs for incident: 1
```

Add a malicious domain:

```
/blueteam ioc add --incident INC-2026-0042 --type domain --value "evil-c2.example.net"
```

```
IOC Added
  Incident:  INC-2026-0042
  Type:      Domain
  Value:     evil-c2.example.net
  Severity:  High
  Tags:      [c2, suspicious-login]
  Total IOCs for incident: 2
```

### Step 3: Export Detection Rules to a SIEM

Generate platform-specific queries for your SIEM. VibeCody supports Splunk SPL, Elastic KQL, Sentinel KQL, QRadar AQL, CrowdStrike, Wazuh, Datadog, and SumoLogic.

```
/blueteam siem export splunk --incident INC-2026-0042
```

```
Splunk SPL Query Generated
  Incident: INC-2026-0042

  index=security sourcetype=firewall
  (src_ip="203.0.113.45" OR dest_ip="203.0.113.45")
  OR (query="evil-c2.example.net" OR url="*evil-c2.example.net*")
  | stats count by src_ip, dest_ip, action, _time
  | sort -_time

Saved to: blueteam/exports/INC-2026-0042-splunk.spl
```

Export the same IOCs for Elastic:

```
/blueteam siem export elastic --incident INC-2026-0042
```

```
Elastic KQL Query Generated
  Incident: INC-2026-0042

  source.ip: "203.0.113.45" or destination.ip: "203.0.113.45"
  or dns.question.name: "evil-c2.example.net"
  or url.domain: "evil-c2.example.net"

Saved to: blueteam/exports/INC-2026-0042-elastic.kql
```

### Step 4: Run a Purple Team Exercise

Create a MITRE ATT&CK exercise to validate your detection capabilities.

```
/purpleteam exercise new "MITRE ATT&CK T1059"
```

```
Purple Team Exercise Created
  ID:          PTE-2026-0018
  Technique:   T1059 - Command and Scripting Interpreter
  Tactic:      Execution
  Description: Adversaries may abuse command and script interpreters
               to execute commands, scripts, or binaries.
  Sub-techniques:
    T1059.001  PowerShell
    T1059.003  Windows Command Shell
    T1059.004  Unix Shell
    T1059.006  Python

  Simulations: 4 prepared
  Status:      Ready

Run /purpleteam simulate PTE-2026-0018 to execute simulations.
```

Run the simulation:

```
/purpleteam simulate PTE-2026-0018
```

```
Running Simulations for PTE-2026-0018 (T1059)

  [1/4] T1059.001 PowerShell - encoded command execution
        Attack:    Executed
        Detected:  Yes (Splunk alert fired in 2.3s)
        Outcome:   PASS

  [2/4] T1059.003 Windows Command Shell - cmd.exe /c
        Attack:    Executed
        Detected:  No
        Outcome:   FAIL - Detection gap identified

  [3/4] T1059.004 Unix Shell - /bin/bash reverse shell
        Attack:    Executed
        Detected:  Yes (Wazuh rule 100210 triggered)
        Outcome:   PASS

  [4/4] T1059.006 Python - base64-encoded payload
        Attack:    Executed
        Detected:  Yes (CrowdStrike ML detection)
        Outcome:   PASS

Summary: 3/4 detected (75%)
Gap: T1059.003 requires new detection rule
```

### Step 5: Check ATT&CK Coverage

View your overall detection coverage across all 14 ATT&CK tactics.

```
/purpleteam coverage
```

```
MITRE ATT&CK Coverage Report

Tactic                    Techniques  Covered  Coverage
Reconnaissance            5           3        60%
Resource Development      4           2        50%
Initial Access            6           5        83%
Execution                 5           4        80%
Persistence               7           5        71%
Privilege Escalation      6           4        67%
Defense Evasion           8           5        63%
Credential Access         5           4        80%
Discovery                 6           5        83%
Lateral Movement          4           3        75%
Collection                5           3        60%
Command and Control       6           4        67%
Exfiltration              4           3        75%
Impact                    5           4        80%

Overall Coverage Score: 71% (54/76 techniques covered)

Top 3 Gaps:
  1. Defense Evasion (37% uncovered) - 3 techniques need rules
  2. Reconnaissance (40% uncovered) - 2 techniques need rules
  3. Resource Development (50% uncovered) - 2 techniques need rules
```

### Step 6: View in VibeUI

Open VibeUI and navigate to the **BlueTeam** panel. The panel has 7 tabs:

- **Incidents** -- List, create, and manage security incidents with status tracking
- **IOCs** -- Browse indicators of compromise with type filters
- **SIEM** -- Export queries to any of 8 supported platforms
- **Forensics** -- Manage forensic cases and evidence chains
- **Detection** -- Author and test detection rules
- **Playbooks** -- Define response playbooks with 8 action types
- **Threat Hunt** -- Run proactive threat hunting queries

Switch to the **PurpleTeam** panel with 5 tabs:

- **Exercises** -- Create and manage ATT&CK exercises
- **Simulations** -- Run attack simulations and view outcomes
- **Coverage** -- ATT&CK coverage heatmap across all tactics
- **Gaps** -- Prioritized list of detection gaps
- **Compare** -- Cross-exercise comparison to track improvement over time

## Demo Recording JSON

```json
{
  "meta": {
    "title": "Blue Team & Purple Team Security",
    "description": "Defensive security incident management and MITRE ATT&CK adversarial validation.",
    "duration_seconds": 300,
    "version": "0.5.1"
  },
  "steps": [
    {
      "id": 1,
      "action": "repl",
      "commands": [
        { "input": "/blueteam incident create \"Suspicious login from unknown IP\"", "delay_ms": 3000 },
        { "input": "/blueteam ioc add --incident INC-2026-0042 --type ip --value \"203.0.113.45\"", "delay_ms": 2000 },
        { "input": "/blueteam ioc add --incident INC-2026-0042 --type domain --value \"evil-c2.example.net\"", "delay_ms": 2000 },
        { "input": "/blueteam siem export splunk --incident INC-2026-0042", "delay_ms": 3000 },
        { "input": "/purpleteam exercise new \"MITRE ATT&CK T1059\"", "delay_ms": 3000 },
        { "input": "/purpleteam simulate PTE-2026-0018", "delay_ms": 5000 },
        { "input": "/purpleteam coverage", "delay_ms": 3000 },
        { "input": "/quit", "delay_ms": 500 }
      ],
      "description": "Full Blue Team and Purple Team workflow in the REPL"
    },
    {
      "id": 2,
      "action": "vibeui_interaction",
      "panel": "BlueTeam",
      "tab": "Incidents",
      "description": "View and manage security incidents"
    },
    {
      "id": 3,
      "action": "vibeui_interaction",
      "panel": "PurpleTeam",
      "tab": "Coverage",
      "description": "View ATT&CK coverage heatmap"
    }
  ]
}
```

## What's Next

- [Demo 24: Red Team Security](../24-red-team/) -- Offensive security scanning and vulnerability detection
- [Demo 35: Compliance & Audit](../35-compliance/) -- SOC 2 controls and audit trails
- [Demo 57: Internal Developer Platform](../57-idp/) -- Platform engineering with security baselines
