# Color

VibeUI uses a two-layer color system: **palette tokens** (raw values) and **semantic tokens** (intent-based aliases). Always use semantic tokens in component code. Only use palette tokens when creating new semantic aliases.

---

## Palette — Dark Theme (default)

```
Background
  --bg-primary:    #0f1117   deepest — page canvas
  --bg-secondary:  #161821   cards, panel surfaces
  --bg-tertiary:   #1c1f2b   inputs, subtle hover areas
  --bg-elevated:   #222638   modals, dropdowns, floating surfaces

Border
  --border-color:  rgba(255,255,255,0.06)   standard
  --border-subtle: rgba(255,255,255,0.03)   dividers between rows

Text
  --text-primary:   #e2e4ea   body, headings
  --text-secondary: #6e7491   labels, captions
  --text-muted:     #4b5068   placeholders, timestamps, disabled

Accent palette
  --accent-blue:   #6c8cff   primary brand
  --accent-green:  #34d399   success
  --accent-purple: #a78bfa   tags, highlights
  --accent-gold:   #f5c542   warning, highlights
  --accent-rose:   #f472b6   destructive highlights

Semantic
  --error-color:   #ef4444   red
  --warning-color: var(--accent-gold)
  --success-color: var(--accent-green)
  --info-color:    var(--accent-blue)
  --accent-color:  var(--accent-blue)   (primary)

Semantic text
  --text-success:  var(--accent-green)
  --text-danger:   #ef4444
  --text-warning:  var(--accent-gold)
  --text-info:     #89b4fa
  --text-accent:   var(--accent-blue)

Semantic backgrounds (10% opacity tints)
  --success-bg:  rgba(52,211,153,0.10)
  --error-bg:    rgba(239,68,68,0.10)
  --warning-bg:  rgba(245,197,66,0.10)
  --info-bg:     rgba(108,140,255,0.10)
  --accent-bg:   rgba(108,140,255,0.15)

Glass / Frosted
  --glass-bg:     rgba(22,24,33,0.75)
  --glass-border: rgba(255,255,255,0.08)
  --glass-blur:   16px

Elevation (shadows)
  --elevation-1: 0 1px 2px rgba(0,0,0,0.30)
  --elevation-2: 0 4px 12px rgba(0,0,0,0.35)
  --elevation-3: 0 8px 30px rgba(0,0,0,0.45)
  --glow-accent: 0 0 20px rgba(108,140,255,0.15)
  --card-shadow: 0 1px 3px rgba(0,0,0,0.4), 0 4px 16px rgba(0,0,0,0.25)
```

## Palette — Light Theme (`[data-theme="light"]`)

```
  --bg-primary:    #fafbfd
  --bg-secondary:  #f0f1f5
  --bg-tertiary:   #e6e8ef
  --bg-elevated:   #ffffff

  --border-color:  rgba(0,0,0,0.08)
  --border-subtle: rgba(0,0,0,0.04)

  --text-primary:   #1a1d2e
  --text-secondary: #6b7089
  --text-muted:     #9ca3af

  --accent-blue:   #4f6df5
  --accent-green:  #10b981
  --accent-gold:   #d4a017
  --error-color:   #dc2626
  --text-danger:   #dc2626
  --text-info:     #4f6df5

  --success-bg: rgba(16,185,129,0.10)
  --error-bg:   rgba(220,38,38,0.10)
```

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

### ✅ Always
```tsx
// Use semantic tokens for status colors
color: "var(--success-color)"
color: "var(--text-danger)"
background: "var(--error-bg)"

// Dynamic color from data — inline is fine
style={{ color: score > 80 ? "var(--success-color)" : "var(--warning-color)" }}
```

### ❌ Never
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
