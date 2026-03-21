---
layout: page
title: "Demo 09 — Autofix & Diagnostics"
permalink: /demos/09-autofix/
---

# Demo 09 — Autofix & Diagnostics

## Overview

VibeCody's **Autofix** system detects and repairs code issues automatically. It integrates with LSP diagnostics (compiler errors, linter warnings, type errors) and uses AI to generate targeted fixes. You can fix a single diagnostic, batch-fix an entire file or project, or delegate to **Cloud Autofix agents** and **BugBot** for large-scale automated issue detection.

---

## Prerequisites

- VibeCody installed (`vibecli --version` returns 0.1+)
- An AI provider configured in `~/.vibecli/config.toml`
- A project with at least one language server available (e.g., `rust-analyzer` for Rust, `typescript-language-server` for TypeScript)
- For VibeUI: `cd vibeui && npm install && npm run tauri dev`

---

## Step-by-Step Walkthrough

### 1. View Diagnostics

Open a file with errors or warnings. VibeCody's LSP integration surfaces diagnostics in real time.

**VibeUI:**
- Error and warning counts appear in the status bar
- Squiggly underlines highlight issues in the editor
- Open the **Problems** panel (Cmd+Shift+M) for a full list

**CLI:**

```bash
# Start the REPL
vibecli

# List diagnostics for the current project
/autofix list
```

Example output:

```
Diagnostics (14 issues in 6 files):

  ERROR  src/auth.rs:23:5      — E0308: mismatched types, expected `String` found `&str`
  ERROR  src/auth.rs:45:12     — E0599: no method named `to_json` found for struct `User`
  WARN   src/handler.rs:10:1   — unused import: `std::collections::HashMap`
  WARN   src/handler.rs:34:9   — variable `result` is never read
  ERROR  src/db.rs:67:20       — E0277: trait bound `Row: Serialize` not satisfied
  WARN   src/main.rs:5:5       — unused variable: `config`
  ...
```

### 2. Fix a Single Diagnostic

**VibeUI:**
1. Hover over an underlined error
2. Click the lightbulb icon or press `Cmd+.`
3. Select **VibeCody: AI Fix** from the quick actions menu
4. Review the proposed change in the diff overlay
5. Click **Accept** or press `Cmd+Enter`

**CLI:**

```bash
# Fix a specific diagnostic by file and line
/autofix fix src/auth.rs:23

# Or use the interactive selector
/autofix fix --interactive
```

Example output:

```
Fixing: E0308 at src/auth.rs:23:5 — mismatched types

AI Analysis:
  The function signature expects `String` but a `&str` literal is passed.
  Adding `.to_string()` converts the reference to an owned String.

Proposed fix:
  - let name = "admin";
  + let name = "admin".to_string();

Apply fix? [y/n/e(dit)]: y
Fixed: src/auth.rs:23
```

### 3. Batch Fix Across Files

Fix all diagnostics in a file or the entire project at once.

**CLI:**

```bash
# Fix all issues in a single file
/autofix fix src/auth.rs --all

# Fix all warnings across the project
/autofix fix --all --severity warn

# Fix all errors across the project
/autofix fix --all --severity error

# Dry run — show fixes without applying
/autofix fix --all --dry-run
```

Example output:

```
Batch autofix (dry run):

  [1/14] src/auth.rs:23     — E0308: .to_string() conversion          FIXABLE
  [2/14] src/auth.rs:45     — E0599: add `use serde_json::ToJson`     FIXABLE
  [3/14] src/handler.rs:10  — unused import: remove line               FIXABLE
  [4/14] src/handler.rs:34  — unused variable: prefix with `_`        FIXABLE
  [5/14] src/db.rs:67       — missing trait: derive Serialize          FIXABLE
  ...

  Fixable: 12/14 | Needs review: 2/14
  Run without --dry-run to apply.
```

