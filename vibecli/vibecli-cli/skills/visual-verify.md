# Visual Verification

Screenshot-based UI verification that compares actual rendered output against expected baselines. Detects visual regressions, layout shifts, and rendering bugs by analyzing screenshots with pixel diffing and AI vision.

## When to Use
- Catching visual regressions after CSS or component changes
- Verifying UI matches design mockups or Figma exports
- Detecting layout shifts, overflow issues, and responsive breakpoints
- Validating dark mode, theme changes, and accessibility contrast
- Running visual smoke tests as part of CI/CD pipelines

## Commands
- `/visual capture <url-or-component>` — Take a screenshot for comparison
- `/visual baseline <name>` — Save current screenshot as the baseline
- `/visual compare <name>` — Compare current state against a named baseline
- `/visual diff <a> <b>` — Show pixel-level diff between two screenshots
- `/visual report` — Generate a visual regression report for all baselines
- `/visual threshold <percent>` — Set acceptable diff threshold (default: 0.1%)
- `/visual responsive <url> <widths>` — Capture at multiple viewport widths
- `/visual approve <name>` — Approve current state as new baseline

## Examples
```
/visual capture http://localhost:3000/dashboard
# Captured: dashboard-2026-03-26-14:32.png (1920x1080)

/visual compare dashboard
# Diff: 2.3% pixels changed (threshold: 0.1%) FAIL
# Regions: header logo shifted 4px left, sidebar width +12px
# AI analysis: Likely caused by flexbox gap change in Layout.tsx

/visual responsive http://localhost:3000 "375,768,1024,1440"
# Captured 4 viewports. Issues found at 375px: text overflow in nav
```

## Best Practices
- Set baselines after design-approved states, not arbitrary checkpoints
- Use AI vision analysis for semantic understanding beyond pixel diffs
- Test at multiple viewport widths to catch responsive layout issues
- Keep threshold low (0.1%) for pixel-perfect components, higher for dynamic content
- Integrate visual checks into PR review for automatic regression detection
