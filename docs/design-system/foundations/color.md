---
render_with_liquid: false
layout: page
title: Color — Design System
permalink: /design-system/foundations/color/
---

# Color

VibeUI uses a two-layer color system: **palette tokens** (raw values) and **semantic tokens** (intent-based aliases). Always use semantic tokens in component code. Only use palette tokens when creating new semantic aliases.

---

## Palette — Dark Theme (default)

<div class="swatch-grid">
  <h4>Background</h4>
  <div class="swatch"><div class="swatch-chip"><span style="background:#0f1117"></span></div><div class="swatch-meta"><div class="swatch-token">--bg-primary</div><div class="swatch-value">#0f1117</div><div class="swatch-desc">deepest — page canvas</div></div></div>
  <div class="swatch"><div class="swatch-chip"><span style="background:#161821"></span></div><div class="swatch-meta"><div class="swatch-token">--bg-secondary</div><div class="swatch-value">#161821</div><div class="swatch-desc">cards, panel surfaces</div></div></div>
  <div class="swatch"><div class="swatch-chip"><span style="background:#1c1f2b"></span></div><div class="swatch-meta"><div class="swatch-token">--bg-tertiary</div><div class="swatch-value">#1c1f2b</div><div class="swatch-desc">inputs, subtle hover areas</div></div></div>
  <div class="swatch"><div class="swatch-chip"><span style="background:#222638"></span></div><div class="swatch-meta"><div class="swatch-token">--bg-elevated</div><div class="swatch-value">#222638</div><div class="swatch-desc">modals, dropdowns, floating</div></div></div>

  <h4>Border</h4>
  <div class="swatch"><div class="swatch-chip"><span style="background:rgba(255,255,255,0.06)"></span></div><div class="swatch-meta"><div class="swatch-token">--border-color</div><div class="swatch-value">rgba(255,255,255,0.06)</div><div class="swatch-desc">standard</div></div></div>
  <div class="swatch"><div class="swatch-chip"><span style="background:rgba(255,255,255,0.03)"></span></div><div class="swatch-meta"><div class="swatch-token">--border-subtle</div><div class="swatch-value">rgba(255,255,255,0.03)</div><div class="swatch-desc">dividers between rows</div></div></div>

  <h4>Text</h4>
  <div class="swatch"><div class="swatch-chip"><span style="background:#e2e4ea"></span></div><div class="swatch-meta"><div class="swatch-token">--text-primary</div><div class="swatch-value">#e2e4ea</div><div class="swatch-desc">body, headings</div></div></div>
  <div class="swatch"><div class="swatch-chip"><span style="background:#6e7491"></span></div><div class="swatch-meta"><div class="swatch-token">--text-secondary</div><div class="swatch-value">#6e7491</div><div class="swatch-desc">labels, captions</div></div></div>
  <div class="swatch"><div class="swatch-chip"><span style="background:#4b5068"></span></div><div class="swatch-meta"><div class="swatch-token">--text-muted</div><div class="swatch-value">#4b5068</div><div class="swatch-desc">placeholders, disabled</div></div></div>

  <h4>Accent palette</h4>
  <div class="swatch"><div class="swatch-chip"><span style="background:#6c8cff"></span></div><div class="swatch-meta"><div class="swatch-token">--accent-blue</div><div class="swatch-value">#6c8cff</div><div class="swatch-desc">primary brand</div></div></div>
  <div class="swatch"><div class="swatch-chip"><span style="background:#34d399"></span></div><div class="swatch-meta"><div class="swatch-token">--accent-green</div><div class="swatch-value">#34d399</div><div class="swatch-desc">success</div></div></div>
  <div class="swatch"><div class="swatch-chip"><span style="background:#a78bfa"></span></div><div class="swatch-meta"><div class="swatch-token">--accent-purple</div><div class="swatch-value">#a78bfa</div><div class="swatch-desc">tags, highlights</div></div></div>
  <div class="swatch"><div class="swatch-chip"><span style="background:#f5c542"></span></div><div class="swatch-meta"><div class="swatch-token">--accent-gold</div><div class="swatch-value">#f5c542</div><div class="swatch-desc">warning, highlights</div></div></div>
  <div class="swatch"><div class="swatch-chip"><span style="background:#f472b6"></span></div><div class="swatch-meta"><div class="swatch-token">--accent-rose</div><div class="swatch-value">#f472b6</div><div class="swatch-desc">destructive highlights</div></div></div>

  <h4>Semantic</h4>
  <div class="swatch"><div class="swatch-chip"><span style="background:#ef4444"></span></div><div class="swatch-meta"><div class="swatch-token">--error-color</div><div class="swatch-value">#ef4444</div><div class="swatch-desc">red</div></div></div>
  <div class="swatch"><div class="swatch-chip"><span style="background:#f5c542"></span></div><div class="swatch-meta"><div class="swatch-token">--warning-color</div><div class="swatch-value">var(--accent-gold)</div><div class="swatch-desc">→ #f5c542</div></div></div>
  <div class="swatch"><div class="swatch-chip"><span style="background:#34d399"></span></div><div class="swatch-meta"><div class="swatch-token">--success-color</div><div class="swatch-value">var(--accent-green)</div><div class="swatch-desc">→ #34d399</div></div></div>
  <div class="swatch"><div class="swatch-chip"><span style="background:#6c8cff"></span></div><div class="swatch-meta"><div class="swatch-token">--info-color</div><div class="swatch-value">var(--accent-blue)</div><div class="swatch-desc">→ #6c8cff</div></div></div>
  <div class="swatch"><div class="swatch-chip"><span style="background:#6c8cff"></span></div><div class="swatch-meta"><div class="swatch-token">--accent-color</div><div class="swatch-value">var(--accent-blue)</div><div class="swatch-desc">primary</div></div></div>

  <h4>Semantic text</h4>
  <div class="swatch"><div class="swatch-chip"><span style="background:#34d399"></span></div><div class="swatch-meta"><div class="swatch-token">--text-success</div><div class="swatch-value">#34d399</div></div></div>
  <div class="swatch"><div class="swatch-chip"><span style="background:#ef4444"></span></div><div class="swatch-meta"><div class="swatch-token">--text-danger</div><div class="swatch-value">#ef4444</div></div></div>
  <div class="swatch"><div class="swatch-chip"><span style="background:#f5c542"></span></div><div class="swatch-meta"><div class="swatch-token">--text-warning</div><div class="swatch-value">#f5c542</div></div></div>
  <div class="swatch"><div class="swatch-chip"><span style="background:#89b4fa"></span></div><div class="swatch-meta"><div class="swatch-token">--text-info</div><div class="swatch-value">#89b4fa</div></div></div>
  <div class="swatch"><div class="swatch-chip"><span style="background:#6c8cff"></span></div><div class="swatch-meta"><div class="swatch-token">--text-accent</div><div class="swatch-value">#6c8cff</div></div></div>

  <h4>Semantic backgrounds (10% tint)</h4>
  <div class="swatch"><div class="swatch-chip"><span style="background:rgba(52,211,153,0.10)"></span></div><div class="swatch-meta"><div class="swatch-token">--success-bg</div><div class="swatch-value">rgba(52,211,153,0.10)</div></div></div>
  <div class="swatch"><div class="swatch-chip"><span style="background:rgba(239,68,68,0.10)"></span></div><div class="swatch-meta"><div class="swatch-token">--error-bg</div><div class="swatch-value">rgba(239,68,68,0.10)</div></div></div>
  <div class="swatch"><div class="swatch-chip"><span style="background:rgba(245,197,66,0.10)"></span></div><div class="swatch-meta"><div class="swatch-token">--warning-bg</div><div class="swatch-value">rgba(245,197,66,0.10)</div></div></div>
  <div class="swatch"><div class="swatch-chip"><span style="background:rgba(108,140,255,0.10)"></span></div><div class="swatch-meta"><div class="swatch-token">--info-bg</div><div class="swatch-value">rgba(108,140,255,0.10)</div></div></div>
  <div class="swatch"><div class="swatch-chip"><span style="background:rgba(108,140,255,0.15)"></span></div><div class="swatch-meta"><div class="swatch-token">--accent-bg</div><div class="swatch-value">rgba(108,140,255,0.15)</div></div></div>
