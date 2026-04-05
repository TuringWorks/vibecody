---
layout: page
title: "Demo 44: Visual Verification"
permalink: /demos/44-visual-verify/
---


## Overview

Code changes can silently break visual layouts. A CSS tweak that fixes one component can misalign another. VibeCody's visual verification system captures screenshots, compares them against baselines pixel-by-pixel, and scores design compliance. This demo shows how to capture screenshots, establish baselines, run visual diffs, and integrate checks into your development workflow.

**Time to complete:** ~8 minutes

## Prerequisites

- VibeCLI 0.5.1 installed and on your PATH
- A running web application (e.g., `http://localhost:3000`)
- Chromium-based browser installed (Chrome, Chromium, or Edge) for headless screenshots
- For VibeUI: the desktop app running with the **VisualVerifyPanel** visible

## Step-by-Step Walkthrough

### Step 1: Capture a screenshot

Start VibeCLI and capture a screenshot of your running application:

```bash
vibecli
```

```
> /vverify screenshot http://localhost:3000
```

```
[VisualVerify] Launching headless browser...
[VisualVerify] Navigating to http://localhost:3000
[VisualVerify] Viewport: 1920×1080
[VisualVerify] Screenshot captured: .vibecli/vverify/screenshots/localhost-3000-2026-03-29T14-30-00.png
[VisualVerify] File size: 284 KB
[VisualVerify] Dimensions: 1920×1080 px

Screenshot saved. Use `/vverify baseline save` to set as baseline.
```

Capture specific pages or viewports:

```
> /vverify screenshot http://localhost:3000/login --viewport 375x812 --name "login-mobile"
```

```
[VisualVerify] Viewport: 375×812 (mobile)
[VisualVerify] Screenshot captured: .vibecli/vverify/screenshots/login-mobile-2026-03-29T14-30-15.png
[VisualVerify] File size: 142 KB
[VisualVerify] Dimensions: 375×812 px
```

### Step 2: Save a baseline

Establish the current screenshot as the known-good baseline:

```
> /vverify baseline save
```

```
[VisualVerify] Saving baselines...

Baselines saved:
  ✓ localhost-3000        → .vibecli/vverify/baselines/localhost-3000.png
  ✓ login-mobile          → .vibecli/vverify/baselines/login-mobile.png

Total baselines: 2
Use `/vverify diff --baseline` after making changes to compare.
```

### Step 3: Make a change and run a visual diff

After modifying your CSS or components, capture a new screenshot and compare:

```
> /vverify screenshot http://localhost:3000
> /vverify diff --baseline
```

```
[VisualVerify] Comparing against baseline: localhost-3000
[VisualVerify] Pixel comparison in progress...

Visual Diff Report — localhost-3000
═══════════════════════════════════

Baseline:  .vibecli/vverify/baselines/localhost-3000.png
Current:   .vibecli/vverify/screenshots/localhost-3000-2026-03-29T14-35-22.png

Total pixels:     2,073,600
Changed pixels:   12,847 (0.62%)
Structural diff:  3 regions detected

Region 1: Header (y: 0-64)
  ├── Changed pixels: 8,201
  ├── Change type: Color shift (background #1a1a2e → #16213e)
  └── Severity: Low (intentional theme change?)

Region 2: Navigation bar (y: 64-120, x: 0-250)
  ├── Changed pixels: 3,412
  ├── Change type: Layout shift (8px rightward)
  └── Severity: Medium (possible unintended side effect)

Region 3: Footer (y: 1020-1080)
  ├── Changed pixels: 1,234
  ├── Change type: Text content change
  └── Severity: Low (copyright year updated)

Diff image saved: .vibecli/vverify/diffs/localhost-3000-diff-2026-03-29T14-35-22.png
  (Changed pixels highlighted in red overlay)

Overall similarity: 99.38%
Design compliance score: 94/100
  Deductions:
    -4  Layout shift in navigation (Region 2)
    -2  Unreviewed color changes (Region 1)
```

### Step 4: Compare specific pages

Run a diff on the mobile login page:

