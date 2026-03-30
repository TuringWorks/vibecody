---
layout: page
title: "Demo 58: Living Documentation Sync"
permalink: /demos/58-doc-sync/
nav_order: 58
parent: Demos
---


## Overview

VibeCody's DocSync module keeps your documentation in lockstep with your code. It detects drift between specs, READMEs, API docs, and the actual implementation, assigns a freshness score to each document, and can automatically reconcile stale sections. DocSync works bidirectionally -- changes in code update docs, and changes in docs can flag code that needs updating. It integrates with the agent loop to propose and apply fixes, ensuring your documentation stays accurate without manual effort.

**Time to complete:** ~10 minutes

## Prerequisites

- VibeCLI v0.5.1 installed and on your PATH
- At least one AI provider configured
- A project with documentation files (Markdown, RST, or inline doc comments)
- (Optional) VibeUI running with the **DocSync** panel visible

## Step-by-Step Walkthrough

### Step 1: Check Documentation Status

Open the VibeCLI REPL and run a documentation status scan.

```bash
vibecli
```

```
/docsync status
```

Expected output:

```
Documentation Sync Status

  Scanned: 14 documents, 47 code files
  Engine:  AST + content hashing

  Document                      Freshness  Status    Drift Points
  README.md                     92%        Fresh     1 minor
  docs/api-reference.md         67%        Stale     4 endpoints changed
  docs/architecture.md          88%        Fresh     1 diagram outdated
  docs/getting-started.md       95%        Fresh     0
  docs/configuration.md         43%        Stale     7 options undocumented
  src/lib.rs (module docs)      78%        Warning   3 functions changed
  src/config.rs (inline docs)   55%        Stale     5 structs modified
  CHANGELOG.md                  100%       Fresh     0
  CONTRIBUTING.md               98%        Fresh     0
  docs/deployment.md            71%        Warning   2 steps outdated
  docs/testing.md               84%        Fresh     1 new test pattern
  docs/security.md              90%        Fresh     0
  docs/providers.md             61%        Stale     3 new providers
  docs/cli-reference.md         39%        Stale     9 flags added

  Overall Freshness: 73%
  Documents needing attention: 5 (stale or warning)

  Run /docsync reconcile to fix stale documents.
```

### Step 2: Inspect Specific Drift

Drill into a specific document to see exactly what has drifted.

```
/docsync status docs/api-reference.md
```

```
Drift Report: docs/api-reference.md
  Freshness: 67%
  Last updated: 2026-03-15
  Code changes since: 12 commits

  Drift Points:

  1. [Line 45] POST /api/chat
     Doc says: accepts "message" (string)
     Code has: accepts "message" (string), "stream" (bool), "model" (string)
     Missing:  "stream" and "model" parameters

  2. [Line 78] GET /api/sessions
     Doc says: returns array of session IDs
     Code has: returns array of session objects with id, title, created_at
     Stale:    response schema changed

  3. [Line 102] DELETE /api/sessions/:id
     Doc says: (not documented)
     Code has: endpoint exists in src/serve.rs:341
     Missing:  entire endpoint undocumented

  4. [Line 130] POST /api/agent
     Doc says: accepts "task" (string)
     Code has: accepts "task" (string), "tools" (array), "max_turns" (int)
     Missing:  "tools" and "max_turns" parameters

  Suggested fix: /docsync reconcile docs/api-reference.md
```

### Step 3: Watch for Changes

Start a file watcher that detects drift in real time as you edit code.

```
/docsync watch
```

```
DocSync Watcher Started
  Watching: 14 documents, 47 code files
  Mode:     Notify on drift (use --auto-fix for automatic reconciliation)

  [10:32:15] src/serve.rs modified
             -> docs/api-reference.md freshness dropped to 62%
             -> 1 new drift point: response type changed on GET /health

  [10:33:42] src/config.rs modified
             -> docs/configuration.md freshness dropped to 38%
             -> 2 new drift points: new config keys added

  Press Ctrl+C to stop watching.
```

