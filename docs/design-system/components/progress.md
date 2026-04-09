---
render_with_liquid: false
layout: page
title: Progress — Design System
permalink: /design-system/components/progress/
---

# Progress

Progress bars communicate completion, score, quality, and acceptance rates. All progress bars animate their width change automatically via CSS transitions.

---

## Anatomy

```
┌──────────────────────────────────────────────────────┐  ← .progress-bar (track)
│▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓░░░░░░░░░░░░░░░░░░░░  │  ← .progress-bar-fill + .progress-bar-{color}
└──────────────────────────────────────────────────────┘
```

---

## Basic Usage

```tsx
{/* Static color */}
<div className="progress-bar">
  <div className="progress-bar-fill progress-bar-accent" style={{ width: "72%" }} />
</div>

{/* Dynamic semantic color */}
<div className="progress-bar">
  <div
    className="progress-bar-fill"
    style={{
      width: `${score}%`,
      background: score >= 80 ? "var(--success-color)"
        : score >= 60 ? "var(--warning-color)"
        : "var(--error-color)"
    }}
  />
</div>

{/* Class-based color */}
<div className="progress-bar">
  <div
    className={`progress-bar-fill ${score >= 80 ? "progress-bar-success" : score >= 60 ? "progress-bar-warning" : "progress-bar-danger"}`}
    style={{ width: `${score}%` }}
  />
</div>
```

---

## CSS Definitions

```css
/* Track */
.progress-bar    { height: 6px; background: var(--bg-primary); border-radius: var(--radius-xs); overflow: hidden; }
.progress-bar-sm { height: 3px; }
.progress-bar-lg { height: 8px; }

/* Fill */
.progress-bar-fill    { height: 100%; border-radius: var(--radius-xs); transition: width var(--transition-smooth); }
.progress-bar-success { background: var(--success-color); }
.progress-bar-warning { background: var(--warning-color); }
.progress-bar-danger  { background: var(--error-color); }
.progress-bar-info    { background: var(--info-color); }
.progress-bar-accent  { background: var(--accent-color); }
```

---

## Size Variants

```
Class            Height   Use
progress-bar     6px      Default — scores, progress, acceptance rates
progress-bar-sm  3px      Very compact rows, minimal visual weight
progress-bar-lg  8px      Emphasis (overall score, primary metric)
```

```tsx
{/* Full row with value label */}
<div style={{ display: "flex", alignItems: "center", gap: 8 }}>
  <div className="progress-bar" style={{ flex: 1 }}>
    <div className="progress-bar-fill progress-bar-success" style={{ width: "82%" }} />
  </div>
  <span style={{ fontSize: "var(--font-size-xs)", fontWeight: "var(--font-semibold)", color: "var(--text-success)", minWidth: 32 }}>
    82%
  </span>
</div>
```

---

## Confidence Bar (AI predictions)

```tsx
const confColor = (c: number) =>
  c > 0.85 ? "var(--success-color)"
  : c > 0.70 ? "var(--warning-color)"
  : "var(--error-color)";

<div style={{ display: "flex", alignItems: "center", gap: 8 }}>
  <div className="progress-bar" style={{ flex: 1 }}>
    <div
      className="progress-bar-fill"
      style={{ width: `${prediction.confidence * 100}%`, background: confColor(prediction.confidence) }}
    />
  </div>
  <span style={{
    fontSize: "var(--font-size-xs)",
    fontWeight: "var(--font-semibold)",
    color: confColor(prediction.confidence),
    minWidth: 32,
  }}>
    {(prediction.confidence * 100).toFixed(0)}%
  </span>
</div>
```

---

## Score Bar (health / quality)

```tsx
const scoreColor = (s: number) =>
  s >= 80 ? "var(--success-color)"
  : s >= 60 ? "var(--warning-color)"
  : "var(--error-color)";

{/* Inside a panel-card */}
<div className="panel-row" style={{ marginBottom: 6 }}>
  <span style={{ fontWeight: "var(--font-semibold)" }}>{dimension.name}</span>
  <span style={{ marginLeft: "auto", color: scoreColor(dimension.score), fontWeight: "var(--font-semibold)" }}>
    {dimension.score.toFixed(0)}/100
  </span>
</div>
<div className="progress-bar" style={{ marginBottom: 6 }}>
  <div
    className="progress-bar-fill"
    style={{ width: `${dimension.score}%`, background: scoreColor(dimension.score) }}
  />
</div>
```

---

## Stacked / Multi-segment Bar

For showing code composition (code / comments / blank):

```tsx
<div style={{ display: "flex", height: 6, borderRadius: "var(--radius-xs)", overflow: "hidden", background: "var(--bg-primary)" }}>
  <div style={{ width: `${codePct}%`, background: "var(--accent-blue)" }} />
  <div style={{ width: `${commentPct}%`, background: "var(--accent-green)" }} />
  <div style={{ width: `${blankPct}%`, background: "var(--bg-tertiary)" }} />
</div>
```

---

## Minimum Width

Always set a minimum width so empty bars are visible:

```tsx
const barWidth = Math.max(4, Math.round((value / max) * 100));
<div className="progress-bar-fill progress-bar-accent" style={{ width: `${barWidth}%` }} />
```

`Math.max(4, ...)` ensures even zero-ish values show a 4% sliver.

---

## Rules

### ✅ Do
- Use `progress-bar` + `progress-bar-fill` always as a pair
- Add a semantic fill class (`progress-bar-success` etc.) or inline `background`
- Use `Math.max(4, ...)` for minimum visible width
- Use `flex: 1` on the track when placing beside a label in a row
- Keep bars in `panel-card` or below a `panel-row` — never standalone in a body

### ❌ Don't
```tsx
// Inline bar from scratch
<div style={{ height: 6, background: "var(--bg-primary)", borderRadius: 2, overflow: "hidden" }}>
  <div style={{ width: "60%", height: "100%", background: "#4caf50", borderRadius: 2 }} />
</div>
// → use .progress-bar + .progress-bar-fill.progress-bar-success

// No minimum width
<div className="progress-bar-fill" style={{ width: `${v/total*100}%` }} />  // can be 0%

// Wrong track color
<div style={{ height: 6, background: "var(--bg-tertiary)", ... }} />  // use --bg-primary for track
```
