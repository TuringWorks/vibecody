# Spacing

VibeUI uses a **4px base grid**. All spacing values are multiples of 4. Space tokens are defined as CSS custom properties.

---

## Space Scale

```
Token        Value   Name       Use cases
--space-1    4px     xs         Icon-to-label gap, tight inline spacing
--space-2    8px     sm         Between related elements (label + input gap, button gap)
--space-3    12px    md         Card internal padding, row gaps, section gaps
--space-4    16px    lg         Large card padding, section padding
--space-5    20px    xl         Panel padding (when generous), between sections
--space-6    24px    2xl        Content blocks, large section separation
--space-8    32px    3xl        Empty-state vertical padding
```

---

## Anatomy of a Panel — Space Allocation

```
┌─────────────────────────────────────────────┐
│  panel-header  (padding: 8px 12px)          │  ← --space-2 vertical, --space-3 horizontal
│─────────────────────────────────────────────│  border: 1px solid --border-color
│  panel-body    (padding: 8px 12px)          │  ← same as header
│                                             │
│  ┌─────────────────────────────────────┐    │
│  │  panel-card  (padding: 12px)        │    │  ← --space-3 all sides
│  │                                     │    │
│  │  card-row    (gap: 8px)             │    │  ← --space-2 between elements
│  │  card-row    (margin-bottom: 8px)   │    │  ← --space-2 between rows
│  └─────────────────────────────────────┘    │
│  (gap between cards: 8px)                   │  ← --space-2
│  ┌─────────────────────────────────────┐    │
│  │  panel-card                         │    │
│  └─────────────────────────────────────┘    │
│─────────────────────────────────────────────│  border: 1px solid --border-color
│  panel-footer  (padding: 8px 12px)          │  ← --space-2 vertical, --space-3 horizontal
└─────────────────────────────────────────────┘
```

---

## Component-Level Spacing Guide

### Buttons
```
panel-btn-xs:  padding 2px 8px    gap 4px
panel-btn-sm:  padding 4px 10px   gap 6px
panel-btn:     padding 5px 12px   gap 6px  (default)
panel-btn-lg:  padding 8px 18px   gap 8px
```

### Button groups
```tsx
// Inline button row — 6px gap
<div style={{ display: "flex", gap: 6 }}>
  <button className="panel-btn panel-btn-secondary">Cancel</button>
  <button className="panel-btn panel-btn-primary">Save</button>
</div>
```

### Cards in body
```tsx
// Cards stacked vertically — 8px margin-bottom
<div className="panel-card" style={{ marginBottom: 8 }}>...</div>
<div className="panel-card" style={{ marginBottom: 8 }}>...</div>

// Or use gap in a flex column
<div style={{ display: "flex", flexDirection: "column", gap: 8 }}>
  <div className="panel-card">...</div>
  <div className="panel-card">...</div>
</div>
```

### Label → Input → Button stack
```tsx
<div className="panel-card" style={{ marginBottom: 10 }}>
  <div className="panel-label">Field Name</div>           {/* label */}
  <input className="panel-input panel-input-full"         {/* input — 8px below label via margin-bottom: 4px from .panel-label */}
    style={{ marginBottom: 8 }} />
  <button className="panel-btn panel-btn-primary">Submit</button>
</div>
```

### Inline row items
```tsx
// Tight: icon next to label
<span style={{ display: "flex", alignItems: "center", gap: "var(--space-1)" }}>
  <Icon name="check" size={12} />
  <span>Verified</span>
</span>

// Normal: label + value pairs
<div style={{ display: "flex", alignItems: "center", gap: "var(--space-2)" }}>
  <span className="panel-label">Status</span>
  <span className="panel-tag panel-tag-success">Active</span>
</div>
```

---

## Stats Grid Spacing

```tsx
// panel-stats uses gap: 8px by default
<div className="panel-stats">
  <div className="panel-stat"> ... </div>   {/* flex: 1 each */}
  <div className="panel-stat"> ... </div>
</div>

// 3-column grid
<div className="panel-stats-grid-3">
  <div className="panel-stat"> ... </div>
  <div className="panel-stat"> ... </div>
  <div className="panel-stat"> ... </div>
</div>
```

---

## Empty State Spacing

```tsx
// panel-empty: padding 40px 20px
<div className="panel-empty">No items found.</div>

// Custom empty state — stick to 32px top/bottom
<div style={{ textAlign: "center", padding: "32px 20px", color: "var(--text-muted)" }}>
  ...
</div>
```

---

## Margin vs Padding

| Situation | Use | Why |
|-----------|-----|-----|
| Inside a card/section | `padding` | Owns the space |
| Between stacked cards | `marginBottom` on card | Separation between siblings |
| Inside flex/grid gap | `gap` | Cleaner than margins |
| Around a progress bar | `marginTop: 6` | Sub-element internal spacing |
| Before a button in a card | `marginTop: 8` | Separates from above content |

---

## ✅ / ❌

### ✅ Do
```tsx
gap: 8          // 2×4 grid
padding: 12     // 3×4 grid
marginBottom: 8
style={{ gap: "var(--space-2)" }}  // explicit token
```

### ❌ Don't
```tsx
padding: 10     // not on the 4px grid
marginBottom: 6  // ok sometimes but prefer 8
padding: "3px 7px"  // off-grid
gap: 14         // off-grid; use 12 or 16
```
