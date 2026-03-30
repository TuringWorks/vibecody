---
layout: page
title: "Demo 59: Agentic Package Manager"
permalink: /demos/59-smart-deps/
nav_order: 59
parent: Demos
---


## Overview

VibeCody's SmartDeps module acts as an AI-powered package manager layer that works across Cargo, npm, pip, Go modules, and Maven. It resolves dependency conflicts with AI-suggested strategies, compares packages head-to-head on metrics like bundle size, maintenance activity, and license compatibility, auto-patches CVEs by upgrading to the nearest safe version, runs license compliance audits, and generates interactive dependency graphs. SmartDeps sits on top of your native package manager -- it does not replace cargo or npm, it makes them smarter.

**Time to complete:** ~10 minutes

## Prerequisites

- VibeCLI v0.5.1 installed and on your PATH
- At least one AI provider configured
- A project with a package manifest (Cargo.toml, package.json, requirements.txt, go.mod, or pom.xml)
- (Optional) VibeUI running with the **SmartDeps** panel visible

## Step-by-Step Walkthrough

### Step 1: Resolve Dependency Conflicts

When your dependency tree has version conflicts, SmartDeps analyzes the constraints and suggests resolution strategies.

```bash
vibecli
```

```
/deps resolve
```

Expected output:

```
Dependency Resolution Analysis

  Manifest:  Cargo.toml (42 dependencies)
  Lockfile:  Cargo.lock (187 crates)

  Conflicts Found: 2

  1. tokio version conflict
     Direct:     tokio = "1.36"
     Via axum:   tokio >= "1.35, < 1.38"
     Via tonic:  tokio >= "1.32, < 1.37"
     Overlap:    1.36.x satisfies all constraints
     Status:     Auto-resolved (current version works)

  2. serde_json version conflict
     Direct:     serde_json = "1.0.114"
     Via reqwest: serde_json >= "1.0.100"
     Via config:  serde_json = "1.0.108" (pinned)
     Overlap:    None (config pins 1.0.108, you require 1.0.114)
     Suggestion: Update config to v0.14.1 which accepts serde_json >= 1.0.110

  Resolution Plan:
    1. Update config 0.14.0 -> 0.14.1
    2. Run cargo update -p serde_json

  Apply? [y/n]:
```

### Step 2: Compare Packages

Compare two packages side-by-side to make an informed decision.

```
/deps compare lodash ramda
```

```
Package Comparison: lodash vs ramda

  Metric               lodash           ramda
  ─────────────────────────────────────────────
  Version              4.17.21          0.30.1
  Weekly downloads     45.2M            4.8M
  Bundle size (min)    71.5 KB          48.2 KB
  Bundle size (gzip)   25.2 KB          11.8 KB
  Tree-shakeable       No (CJS)         Yes (ESM)
  TypeScript types     @types/lodash    Built-in
  Last publish         2021-02-20       2024-01-15
  Open issues          298              85
  License              MIT              MIT
  Dependencies         0                0
  Maintainers          3                5
  GitHub stars         58.7K            23.5K

  AI Analysis:
  - lodash has wider adoption but is no longer actively maintained.
  - ramda is smaller, tree-shakeable, and actively maintained.
  - For new projects, ramda or lodash-es (tree-shakeable fork) is
    recommended for smaller bundle sizes.
  - lodash remains a safe choice for existing projects due to stability.

  Recommendation: ramda (for new projects), lodash-es (for migration)
```

### Step 3: Auto-Patch CVEs

Scan your dependencies for known vulnerabilities and apply the nearest safe upgrade.

```
/deps patch --cve
```

```
CVE Scan & Patch

  Scanning 187 crates against RustSec advisory database...

  Vulnerabilities Found: 3

  1. RUSTSEC-2026-0012  HIGH
     Crate:     h2 0.3.24
     Advisory:  HTTP/2 CONTINUATION flood denial of service
     Fix:       Upgrade to h2 >= 0.3.26
     Patch:     h2 0.3.24 -> 0.3.26 (compatible)
     Status:    Auto-patchable

  2. RUSTSEC-2026-0008  MEDIUM
     Crate:     rustls 0.21.9
     Advisory:  Certificate validation bypass in certain configurations
     Fix:       Upgrade to rustls >= 0.21.11
     Patch:     rustls 0.21.9 -> 0.21.11 (compatible)
     Status:    Auto-patchable

  3. RUSTSEC-2025-0045  LOW
     Crate:     regex 1.10.2
     Advisory:  ReDoS in certain patterns with large repetitions
     Fix:       Upgrade to regex >= 1.10.4
     Patch:     regex 1.10.2 -> 1.10.4 (compatible)
     Status:    Auto-patchable

  All 3 vulnerabilities are auto-patchable.

  Applying patches...
    [1/3] h2 0.3.24 -> 0.3.26        Done
    [2/3] rustls 0.21.9 -> 0.21.11   Done
    [3/3] regex 1.10.2 -> 1.10.4     Done

  Running cargo check... OK (0 errors, 0 warnings)

  3 CVEs patched. Cargo.lock updated.
  Review: git diff Cargo.lock
```

