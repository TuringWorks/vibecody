---
layout: page
title: Color — Design System
permalink: /design-system/foundations/color/
---

# Color

VibeUI uses a two-layer color system: **palette tokens** (raw values) and **semantic tokens** (intent-based aliases). Always use semantic tokens in component code.

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
  --accent-gold:   #f5c542   warning
  --accent-rose:   #f472b6   destructive highlights

Semantic
  --error-color:   #ef4444
  --warning-color: var(--accent-gold)
  --success-color: var(--accent-green)
  --info-color:    var(--accent-blue)
  --accent-color:  var(--accent-blue)

Semantic backgrounds (10% opacity tints)
  --success-bg:  rgba(52,211,153,0.10)
  --error-bg:    rgba(239,68,68,0.10)
  --warning-bg:  rgba(245,197,66,0.10)
  --info-bg:     rgba(108,140,255,0.10)
  --accent-bg:   rgba(108,140,255,0.15)
```

## Palette — Light Theme (`[data-theme="light"]`)

```
  --bg-primary:    #fafbfd
  --bg-secondary:  #f0f1f5
  --bg-tertiary:   #e6e8ef
  --bg-elevated:   #ffffff

  --text-primary:   #1a1d2e
  --text-secondary: #6b7089
  --text-muted:     #9ca3af

  --accent-blue:   #4f6df5
  --accent-green:  #10b981
  --accent-gold:   #d4a017
  --error-color:   #dc2626
```

---

## Semantic Color Decision Tree

```
Is it communicating status?
  ├─ Success / pass / healthy    → --success-color / --text-success / --success-bg
  ├─ Error / fail / critical     → --error-color   / --text-danger  / --error-bg
  ├─ Warning / degraded          → --warning-color / --text-warning / --warning-bg
  └─ Info / in-progress          → --info-color    / --text-info    / --info-bg

Is it interactive?
  ├─ Primary action              → --accent-color
  └─ Secondary / ghost           → --text-secondary, --border-color

Is it text?
  ├─ Body copy                   → --text-primary
  ├─ Labels, metadata            → --text-secondary
  └─ Disabled, placeholder       → --text-muted

Is it a surface?
  ├─ Page background             → --bg-primary
  ├─ Card, panel body            → --bg-secondary
  ├─ Input, hover                → --bg-tertiary
  └─ Modal, dropdown, tooltip    → --bg-elevated
```

---

## CSS Utility Classes

```css
/* Text color */
.text-success, .text-error, .text-warning, .text-info, .text-muted, .text-accent

/* Background tint */
.bg-success, .bg-error, .bg-warning, .bg-info

/* Filled badges */
.badge-success, .badge-error, .badge-warning, .badge-info, .badge-neutral
```

---

## Usage Rules

```tsx
// ✅ Always use semantic tokens
color: "var(--success-color)"
background: "var(--error-bg)"
style={{ color: score > 80 ? "var(--success-color)" : "var(--warning-color)" }}

// ❌ Never hardcode hex — breaks themes
color: "#4caf50"     // use var(--success-color)
color: "#f44336"     // use var(--error-color)
background: "rgba(239,68,68,0.1)"  // use var(--error-bg)
```

## Score / Health Color Pattern

```tsx
const scoreColor = (n: number) =>
  n >= 80 ? "var(--success-color)"
  : n >= 60 ? "var(--warning-color)"
  : "var(--error-color)";
```

## Status Tag Pattern

```tsx
const statusTag = (s: string) => {
  const l = s.toLowerCase();
  if (l.includes("pass") || l.includes("ok") || l.includes("complete"))
    return "panel-tag panel-tag-success";
  if (l.includes("warn") || l.includes("progress"))
    return "panel-tag panel-tag-warning";
  if (l.includes("fail") || l.includes("error") || l.includes("critical"))
    return "panel-tag panel-tag-danger";
  return "panel-tag panel-tag-neutral";
};
```
