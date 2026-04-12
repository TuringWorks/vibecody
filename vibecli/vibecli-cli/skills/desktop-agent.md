---
triggers: ["desktop automation", "computer use", "click automation", "browser debugging", "desktop testing"]
tools_allowed: ["read_file", "write_file", "bash"]
category: agent
---

# Desktop Computer Use Automation

When automating desktop applications and browser interactions with an agent:

1. **Platform-Specific Tool Selection** — Linux: prefer AT-SPI2 (accessibility tree) for reliable element discovery; use xdotool only as fallback for raw input simulation when AT-SPI is unavailable. macOS: use AppleScript/Accessibility APIs for native apps; use CDP (Chrome DevTools Protocol) for browser targets. Windows: use UI Automation (UIA) framework first, Win32 SendMessage as last resort. Never default to pixel-based clicking unless no structured API exists.
2. **Element Discovery Strategies** — Always prefer semantic identifiers in this priority order: (1) accessibility role + name, (2) test-id attribute, (3) stable CSS selector, (4) XPath, (5) text content match, (6) visual bounding box. Avoid positional coordinates in automation scripts — they break on resolution or locale changes. Cache discovered element references within a single task session but re-query on navigation events.
3. **Dry-Run Mode Safety** — Implement a dry-run flag that logs every intended action (click target, text to type, key sequence) without executing it. Always default to dry-run=true for the first invocation on any new machine or application context. Require explicit opt-in to live execution. Print a human-readable preview of the action sequence before live runs.
4. **Session Recording Best Practices** — Record all desktop sessions to a compressed video or action log (element ID + action + timestamp). Store recordings for the duration of the task plus a configurable retention window (default 7 days). Recordings are essential for debugging flaky automation; include them in failure reports. Never record screens containing credential entry fields — pause recording and mask the segment.
5. **CDP Browser Automation vs Native** — Use CDP for: headless test runs, JavaScript injection, network interception, performance profiling, and cross-browser portability. Use native OS automation for: system dialogs (file pickers, auth prompts), non-Chromium browsers, and actions that require OS-level permissions. Do not mix CDP and native input events in the same interaction sequence; they can conflict on focus handling.
6. **Error Recovery and Retry** — When an element is not found, wait up to 3 seconds with 500ms polling before declaring failure. On stale element references, re-query and retry the action once. After 3 consecutive failures on the same element, take a screenshot, log the DOM/AT-SPI tree snapshot, and surface the failure to the user rather than looping indefinitely.
7. **Flakiness Mitigation** — Introduce explicit waits for element visibility and interactability rather than fixed `sleep` delays. Set a global implicit wait (1s) and action timeout (10s). For timing-sensitive UI (animations, lazy-loaded content), listen for network-idle or DOMContentLoaded events before proceeding. Track per-element flakiness rates and flag elements that fail >10% of attempts for manual review.
8. **Permission and Accessibility Setup** — Document and automate the required OS permission grants (macOS: Accessibility, Screen Recording; Linux: AT-SPI bridge enabled; Windows: UIAccess). Verify permissions at agent startup and surface a clear error with remediation steps if any are missing. Never silently skip automation steps due to missing permissions.