### Step 4: Reconcile Stale Documents

Let VibeCody's AI update stale documents to match the current code.

```
/docsync reconcile
```

```
Reconciling 5 stale documents...

  [1/5] docs/api-reference.md
        Fixed 4 drift points:
          + Added "stream" and "model" params to POST /api/chat
          + Updated GET /api/sessions response schema
          + Added DELETE /api/sessions/:id documentation
          + Added "tools" and "max_turns" params to POST /api/agent
        Freshness: 67% -> 100%

  [2/5] docs/configuration.md
        Fixed 7 drift points:
          + Documented 7 new config keys in [agent] section
        Freshness: 43% -> 100%

  [3/5] src/config.rs (inline docs)
        Fixed 5 drift points:
          + Updated doc comments on 3 structs
          + Added doc comments to 2 new fields
        Freshness: 55% -> 100%

  [4/5] docs/providers.md
        Fixed 3 drift points:
          + Added Gemini, DeepSeek, Cerebras provider sections
        Freshness: 61% -> 100%

  [5/5] docs/cli-reference.md
        Fixed 9 drift points:
          + Added 9 new CLI flags to reference table
        Freshness: 39% -> 100%

Summary:
  Documents reconciled: 5
  Drift points fixed:   28
  Overall freshness:    73% -> 98%

Review changes with: git diff docs/
```

You can also reconcile a single document:

```
/docsync reconcile docs/api-reference.md
```

### Step 5: Reconcile Code from Docs

DocSync works bidirectionally. If a spec document was updated but the code has not caught up, you can flag those gaps.

```
/docsync reconcile --direction doc-to-code
```

```
Doc-to-Code Drift

  Spec: docs/api-spec.md
    [Line 22] Spec requires: POST /api/batch endpoint
    Code:     Not implemented
    Action:   Generate stub? [y/n]

  Spec: docs/api-spec.md
    [Line 45] Spec requires: rate limiting on all endpoints (100 req/min)
    Code:     Rate limiting only on /api/chat
    Action:   Extend rate limiting? [y/n]

  2 spec-to-code gaps found.
```

### Step 6: View in VibeUI

Open VibeUI and navigate to the **DocSync** panel. The panel provides:

- **Overview** -- Freshness gauge and document status table
- **Drift Map** -- Visual graph showing which docs link to which code files, with drift highlighted in red
- **Reconcile** -- One-click reconciliation with diff preview before applying
- **Watch Log** -- Real-time feed of file changes and freshness updates
- **Settings** -- Configure watched paths, ignore patterns, and auto-fix preferences

## Demo Recording JSON

```json
{
  "meta": {
    "title": "Living Documentation Sync",
    "description": "Bidirectional spec-code sync with drift detection and AI reconciliation.",
    "duration_seconds": 240,
    "version": "0.5.1"
  },
  "steps": [
    {
      "id": 1,
      "action": "repl",
      "commands": [
        { "input": "/docsync status", "delay_ms": 4000 },
        { "input": "/docsync status docs/api-reference.md", "delay_ms": 3000 },
        { "input": "/docsync watch", "delay_ms": 8000 },
        { "input": "/docsync reconcile", "delay_ms": 6000 },
        { "input": "/quit", "delay_ms": 500 }
      ],
      "description": "Full DocSync workflow: status, inspect, watch, reconcile"
    },
    {
      "id": 2,
      "action": "vibeui_interaction",
      "panel": "DocSync",
      "tab": "Drift Map",
      "description": "View doc-to-code dependency graph with drift highlights"
    }
  ]
}
```

## What's Next

- [Demo 09: Autofix & Diagnostics](../09-autofix/) -- Automated bug detection and repair
- [Demo 10: Code Transforms](../10-code-transforms/) -- AST-based refactoring
- [Demo 35: Compliance & Audit](../35-compliance/) -- Audit trails for documentation changes
