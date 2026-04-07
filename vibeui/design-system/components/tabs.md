# Tabs

VibeUI has two tab systems: **panel tabs** (inside a panel, compact underline style) and **TabbedPanel** (the composite panel system). Both use the same CSS foundation.

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

## TabbedPanel (Composite System)

`TabbedPanel` is the outer composite system used by `createComposite`. It renders the outer tab bar for composite panels (Metrics | AST Edit | Predict | …) and keeps sub-panels alive when switching.

```tsx
// TabbedPanel tab bar CSS (rendered by TabbedPanel.tsx)
// Tab bar: display:flex, gap:2, border-bottom, bg-secondary, overflow-x:auto
// Tab button: padding 8px 14px, border-bottom 2px solid transparent/accent-color
// Active: border-bottom-color: --accent-color, color: --accent-color
```

Do not replicate `TabbedPanel` manually — use `createComposite` instead.

---

## Rules

### ✅ Do
- Use `panel-tab-bar` + `panel-tab` for all panel sub-tab navigation
- Add `.active` class dynamically based on state
- Place the tab bar between the header and body
- Use `border: "none"` override when embedding in header
- Use `panel-btn` toggle pairs for 2-option choices in header

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