### Step 4: Run a License Audit

Check all dependencies for license compatibility with your project.

```
/deps audit
```

```
License Compliance Audit

  Project license: MIT
  Dependencies scanned: 187

  License Distribution:
    MIT                 124  (66%)
    Apache-2.0           38  (20%)
    MIT OR Apache-2.0    18  (10%)
    BSD-2-Clause          4  (2%)
    BSD-3-Clause          2  (1%)
    ISC                   1  (1%)

  Compatibility Check:
    [PASS] All licenses are compatible with MIT
    [PASS] No copyleft licenses detected (GPL, AGPL, LGPL)
    [PASS] No unknown or custom licenses

  SBOM Export:
    /deps audit --export sbom.json    (SPDX format)
    /deps audit --export sbom.cdx     (CycloneDX format)

  Status: All clear. No license compliance issues.
```

### Step 5: Generate a Dependency Graph

Visualize your dependency tree as an ASCII graph or export to DOT format.

```
/deps graph
```

```
Dependency Graph (top-level only, 42 crates)

  vibecli v0.5.1
  ├── tokio v1.36.0
  │   ├── pin-project-lite v0.2.13
  │   ├── bytes v1.5.0
  │   └── mio v0.8.10
  ├── axum v0.7.4
  │   ├── tokio v1.36.0 (*)
  │   ├── hyper v1.2.0
  │   │   ├── h2 v0.3.26
  │   │   └── http v1.0.0
  │   └── tower v0.4.13
  ├── serde v1.0.197
  │   └── serde_derive v1.0.197
  ├── serde_json v1.0.114
  │   └── serde v1.0.197 (*)
  ├── clap v4.5.1
  │   ├── clap_builder v4.5.1
  │   └── clap_derive v4.5.0
  └── ... (37 more)

  (*) = deduplicated (shared dependency)

  Total crates: 187 (42 direct, 145 transitive)
  Max depth: 8

  Export: /deps graph --format dot > deps.dot
```

### Step 6: View in VibeUI

Open VibeUI and navigate to the **SmartDeps** panel. The panel provides:

- **Overview** -- Dependency count, outdated packages, CVE summary
- **Resolve** -- Interactive conflict resolution with diff preview
- **Compare** -- Side-by-side package comparison with charts
- **Security** -- CVE scan results with one-click patching
- **License** -- License distribution pie chart and compliance status
- **Graph** -- Interactive dependency graph with zoom and search

## Demo Recording JSON

```json
{
  "meta": {
    "title": "Agentic Package Manager",
    "description": "AI-powered dependency resolution, CVE patching, license auditing, and package comparison.",
    "duration_seconds": 240,
    "version": "0.5.1"
  },
  "steps": [
    {
      "id": 1,
      "action": "repl",
      "commands": [
        { "input": "/deps resolve", "delay_ms": 5000 },
        { "input": "/deps compare lodash ramda", "delay_ms": 4000 },
        { "input": "/deps patch --cve", "delay_ms": 6000 },
        { "input": "/deps audit", "delay_ms": 4000 },
        { "input": "/deps graph", "delay_ms": 3000 },
        { "input": "/quit", "delay_ms": 500 }
      ],
      "description": "Full SmartDeps workflow: resolve, compare, patch, audit, graph"
    },
    {
      "id": 2,
      "action": "vibeui_interaction",
      "panel": "SmartDeps",
      "tab": "Security",
      "description": "View CVE scan results and apply patches"
    },
    {
      "id": 3,
      "action": "vibeui_interaction",
      "panel": "SmartDeps",
      "tab": "Graph",
      "description": "Explore interactive dependency graph"
    }
  ]
}
```

## What's Next

- [Demo 11: Docker & Container Management](../11-docker/) -- Build and manage containers
- [Demo 35: Compliance & Audit](../35-compliance/) -- License compliance in your audit trail
- [Demo 09: Autofix & Diagnostics](../09-autofix/) -- Automated vulnerability remediation
