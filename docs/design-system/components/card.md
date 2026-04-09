---
layout: page
title: Card — Design System
permalink: /design-system/components/card/
---

{% raw %}
# Card

Cards group related content into a visually distinct surface. In VibeUI, cards are the primary content container within `panel-body`.

---

## Base Card

```tsx
<div className="panel-card">
  Content
</div>
```

```css
.panel-card {
  background: var(--bg-secondary);
  border-radius: var(--radius-sm);   /* 6px */
  padding: 12px;
  border: 1px solid var(--border-color);
}
```

---

## Card Spacing

Cards in a `panel-body` need vertical separation. Use `marginBottom: 8` for consistent 8px gaps:

```tsx
<div className="panel-body">
  <div className="panel-card" style={{ marginBottom: 8 }}>Card A</div>
  <div className="panel-card" style={{ marginBottom: 8 }}>Card B</div>
  <div className="panel-card">Card C</div>   {/* last card — no margin */}
</div>
```

Or use a flex column with `gap`:

```tsx
<div className="panel-body">
  <div style={{ display: "flex", flexDirection: "column", gap: 8 }}>
    <div className="panel-card">Card A</div>
    <div className="panel-card">Card B</div>
  </div>
</div>
```

---

## Card Anatomy

```
┌─────────────────────────────────────────────────────┐
│ card-header-row  (flex, justify-between)            │
│   card-title                   card-status/badge    │
│─────────────────────────────────────────────────────│ optional divider
│ card-description / details                          │
│─────────────────────────────────────────────────────│
│ progress-bar or sub-stats                           │
│─────────────────────────────────────────────────────│
│ card-footer-row (flex, actions)                     │
└─────────────────────────────────────────────────────┘
```

```tsx
<div className="panel-card" style={{ marginBottom: 8 }}>
  {/* Header row */}
  <div className="panel-row" style={{ marginBottom: 6 }}>
    <span style={{ fontWeight: "var(--font-semibold)", fontSize: "var(--font-size-base)" }}>
      Item Name
    </span>
    <span className="panel-tag panel-tag-success" style={{ marginLeft: 8 }}>active</span>
    <span style={{ marginLeft: "auto", fontSize: "var(--font-size-xs)", color: "var(--text-secondary)" }}>
      2 min ago
    </span>
  </div>

  {/* Description */}
  <div style={{ fontSize: "var(--font-size-base)", color: "var(--text-secondary)", marginBottom: 8 }}>
    Brief description of the item goes here.
  </div>

  {/* Progress */}
  <div className="progress-bar" style={{ marginBottom: 8 }}>
    <div className="progress-bar-fill progress-bar-success" style={{ width: "72%" }} />
  </div>

  {/* Footer actions */}
  <div style={{ display: "flex", gap: 6, justifyContent: "flex-end" }}>
    <button className="panel-btn panel-btn-secondary panel-btn-xs">Dismiss</button>
    <button className="panel-btn panel-btn-primary panel-btn-xs">Apply</button>
  </div>
</div>
```

---

## Accent Border (status cards)

Add a left border accent to signal semantic status without full badge:

```tsx
{/* Success — start of review */}
<div className="panel-card" style={{ borderLeft: "3px solid var(--success-color)" }}>
  <div style={{ fontWeight: "var(--font-semibold)" }}>Review Started</div>
  ...
</div>

{/* Error — critical issue */}
<div className="panel-card" style={{ borderLeft: "3px solid var(--error-color)" }}>
  ...
</div>

{/* Warning */}
<div className="panel-card" style={{ borderLeft: "3px solid var(--warning-color)" }}>
  ...
</div>

{/* Info */}
<div className="panel-card" style={{ borderLeft: "3px solid var(--info-color)" }}>
  ...
</div>
```

---

## Nested Surfaces

When you need a sub-section inside a card, use `--bg-primary` (one level deeper):

```tsx
<div className="panel-card">
  <div style={{ fontWeight: "var(--font-semibold)", marginBottom: 8 }}>Q-Table Statistics</div>
  <div style={{ display: "flex", gap: 8 }}>
    {stats.map(([label, value]) => (
      <div
        key={label}
        style={{
          flex: 1, textAlign: "center", padding: 8,
          background: "var(--bg-primary)",   {/* one step deeper than card */}
          borderRadius: "var(--radius-sm)",
        }}
      >
        <div className="panel-mono" style={{ fontSize: "var(--font-size-2xl)", fontWeight: "var(--font-bold)" }}>
          {value}
        </div>
        <div className="panel-stat-label">{label}</div>
      </div>
    ))}
  </div>
</div>
```

This creates a clear nested hierarchy: `bg-secondary` (card) → `bg-primary` (inner tile) without additional border.

---

## Selectable Cards

When a card can be selected (e.g. choosing from a list):

```tsx
<div
  role="button"
  tabIndex={0}
  className="panel-card"
  style={{
    cursor: "pointer",
    opacity: item.state !== "pending" ? 0.6 : 1,
    border: `1px solid ${isSelected ? "var(--accent-color)" : "var(--border-color)"}`,
    background: isSelected
      ? "color-mix(in srgb, var(--accent-blue) 12%, var(--bg-secondary))"
      : "var(--bg-secondary)",
  }}
  onClick={() => setSelected(item.id)}
  onKeyDown={e => e.key === "Enter" && setSelected(item.id)}
>
  ...
</div>
```

---

## Grid of Cards

For 2–3 column card grids (e.g. stat cards):

```tsx
{/* Use panel-stats for horizontal stat cards */}
<div className="panel-stats" style={{ marginBottom: 10 }}>
  <div className="panel-stat"> ... </div>
  <div className="panel-stat"> ... </div>
  <div className="panel-stat"> ... </div>
</div>

{/* 3-column grid for equal content cards */}
<div className="panel-stats-grid-3">
  <div className="panel-card"> ... </div>
  <div className="panel-card"> ... </div>
  <div className="panel-card"> ... </div>
</div>
```

---

## Rules

### ✅ Do
- Use `panel-card` as the base — never replicate the border/bg/radius inline
- Add `marginBottom: 8` between stacked cards
- Use `--bg-primary` for nested sub-surfaces inside cards
- Use accent `borderLeft` for semantic status cards (not color fills)
- Keep padding at the card level — don't add extra padding inside the card's first child

### ❌ Don't
```tsx
// Replicating panel-card inline
<div style={{ background: "var(--bg-secondary)", borderRadius: 6, padding: 12, border: "1px solid var(--border-color)" }}>

// Card inside card (same level)
<div className="panel-card">
  <div className="panel-card"> ... </div>   // use bg-primary nested div instead

// Hardcoded accent border
<div style={{ borderLeft: "3px solid #4caf50" }}>   // use var(--success-color)
```
{% endraw %}
