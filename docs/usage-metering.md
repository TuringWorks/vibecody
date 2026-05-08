---
layout: page
title: Usage Metering
permalink: /usage-metering/
---

# Usage Metering

The Usage Metering panel tracks AI spend across providers and models, lets you define budgets, and surfaces alerts when consumption crosses thresholds. It's the operator dashboard for "where is the money going" — Counsel-style panels and agent-loop runs are typically the dominant cost centers.

This page covers the desktop panel. Mobile/watch/IDE clients don't have a metering surface today; usage data is daemon-side and observable only via the desktop UI or by reading `~/.vibeui/usage-metering.json` directly.

---

## Four tabs

| Tab | What it shows |
|---|---|
| **Dashboard** | Total spend / tokens / requests across all time + spend-by-provider bar chart |
| **Budgets** | Configured budgets (daily / weekly / monthly) with progress bars and a creation form |
| **Reports** | Per-provider and per-model breakdown (cost, tokens, requests) |
| **Alerts** | Triggered alerts (budget exceeded, anomalous spend) with severity badges and dismiss |

---

## Dashboard

Three KPIs at the top:

- **Total Spend** — running USD total across every recorded request.
- **Tokens Used** — `M` for ≥1M, `k` for ≥1k, raw count otherwise.
- **Requests** — count of LLM calls.

Below: a horizontal bar chart of spend per provider, with each bar's width proportional to the provider's share of total spend. Empty state: "No provider data yet."

The dashboard is read-only — usage is recorded automatically as the daemon proxies LLM requests. There's no manual "log this request" affordance because that would defeat the audit-trail purpose.

---

## Budgets

A **budget** is `(name, limit, period)`. The panel renders one card per budget with:

- A progress bar (`role="progressbar"` with `aria-valuenow`) that turns amber at 70% and red at 90%.
- `$X used / $Y limit (Z%)` summary line.
- Remove button (with two-click intent — but currently a single click; see roadmap).

### Create

The Create Budget form takes:

| Field | Notes |
|---|---|
| Name | Free-form label, e.g. "Marketing campaign Q2" |
| Limit ($) | Numeric, parsed as float |
| Period | `daily` / `weekly` / `monthly` |

Submitting calls `create_usage_budget` and persists to `~/.vibeui/usage-metering.json`. The new budget appears immediately above the form.

### Delete (was broken — fixed)

**Bug fix:** until this release, "Remove" only mutated local React state. After a page reload the deleted budget came back. The panel now calls a real `delete_usage_budget` Tauri command that updates the on-disk record.

If you have a workspace with "ghost budgets" that keep returning, edit `~/.vibeui/usage-metering.json` directly to remove them — the new delete path won't fix entries that pre-existed the bug.

### Period semantics

The `period` field controls how `used` resets. The reset logic is daemon-side (a per-budget cron-like reset at the period boundary) — the UI only displays the current `used` value. If a budget shows usage that should have rolled over, check `RUST_LOG=vibecody::usage=info` for the reset event.

---

## Reports

Two views — by provider and by model. Each row: label, tokens (formatted), request count, cost. Sortable by cost (default) — descending.

If both views are empty, no LLM calls have been recorded yet. This is the most common state on a fresh install before any provider is wired up.

---

## Alerts

Alerts appear when:

| Severity | Trigger |
|---|---|
| `info` | Budget at 50% |
| `warning` | Budget at 80% |
| `critical` | Budget at 100%, or anomalous spend (>3× rolling 7-day average) |

Each alert card shows the severity badge, the message, and a timestamp. Dismiss removes the alert from the active list (kept in the file with `dismissed: true` for audit).

---

## /health declaration

`features.usage_metering`:

```json
{
  "available": true,
  "transport": "tauri-desktop",
  "store_path": "~/.vibeui/usage-metering.json",
  "budget_periods": ["daily", "weekly", "monthly"]
}
```

Usage metering has no provider dependency — it observes the daemon's own request log. `available` is always true.

---

## Observability

Backend operations emit structured tracing events under `vibecody::usage`:

```bash
RUST_LOG=vibecody::usage=info vibecli serve
```

Events:

```
INFO vibecody::usage: usage.budget.create
  id=bg1714900000123 name="Marketing Q2" limit=500.0 period=monthly total_budgets=4

INFO vibecody::usage: usage.budget.delete
  id=bg1714900000123 remaining=3

INFO vibecody::usage: usage.alert.dismiss id=al17149...
```

**Budget names are logged** (they're already operator-facing labels), but per-request prompt content, response tokens, and cost details are NOT. Cost aggregation events live under `vibecody::cost` (a separate target, not in this list yet).

---

## Accessibility

- Tab strip uses `role="tablist"` with `aria-selected` per tab.
- Error banner gains `role="alert"` `aria-live="assertive"`.
- Budget progress bars are exposed as `role="progressbar"` with `aria-valuenow`/`aria-valuemin`/`aria-valuemax` and a verbose `aria-label` carrying name, used, limit, period, and percent.
- Remove buttons gain `aria-label` with the budget name.
- Alert severity badges use color + text label (severity name visible in addition to badge color), so the panel doesn't rely on color alone.

---

## Cross-client behaviour

| Client | Usage UI |
|---|---|
| **VibeUI / VibeApp** | Full panel |
| **VibeMobile** | None |
| **VibeWatch** | None |
| **IDE plugins** | None |

Mobile and IDE clients still incur metered usage when they make LLM calls through the daemon — they just can't view the metering data. A future surface might add a daily-spend-summary push to mobile, but that doesn't exist today.

---

## Troubleshooting

### "Total spend is $0 but I've made requests"

The daemon may not be configured to log to the metering store. Check `~/.vibeui/usage-metering.json` exists; if not, the metering hooks aren't wired into the active provider path. Restart the daemon with `RUST_LOG=vibecody::usage=info` and watch for `usage.request.recorded` events on each LLM call.

### "Budget shows 100% but no alert appeared"

Alerts are generated at the moment the threshold is crossed, not on a periodic check. If the threshold was crossed before the alert dispatcher was wired, no alert will ever fire for it. Increment usage by 1 cent and watch for the alert.

### "Removed budgets keep coming back"

You're on a build before the `delete_usage_budget` fix. Edit `~/.vibeui/usage-metering.json` and remove the entries from the `budgets` array directly.

### "By Task report is always empty"

That column isn't wired to a real source yet. The `task` reportData is `[]` in the panel; the placeholder exists for a future wiring of agent-task → cost attribution. Don't read into the empty state.

---

## Related

- **Cost router:** [`docs/cost-router/`] (TODO) — the routing layer that picks the cheapest provider for a request given quality constraints
- **Source:** `vibeui/src/components/UsageMeteringPanel.tsx` (~310 LOC) · backend `vibeui/src-tauri/src/commands.rs` (`create_usage_budget`, `delete_usage_budget`, `dismiss_usage_alert`, `get_usage_kpis`, `get_usage_budgets`, `get_usage_by_provider`, `get_usage_by_model`, `get_usage_alerts`)