**VibeUI:**
1. Open the **Autofix** panel (AI Panel > Autofix tab)
2. Click **Scan Project**
3. Review the list of detected issues with proposed fixes
4. Click **Fix All** or select individual fixes to apply
5. A diff view shows all changes before confirmation

### 4. AI-Powered Fix Suggestions

For non-trivial issues where a simple mechanical fix is not sufficient, VibeCody uses AI to analyze the broader context and propose a semantic fix.

```bash
# Get an AI analysis of a complex error
/autofix analyze src/db.rs:67
```

Example output:

```
AI Analysis for E0277 at src/db.rs:67:

The `Row` struct needs to implement `Serialize` for the JSON API response.
However, it contains a `chrono::NaiveDateTime` field which requires the
`chrono` crate's `serde` feature.

Recommended fix (2 changes):
  1. Cargo.toml: Change `chrono = "0.4"` to `chrono = { version = "0.4", features = ["serde"] }`
  2. src/db.rs: Add `#[derive(Serialize)]` to `struct Row`

Apply multi-file fix? [y/n]:
```

### 5. Cloud Autofix Agents

For large projects, offload autofix to cloud agents that run in sandboxed containers.

**CLI:**

```bash
# Launch a cloud autofix agent
/autofix cloud --scope "src/**/*.rs" --max-fixes 50

# Check agent status
/autofix cloud status
```

Example output:

```
Cloud Autofix Agent [agent-af-8c3d]:
  Status:     Running
  Scope:      src/**/*.rs (87 files)
  Progress:   34/87 files scanned
  Fixes:      12 applied, 3 pending review
  Runtime:    2m 14s
  Container:  vibecody-sandbox-af-8c3d
```

**VibeUI:**
1. In the **Autofix** panel, click **Cloud Autofix**
2. Configure the scope and max fixes
3. Click **Launch Agent**
4. Monitor progress in real time; review fixes as they arrive

### 6. BugBot Automated Issue Detection

BugBot proactively scans your codebase for potential bugs, security issues, and anti-patterns beyond what the compiler or linter detects.

**CLI:**

```bash
# Run BugBot scan
/bugbot scan

# Scan with specific checks
/bugbot scan --checks "null-safety,sql-injection,error-handling"

# View BugBot findings
/bugbot findings
```

Example output:

```
BugBot Scan Results (3 findings):

  HIGH   src/db.rs:34      SQL Injection: User input interpolated into query string
         Suggestion: Use parameterized query with `sqlx::query!` macro

  MEDIUM src/auth.rs:78    Timing Attack: String comparison on password hash
         Suggestion: Use `constant_time_eq` from the `subtle` crate

  LOW    src/handler.rs:92 Unwrap on network result: `.unwrap()` on HTTP response
         Suggestion: Replace with `?` operator or `.expect()` with message