```
> /vverify screenshot http://localhost:3000/login --viewport 375x812 --name "login-mobile"
> /vverify diff --baseline --name "login-mobile"
```

```
[VisualVerify] Comparing against baseline: login-mobile

Visual Diff Report — login-mobile
══════════════════════════════════

Total pixels:     304,200
Changed pixels:   0 (0.00%)

✓ Perfect match — no visual changes detected.
Design compliance score: 100/100
```

### Step 5: Run multi-page verification

Verify all saved baselines at once:

```
> /vverify diff --all
```

```
[VisualVerify] Running full baseline comparison (2 pages)...

Page                │ Changed │ Similarity │ Score  │ Status
────────────────────┼─────────┼────────────┼────────┼────────
localhost-3000      │ 0.62%   │ 99.38%     │ 94/100 │ ⚠ Review
login-mobile        │ 0.00%   │ 100.00%    │ 100    │ ✓ Pass

Overall compliance: 97/100
Pages passing: 1/2
Action needed: Review localhost-3000 (navigation layout shift)
```

### Step 6: Set compliance thresholds

Configure minimum acceptable scores:

```
> /vverify config threshold 95
```

```
Compliance threshold updated: 90 → 95
Pages scoring below 95/100 will be flagged as failures.

Current status with new threshold:
  localhost-3000:  94/100 — FAIL (below 95)
  login-mobile:   100/100 — PASS
```

### Step 7: Accept intentional changes

When a visual change is intentional, update the baseline:

```
> /vverify baseline update localhost-3000
```

```
[VisualVerify] Updating baseline for localhost-3000...
  Old baseline: 2026-03-29T14:30:00 (archived)
  New baseline: 2026-03-29T14:35:22

Baseline updated. Previous baseline archived at:
  .vibecli/vverify/baselines/archive/localhost-3000-2026-03-29T14-30-00.png
```

### Step 8: Use visual verification in VibeUI

In the VibeUI desktop app, open the **VisualVerifyPanel** from the AI sidebar. The panel provides:

- **Side-by-Side View** -- Baseline and current screenshot with synchronized zoom and pan
- **Overlay Mode** -- Red pixel overlay highlighting exact differences
- **Slider Mode** -- Drag a vertical slider to reveal baseline vs current
- **Compliance Dashboard** -- Scores for all tracked pages with trend history
- **Baseline Manager** -- Save, update, archive, and restore baselines

## Demo Recording

```json
{
  "meta": {
    "title": "Visual Verification",
    "description": "Capture screenshots, compare against baselines, and score design compliance.",
    "duration_seconds": 160,
    "version": "0.5.1"
  },
  "steps": [
    {
      "id": 1,
      "action": "repl",
      "commands": [
        { "input": "/vverify screenshot http://localhost:3000", "delay_ms": 5000 },
        { "input": "/vverify baseline save", "delay_ms": 2000 },
        { "input": "/vverify screenshot http://localhost:3000/login --viewport 375x812 --name \"login-mobile\"", "delay_ms": 5000 },
        { "input": "/vverify baseline save", "delay_ms": 2000 },
        { "input": "/quit", "delay_ms": 500 }
      ],
      "description": "Capture initial screenshots and save baselines"
    },
    {
      "id": 2,
      "action": "shell",
      "command": "echo 'Make a CSS change to your application, then continue'",
      "description": "Pause for user to make changes",
      "delay_ms": 5000
    },
    {
      "id": 3,
      "action": "repl",
      "commands": [
        { "input": "/vverify screenshot http://localhost:3000", "delay_ms": 5000 },
        { "input": "/vverify diff --baseline", "delay_ms": 3000 },
        { "input": "/vverify diff --all", "delay_ms": 5000 },
        { "input": "/quit", "delay_ms": 500 }
      ],
      "description": "Capture new screenshots and run visual diff"
    }
  ]
}
```

## What's Next

- [Demo 45: Offline Voice Coding](../45-offline-voice/) -- Code with voice commands without internet
- [Demo 42: MCTS Code Repair](../42-mcts-repair/) -- Fix bugs with tree-search exploration
- [Demo 9: Autofix](../09-autofix/) -- Automated lint and test failure fixes