</div>

### Glass / Frosted

```
--glass-bg:     rgba(22,24,33,0.75)
--glass-border: rgba(255,255,255,0.08)
--glass-blur:   16px
```

### Elevation (shadows)

```
--elevation-1: 0 1px 2px rgba(0,0,0,0.30)
--elevation-2: 0 4px 12px rgba(0,0,0,0.35)
--elevation-3: 0 8px 30px rgba(0,0,0,0.45)
--glow-accent: 0 0 20px rgba(108,140,255,0.15)
--card-shadow: 0 1px 3px rgba(0,0,0,0.4), 0 4px 16px rgba(0,0,0,0.25)
```

---

## Palette — Light Theme (`[data-theme="light"]`)

<div class="swatch-grid">
  <h4>Background</h4>
  <div class="swatch"><div class="swatch-chip"><span style="background:#fafbfd"></span></div><div class="swatch-meta"><div class="swatch-token">--bg-primary</div><div class="swatch-value">#fafbfd</div></div></div>
  <div class="swatch"><div class="swatch-chip"><span style="background:#f0f1f5"></span></div><div class="swatch-meta"><div class="swatch-token">--bg-secondary</div><div class="swatch-value">#f0f1f5</div></div></div>
  <div class="swatch"><div class="swatch-chip"><span style="background:#e6e8ef"></span></div><div class="swatch-meta"><div class="swatch-token">--bg-tertiary</div><div class="swatch-value">#e6e8ef</div></div></div>
  <div class="swatch"><div class="swatch-chip"><span style="background:#ffffff"></span></div><div class="swatch-meta"><div class="swatch-token">--bg-elevated</div><div class="swatch-value">#ffffff</div></div></div>

  <h4>Border</h4>
  <div class="swatch"><div class="swatch-chip"><span style="background:rgba(0,0,0,0.08)"></span></div><div class="swatch-meta"><div class="swatch-token">--border-color</div><div class="swatch-value">rgba(0,0,0,0.08)</div></div></div>
  <div class="swatch"><div class="swatch-chip"><span style="background:rgba(0,0,0,0.04)"></span></div><div class="swatch-meta"><div class="swatch-token">--border-subtle</div><div class="swatch-value">rgba(0,0,0,0.04)</div></div></div>

  <h4>Text</h4>
  <div class="swatch"><div class="swatch-chip"><span style="background:#1a1d2e"></span></div><div class="swatch-meta"><div class="swatch-token">--text-primary</div><div class="swatch-value">#1a1d2e</div></div></div>
  <div class="swatch"><div class="swatch-chip"><span style="background:#6b7089"></span></div><div class="swatch-meta"><div class="swatch-token">--text-secondary</div><div class="swatch-value">#6b7089</div></div></div>
  <div class="swatch"><div class="swatch-chip"><span style="background:#9ca3af"></span></div><div class="swatch-meta"><div class="swatch-token">--text-muted</div><div class="swatch-value">#9ca3af</div></div></div>

  <h4>Accent &amp; semantic</h4>
  <div class="swatch"><div class="swatch-chip"><span style="background:#4f6df5"></span></div><div class="swatch-meta"><div class="swatch-token">--accent-blue</div><div class="swatch-value">#4f6df5</div></div></div>
  <div class="swatch"><div class="swatch-chip"><span style="background:#10b981"></span></div><div class="swatch-meta"><div class="swatch-token">--accent-green</div><div class="swatch-value">#10b981</div></div></div>
  <div class="swatch"><div class="swatch-chip"><span style="background:#d4a017"></span></div><div class="swatch-meta"><div class="swatch-token">--accent-gold</div><div class="swatch-value">#d4a017</div></div></div>
  <div class="swatch"><div class="swatch-chip"><span style="background:#dc2626"></span></div><div class="swatch-meta"><div class="swatch-token">--error-color</div><div class="swatch-value">#dc2626</div></div></div>
  <div class="swatch"><div class="swatch-chip"><span style="background:#dc2626"></span></div><div class="swatch-meta"><div class="swatch-token">--text-danger</div><div class="swatch-value">#dc2626</div></div></div>
  <div class="swatch"><div class="swatch-chip"><span style="background:#4f6df5"></span></div><div class="swatch-meta"><div class="swatch-token">--text-info</div><div class="swatch-value">#4f6df5</div></div></div>

  <h4>Semantic backgrounds</h4>
  <div class="swatch"><div class="swatch-chip"><span style="background:rgba(16,185,129,0.10)"></span></div><div class="swatch-meta"><div class="swatch-token">--success-bg</div><div class="swatch-value">rgba(16,185,129,0.10)</div></div></div>
  <div class="swatch"><div class="swatch-chip"><span style="background:rgba(220,38,38,0.10)"></span></div><div class="swatch-meta"><div class="swatch-token">--error-bg</div><div class="swatch-value">rgba(220,38,38,0.10)</div></div></div>
