# Security Posture — UI contract

The `SecurityPosturePanel.tsx` component spec — states, keyboard
map, accessibility, and the visual language for the severity feed.

## Three-pane layout

```
┌──────────────────────────────────────────────────────────────────┐
│ Security Posture           [Rescan all] [Filters ▾] [Settings ⚙]│
├──────────────────────────────────────────────────────────────────┤
│ Feed (1/3 width)        │ Detail (2/3 width)                     │
│                          │                                        │
│ ● Critical (12)          │ ┌────────────────────────────────────┐│
│   ▸ AWS key in           │ │ CWE-798 — Hardcoded Credentials    ││
│     src/api/keys.rs:42   │ │                                    ││
│   ▸ command injection in │ │ src/api/keys.rs · line 42          ││
│     server/exec.ts:103   │ │ Scanner: secrets                   ││
│                          │ │                                    ││
│ ● High (34)              │ │ const KEY = "AKIA********";        ││
│   ▸ GPL dep in MIT proj. │ │ // ─ matched: AKIA-prefix          ││
│   ...                    │ │                                    ││
│                          │ │ Remediation:                        ││
│ ● Medium (89)            │ │ Move to env var, rotate the key   ││
│   ...                    │ │ via the affected provider.         ││
│                          │ │                                    ││
│ ● Low (147)              │ │ [Create work item] [Suppress…]    ││
│                          │ └────────────────────────────────────┘│
└──────────────────────────────────────────────────────────────────┘
```

## States

- **Idle / no scan yet** — show "Scan never run" banner, single
  "Run scan" button. Don't auto-scan if the user has explicitly
  navigated away — only auto-scan on workspace open.
- **Scanning** — header shows running scanner names ("scanning:
  vulnerability_db, sonar, secrets…"). Feed shows partial results
  as each scanner finishes (the aggregator streams via the
  existing event-bus pattern used by `chat_stream`).
- **Loaded** — three-pane layout above.
- **Empty (no findings)** — celebrate with "✓ No findings at
  current severity threshold". Hide the empty severity sections.
- **Stale** — if last scan was > 1 hour ago and the workspace has
  been edited since, show a yellow "Findings may be outdated"
  banner with a "Rescan" inline action.
- **Error (one scanner failed)** — don't block the rest. Show a
  per-scanner error chip at the top of the feed: "secrets scanner:
  could not parse ..." — link to scanner log.

## Filters

Default: show Critical + High + Medium, hide Low + Info, hide
Suppressed, hide Fixed.

Filterable axes:
- **Severity** (multi-select: Critical / High / Medium / Low / Info)
- **Category** (multi-select: every `Category` variant)
- **Scanner** (multi-select: every registered scanner name)
- **Status** (Open / Suppressed / GoalLinked / Fixed)
- **File path** (text input, substring match)

Filter state persists in `WorkspaceStore` (`posture:ui_filter:*`)
so it survives panel close / reopen.

## Detail pane

Selected finding shows:

1. Title bar — rule_id + CWE / OWASP if available + scanner badge
2. Location — `file:line:column` with click-to-open in editor
3. Severity chip (color-coded)
4. Snippet — fixed-width font, redacted form (secrets show
   `<prefix>***`, never the full value)
5. Remediation — multi-line markdown
6. References — clickable links to CWE / OWASP / advisory pages
7. Status row — current state, last-seen timestamp, first-seen timestamp
8. Actions row:
   - **Create work item** — opens a Goal-create modal pre-filled
     with finding metadata; on submit, calls
     `security_posture_create_goal(id)` and updates status to
     `GoalLinked { goal_id }`
   - **Suppress…** — opens a modal requiring a free-text reason;
     on submit, calls `security_posture_suppress(id, reason)` and
     updates status to `Suppressed { reason }`
   - **Open in editor** — primary action when location is known;
     uses the existing `commands::open_file` flow
   - **View audit log** — drawer showing every state-change for
     this finding-id (suppressions added/removed, goal linked, etc.)

## Keyboard map

- `↑` / `↓` — move feed selection
- `Enter` — open selected finding in editor
- `s` — open suppress modal for selected
- `g` — open goal-create modal for selected
- `/` — focus filter text input
- `r` — rescan all
- `?` — show this map

## Accessibility

- All severity colors paired with shape icons (✗ Critical / ▲ High /
  ● Medium / ◆ Low / · Info) so color-blind users get the same
  signal
- ARIA `role="listbox"` on the feed, `role="option"` on each row
- Live region announces scan start / per-scanner completion / total
  count when scan finishes
- Detail pane is a `region` with a labelled heading so screen
  readers can navigate directly to it

## Performance budget

- Initial render with cached findings (no scan): < 100 ms
- Filter change with 500 findings in feed: < 50 ms (virtualized
  list via the same `react-virtual` already used in `ChatComposite`)
- Detail-pane render on selection change: < 30 ms

## Lifecycle / triggers from the panel

| Trigger | Action |
|---|---|
| Panel mounted | Load cached findings via `security_posture_findings` |
| Workspace switched | Discard old findings, load new from cache, no auto-scan |
| "Rescan all" clicked | Fire `security_posture_scan`, stream results |
| File saved in editor (panel open) | Single-file fast-path scan, merge into feed |
| Panel unmounted | No action (keep cache warm) |

## Slot in the composite

Registered in `EnterpriseGovernanceComposite.tsx` between MCP
Governance and Plugin Governance:

```ts
{ id: "mcp-governance", label: "MCP Governance", ... },
{ id: "security-posture", label: "Security Posture", importFn: () => import("../SecurityPosturePanel"), exportName: "SecurityPosturePanel" },
{ id: "plugin-governance", label: "Plugin Governance", ... },
```

Rationale: all three are "what's running and is it safe" surfaces.
A user looking at MCP Governance is already in the security mindset.
