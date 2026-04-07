---
layout: page
title: Card — Design System
permalink: /design-system/components/card/
---

# Card

Cards are the primary content container inside panels. Use `.panel-card` for any grouped piece of information.

---

## Basic Card

```tsx
<div className="panel-card">
  Content goes here
</div>
```

## Card with Header Row

```tsx
<div className="panel-card">
  <div style={{ display: "flex", justifyContent: "space-between", alignItems: "center", marginBottom: "var(--space-2)" }}>
    <span style={{ fontWeight: "var(--font-semibold)", fontSize: "var(--font-size-lg)" }}>Title</span>
    <span className="panel-tag panel-tag-success">Active</span>
  </div>
  <p style={{ color: "var(--text-secondary)", fontSize: "var(--font-size-base)" }}>Description</p>
</div>
```

## Metric Card (2-column grid)

```tsx
<div style={{ display: "grid", gridTemplateColumns: "1fr 1fr", gap: "var(--space-3)" }}>
  <div className="panel-card">
    <div style={{ fontSize: "var(--font-size-sm)", color: "var(--text-secondary)" }}>Total</div>
    <div style={{ fontSize: "var(--font-size-2xl)", fontWeight: "var(--font-bold)" }}>142</div>
  </div>
  <div className="panel-card">
    <div style={{ fontSize: "var(--font-size-sm)", color: "var(--text-secondary)" }}>Passing</div>
    <div style={{ fontSize: "var(--font-size-2xl)", fontWeight: "var(--font-bold)", color: "var(--success-color)" }}>139</div>
  </div>
</div>
```

---

## Surface Hierarchy

```
.panel-container  →  bg-primary   (page)
.panel-body       →  bg-primary   (panel body)
.panel-card       →  bg-secondary (card surface)
nested card       →  bg-tertiary  (inset area)
```

---

## Rules

- Wrap every distinct data entity in its own `.panel-card`
- Don't nest `.panel-card` more than one level deep
- Don't apply `overflow: hidden` to cards — content should reflow, not clip