Auto-fix available for 3/3 findings. Run `/bugbot fix --all` to apply.
```

**VibeUI:**
1. Open the **BugBot** panel (AI Panel > BugBot tab)
2. Click **Scan** to start a proactive scan
3. Findings appear with severity badges and one-click fixes
4. Click **Fix** on individual findings or **Fix All**

---

## Demo Recording

```json
{
  "id": "demo-autofix",
  "title": "Autofix & Diagnostics",
  "description": "Demonstrates automated bug detection, LSP diagnostic integration, and AI-powered fix suggestions across single files and entire projects",
  "estimated_duration_s": 140,
  "steps": [
    {
      "action": "Navigate",
      "target": "vibeui://open?folder=/home/user/my-project"
    },
    {
      "action": "Narrate",
      "value": "We have a project with several compiler errors and warnings. Let's see how Autofix detects and repairs them."
    },
    {
      "action": "Click",
      "target": ".explorer-file[data-path='src/auth.rs']",
      "description": "Open auth.rs which has errors"
    },
    {
      "action": "Wait",
      "duration_ms": 1500
    },
    {
      "action": "Screenshot",
      "label": "file-with-errors"
    },
    {
      "action": "Assert",
      "target": ".status-bar .error-count",
      "value": "greater_than:0"
    },
    {
      "action": "Narrate",
      "value": "Red squiggles indicate compiler errors. Let's hover over one to see the diagnostic."
    },
    {
      "action": "Click",
      "target": ".editor-line:nth-child(23) .squiggly-error",
      "description": "Hover over the error on line 23"
    },
    {
      "action": "Wait",
      "duration_ms": 500
    },
    {
      "action": "Screenshot",
      "label": "error-hover-tooltip"
    },
    {
      "action": "Click",
      "target": ".lightbulb-icon",
      "description": "Click the lightbulb to see quick actions"
    },
    {
      "action": "Click",
      "target": ".quick-action[data-action='ai-fix']",
      "description": "Select AI Fix from quick actions"
    },
    {
      "action": "Wait",
      "duration_ms": 2000
    },
    {
      "action": "Screenshot",
      "label": "ai-fix-diff-overlay"
    },
    {
      "action": "Assert",
      "target": ".diff-overlay .added-line",
      "value": "contains:to_string"
    },
    {
      "action": "Type",
      "target": "keyboard",
      "value": "Cmd+Enter",
      "description": "Accept the fix"
    },
    {
      "action": "Narrate",
      "value": "Fix applied. Now let's batch-fix all remaining issues in the project."
    },
    {
      "action": "Click",
      "target": ".panel-tab[data-panel='autofix']",
      "description": "Open the Autofix panel"
    },
    {
      "action": "Click",
      "target": "#scan-project-btn",
      "description": "Scan the entire project"
    },
    {
      "action": "Wait",
      "duration_ms": 3000
    },
    {
      "action": "Screenshot",
      "label": "autofix-scan-results"
    },
    {
      "action": "Assert",
      "target": ".autofix-issue-list .issue",
      "value": "count_greater_than:5"
    },
    {
      "action": "Click",
      "target": "#fix-all-btn",
      "description": "Click Fix All to batch-apply fixes"
    },
    {
      "action": "Wait",
      "duration_ms": 4000
    },
    {
      "action": "Screenshot",
      "label": "batch-fix-diff-review"
    },
    {
      "action": "Click",
      "target": "#confirm-fixes-btn",
      "description": "Confirm all fixes after reviewing diffs"
    },
    {
      "action": "Narrate",
      "value": "All fixable issues resolved. Let's also run BugBot for proactive bug detection beyond compiler diagnostics."
    },
    {
      "action": "Click",
      "target": ".panel-tab[data-panel='bugbot']",
      "description": "Switch to BugBot panel"
    },
    {
      "action": "Click",
      "target": "#bugbot-scan-btn",
      "description": "Run BugBot scan"
    },
    {
      "action": "Wait",
      "duration_ms": 5000
    },
    {
      "action": "Screenshot",
      "label": "bugbot-findings"
    },
    {
      "action": "Assert",
      "target": ".bugbot-finding.severity-high",
      "value": "exists"
    },
    {
      "action": "Narrate",
      "value": "BugBot found a SQL injection vulnerability and a timing attack issue that no compiler or linter would catch. Each finding includes a one-click fix."
    },
    {
      "action": "Click",
      "target": ".bugbot-finding:first-child .fix-btn",
      "description": "Fix the highest-severity finding"
    },
    {
      "action": "Wait",
      "duration_ms": 2000
    },
    {
      "action": "Screenshot",
      "label": "bugbot-fix-applied"
    }
  ],
  "tags": ["autofix", "diagnostics", "lsp", "bugbot", "cloud-autofix", "batch-fix"]
}
```

---

## What's Next

- [Demo 10 — Code Transforms](../10-code-transforms/) — Structural refactoring with AST-based transforms
- [Demo 08 — Code Search & Embeddings](../08-code-search/) — Find code to fix with semantic search
- [Demo 11 — Docker & Container Management](../11-docker/) — Run autofix agents in sandboxed containers
