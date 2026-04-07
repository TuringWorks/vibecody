---
layout: page
title: Progress — Design System
permalink: /design-system/components/progress/
---

# Progress

Progress bars for loading, scores, and quota display.

---

## Basic Progress Bar

```tsx
<div className="progress-bar">
  <div className="progress-bar-fill" style={{ width: `${pct}%` }} />
</div>
```

## Semantic Color Variants

```tsx
// Success (green) — scores 80+
<div className="progress-bar">
  <div className="progress-bar-fill progress-bar-success" style={{ width: "85%" }} />
</div>

// Warning (gold) — scores 60–79
<div className="progress-bar">
  <div className="progress-bar-fill progress-bar-warning" style={{ width: "65%" }} />
</div>

// Danger (red) — scores <60
<div className="progress-bar">
  <div className="progress-bar-fill progress-bar-danger" style={{ width: "40%" }} />
</div>

// Info (blue) — neutral progress
<div className="progress-bar">
  <div className="progress-bar-fill progress-bar-info" style={{ width: "30%" }} />
</div>
```

## Dynamic Color Based on Score

```tsx
const barClass = (score: number) =>
  score >= 80 ? "progress-bar-success"
  : score >= 60 ? "progress-bar-warning"
  : "progress-bar-danger";

<div className="progress-bar">
  <div className={`progress-bar-fill ${barClass(score)}`} style={{ width: `${score}%` }} />
</div>
```

---

## Sizes

```tsx
// Default (4px height)
<div className="progress-bar">...</div>

// Thin (2px)
<div className="progress-bar progress-bar-thin">...</div>

// Thick (8px)
<div className="progress-bar progress-bar-thick">...</div>
```
