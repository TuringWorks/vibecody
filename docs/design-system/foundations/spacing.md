---
layout: page
title: Spacing — Design System
permalink: /design-system/foundations/spacing/
---

# Spacing

All spacing uses a **4px base grid**. Never use arbitrary pixel values — always pick the nearest token.

---

## Space Scale

| Token | Value | Use case |
|---|---|---|
| `--space-1` | 4px | Icon gap, tight row spacing |
| `--space-2` | 8px | Related elements, compact padding |
| `--space-3` | 12px | Card padding, default gap |
| `--space-4` | 16px | Section padding, form fields |
| `--space-5` | 20px | Large section gap |
| `--space-6` | 24px | Between major sections |
| `--space-8` | 32px | Empty state padding, hero sections |

---

## Layout vs Component Spacing

**Layout spacing** (between panels, sections): `--space-4` to `--space-8`

**Component spacing** (inside cards, between elements): `--space-1` to `--space-3`

---

## Usage Rules

```tsx
// ✅ Use tokens
gap: "var(--space-2)"
padding: "var(--space-3)"
marginBottom: "var(--space-4)"

// ❌ Avoid arbitrary values
gap: 7          // use --space-2 (8px)
padding: 14     // use --space-3 (12px) or --space-4 (16px)
marginBottom: 5 // use --space-1 (4px) or --space-2 (8px)
```

---

## Common Patterns

```tsx
// Card
<div className="panel-card" style={{ marginBottom: "var(--space-2)" }}>

// Icon + label row
<div style={{ display: "flex", alignItems: "center", gap: "var(--space-1)" }}>
  <Icon size={14} />
  <span>Label</span>
</div>

// Form field
<div style={{ marginBottom: "var(--space-3)" }}>
  <label style={{ marginBottom: "var(--space-1)" }}>Field</label>
  <input />
</div>
```
