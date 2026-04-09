---
render_with_liquid: false
layout: page
title: Tabs — Design System
permalink: /design-system/components/tabs/
---

# Tabs

VibeUI has **three tab levels** with a clear visual hierarchy:

| Level | Class | Font | Padding | Use case |
|---|---|---|---|---|
| **Primary nav** | `panel-tab-bar panel-tab-bar--primary` | 13px | 7px 16px | Composite outer nav (Dashboard/Agent/…) |
| **Sub-tabs** | `panel-tab-bar` | 12px | 6px 14px | Within-panel section switching (Tasks/Lessons/Rules) |
| **Header toggle** | `panel-btn` pairs | — | — | 2–3 options in a panel header |

The size difference (13px vs 12px) and padding (7px vs 6px) signal hierarchy when two tab bars stack vertically.

---

## Panel Tab Bar

```tsx
<div className="panel-tab-bar">
  <button className={`panel-tab ${active === "scan" ? "active" : ""}`} onClick={() => setActive("scan")}>
    Scan
  </button>
  <button className={`panel-tab ${active === "remediate" ? "active" : ""}`} onClick={() => setActive("remediate")}>
    Remediate
  </button>
</div>
```

```css
.panel-tab-bar {
  display: flex;
  gap: 0;
  border-bottom: 1px solid var(--border-color);
}

.panel-tab {
  padding: 6px 14px;
  cursor: pointer;
  font-size: 12px;
  font-weight: 500;
  border: none;
  border-bottom: 2px solid transparent;
  color: var(--text-secondary);
  background: none;
  font-family: inherit;
  transition: all var(--transition-fast);
}
.panel-tab:hover { color: var(--text-primary); }
.panel-tab.active {
  color: var(--accent-color);
  border-bottom-color: var(--accent-color);
}
```

---

## Placement

### Between header and body (most common)

Sticky below the header, above the scrollable body. Tabs do not scroll.

```tsx
<div className="panel-container">
  <div className="panel-header">
    <h3>Health Score</h3>
    {/* header controls */}
  </div>

  {/* Tab bar — NOT inside panel-body */}
  <div className="panel-tab-bar">
    <button className={`panel-tab ${tab === "scan" ? "active" : ""}`} onClick={() => setTab("scan")}>Scan</button>
    <button className={`panel-tab ${tab === "remediate" ? "active" : ""}`} onClick={() => setTab("remediate")}>Remediate</button>
  </div>

  <div className="panel-body">
    {/* tab content */}
  </div>
</div>
```

### Inside the header (compact — tab-switcher style)

When tabs are the primary navigation and panel is simple:

```tsx
<div className="panel-header">
  <div className="panel-tab-bar" style={{ flex: 1, border: "none" }}>
    <button className={`panel-tab ${tab === "predictions" ? "active" : ""}`} onClick={() => setTab("predictions")}>
      Predictions
    </button>
    <button className={`panel-tab ${tab === "patterns" ? "active" : ""}`} onClick={() => setTab("patterns")}>
      Patterns
    </button>
    <button className={`panel-tab ${tab === "model" ? "active" : ""}`} onClick={() => setTab("model")}>
      Model
    </button>
  </div>
  {/* right-side controls after tabs */}
  <span className="panel-tag panel-tag-neutral">92% accept</span>
</div>
```

Override `border: "none"` on `.panel-tab-bar` when embedded in header (the header already has a `border-bottom`).

---

## Tab with Count

```tsx
<button className={`panel-tab ${tab === "languages" ? "active" : ""}`} onClick={() => setTab("languages")}>
  Languages ({metrics.languages.length})
</button>
```

---

## Controlled Tabs — Full Pattern

```tsx
type Tab = "plan" | "suggest" | "history";

const TABS: { id: Tab; label: string }[] = [
  { id: "plan",    label: "Plan" },
  { id: "suggest", label: "Suggest" },
  { id: "history", label: "History" },
];

const [tab, setTab] = useState<Tab>("plan");

// In render:
<div className="panel-tab-bar">
  {TABS.map(t => (
    <button
      key={t.id}
      className={`panel-tab ${tab === t.id ? "active" : ""}`}
      onClick={() => setTab(t.id)}
    >
      {t.label}
    </button>
  ))}
</div>
```

---

## Tab Button Switcher (not underline style)

When you want pill-style toggle buttons instead of underlines — use `panel-btn` pairs:

```tsx
<div style={{ display: "flex", gap: 6 }}>
  {(["scan", "remediate"] as Tab[]).map(t => (
    <button
      key={t}
      className={`panel-btn ${tab === t ? "panel-btn-primary" : "panel-btn-secondary"}`}
      onClick={() => setTab(t)}
    >
      {t === "scan" ? "Scan" : "Remediate"}
    </button>
  ))}
</div>
```

Use this in the **header** when there are only 2–3 options and switching immediately changes the view. Prefer `panel-tab-bar` with underline when there are 3+ tabs.

---

## Primary Nav — Composite System

`TabbedPanel` renders the outer nav for composite panels using `panel-tab-bar--primary`. It is larger and heavier than sub-tabs to signal top-level navigation.

```tsx
// TabbedPanel.tsx (managed by createComposite — do not replicate manually)
<div className="panel-tab-bar panel-tab-bar--primary" style={{ overflowX: "auto" }}>
  <button className={`panel-tab ${active === t.id ? "active" : ""}`}>…</button>
</div>
```

```css
.panel-tab-bar--primary .panel-tab {
  font-size: 13px;
  padding: 7px 16px;
  letter-spacing: 0.01em;
}
```

Do not replicate `TabbedPanel` manually — use `createComposite` instead.

---

## Section Titles

Use `.panel-section-title` for section headings inside `panel-body` instead of raw `<h3>` with inline styles.

```tsx
<h3 className="panel-section-title">New Task</h3>
// With non-standard margin: add only the override
<h3 className="panel-section-title" style={{ marginBottom: "12px" }}>Rules</h3>
```

```css
.panel-section-title {
  font-size: 13px;
  font-weight: 600;
  color: var(--text-primary);
  margin: 0 0 8px;
}
```

---

## Rules

### ✅ Do
- Use `panel-tab-bar panel-tab-bar--primary` for composite outer nav (`TabbedPanel`)
- Use `panel-tab-bar` (no modifier) for within-panel sub-tabs
- Add `.active` class dynamically based on state
- Place the tab bar between the header and body
- Use `border: "none"` override when embedding in header
- Use `panel-btn` toggle pairs for 2-option choices in header
- Use `panel-section-title` for `<h3>` headings inside `panel-body`

### ❌ Don't
```tsx
// Inline tab button styles
<button style={{
  padding: "6px 14px", fontSize: 11, fontWeight: active ? 600 : 400,
  background: active ? "color-mix(...)" : "transparent",
  border: "1px solid " + (active ? "var(--accent-primary)" : "var(--border-color)"),
  borderRadius: 4, color: active ? "var(--text-info)" : "var(--text-secondary)", cursor: "pointer"
}}>
// → use className={`panel-tab ${active ? "active" : ""}`}

// Border still showing when tab bar is in header
<div className="panel-tab-bar">  // in header — add style={{ border: "none" }}
```