</div>

---

## Semantic Color Decision Tree

```
Is it communicating status?
  ├─ Success / pass / active / healthy    → --success-color / --text-success / --success-bg
  ├─ Error / fail / critical              → --error-color   / --text-danger   / --error-bg
  ├─ Warning / slow / degraded           → --warning-color / --text-warning  / --warning-bg
  └─ Info / in-progress / neutral        → --info-color    / --text-info     / --info-bg

Is it interactive?
  ├─ Primary action (one per view)       → --accent-color
  └─ Secondary / ghost                   → --text-secondary, --border-color

Is it text?
  ├─ Body copy, values                   → --text-primary
  ├─ Labels, captions, metadata          → --text-secondary
  └─ Timestamps, disabled, placeholder   → --text-muted

Is it a surface?
  ├─ Page background                     → --bg-primary
  ├─ Card, panel body                    → --bg-secondary
  ├─ Input, hover background             → --bg-tertiary
  └─ Floating (modal, dropdown, tooltip) → --bg-elevated
```

---

## CSS Utility Classes

```css
/* Text color */
.text-success   → color: var(--success-color)
.text-error     → color: var(--error-color)
.text-warning   → color: var(--warning-color)
.text-info      → color: var(--info-color)
.text-muted     → color: var(--text-muted)
.text-accent    → color: var(--accent-color)

/* Background tint */
.bg-success     → background: var(--success-bg)
.bg-error       → background: var(--error-bg)
.bg-warning     → background: var(--warning-bg)
.bg-info        → background: var(--info-bg)

/* Filled badges */
.badge-success  → background: --success-color, white text
.badge-error    → background: --error-color,   white text
.badge-warning  → background: --warning-color, white text
.badge-info     → background: --info-color,    white text
.badge-neutral  → background: --bg-tertiary,   secondary text
```

---

## Usage Rules

### <span class="docs-do" aria-hidden="true"></span>Always

```tsx
// Use semantic tokens for status colors
color: "var(--success-color)"
color: "var(--text-danger)"
background: "var(--error-bg)"

// Dynamic color from data — inline is fine
style={{ color: score > 80 ? "var(--success-color)" : "var(--warning-color)" }}
```

### <span class="docs-dont" aria-hidden="true"></span>Never

```tsx
// Hardcoded hex — breaks both themes
color: "#4caf50"    // → var(--success-color)
color: "#f44336"    // → var(--error-color)
color: "#ff9800"    // → var(--warning-color)
color: "#2196f3"    // → var(--info-color)
color: "white"      // → var(--btn-primary-fg)
color: "#fff"       // → var(--btn-primary-fg)
background: "rgba(239,68,68,0.1)"  // → var(--error-bg)
```

---

## Score / Health Color Pattern

A common pattern for health scores, confidence values, and quality metrics:

```tsx
// Tri-state: good / warning / bad
const scoreColor = (n: number) =>
  n >= 80 ? "var(--success-color)"
  : n >= 60 ? "var(--warning-color)"
  : "var(--error-color)";

// Confidence (0–1)
const confColor = (c: number) =>
  c > 0.85 ? "var(--success-color)"
  : c > 0.70 ? "var(--warning-color)"
  : "var(--error-color)";
```

## Status Tag Pattern

```tsx
const statusTag = (s: string): string => {
  const lower = s.toLowerCase();
  if (lower.includes("pass") || lower.includes("ok") || lower.includes("success") || lower.includes("complete"))
    return "panel-tag panel-tag-success";
  if (lower.includes("warn") || lower.includes("slow") || lower.includes("progress"))
    return "panel-tag panel-tag-warning";
  if (lower.includes("fail") || lower.includes("error") || lower.includes("critical"))
    return "panel-tag panel-tag-danger";
  if (lower.includes("info") || lower.includes("run"))
    return "panel-tag panel-tag-info";
  return "panel-tag panel-tag-neutral";
};

<span className={statusTag(item.status)}>{item.status}</span>
```

---

## Accent Colors — Decorative Use Only

Use `--accent-purple`, `--accent-gold`, `--accent-rose` only for decoration (language chips, syntax highlighting, category indicators). Never for semantic status.

```tsx
const LANG_COLOR: Record<string, string> = {
  Rust:       "#dea584",
  TypeScript: "#3178c6",
  Python:     "#4584b6",
  // etc — these are decorative, not semantic
};
```
