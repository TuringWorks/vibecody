# VibeUI Panel UI/UX Guidelines

Design guidelines for building and improving panels in VibeUI. Follow these to ensure visual consistency across all 196+ panels.

---

## 1. Panel Structure

Every panel **must** use this flex-column skeleton. This is the only layout that works correctly inside Tauri's WebKit renderer.

```tsx
// ✅ Correct — flex: 1 fills the flex parent properly
<div className="panel-container">           {/* flex col, overflow: hidden */}
  <div className="panel-header">            {/* flex row, border-bottom, flex-shrink: 0 */}
    <h3>Panel Title</h3>
    <div style={{ marginLeft: "auto" }}>    {/* right-side controls */}
      <button className="panel-btn panel-btn-primary">Action</button>
    </div>
  </div>
  <div className="panel-body">              {/* flex: 1, overflow-y: auto, padding: 8px 12px */}
    {/* scrollable content */}
  </div>
  {/* optional: */}
  <div className="panel-footer">            {/* border-top, flex-shrink: 0 */}
    {/* footer actions */}
  </div>
</div>
```

### ⚠️ Critical height rules

| Pattern | Status | Reason |
|---------|--------|--------|
| `height: "100%"` on root | ❌ Never | Doesn't resolve against flex parent in WebKit |
| `height: "100%"` on root + `overflow: auto` | ❌ Never | Same issue — content collapses to 0 |
| `flex: 1; minHeight: 0` on root | ✅ Use | Properly fills flex parent |
| `className="panel-container"` | ✅ Preferred | Encodes the correct pattern |

If a panel **must** use inline styles (e.g. has no header), use:
```tsx
<div style={{ display: "flex", flexDirection: "column", flex: 1, minHeight: 0, overflow: "hidden" }}>
```

---

## 2. Design Tokens

Always use CSS variables. Never hardcode colors, spacing, or radii.

### Colors

```css
/* Backgrounds */
var(--bg-primary)          /* darkest — page bg */
var(--bg-secondary)        /* cards, panels */
var(--bg-tertiary)         /* inputs, subtle hover */
var(--bg-elevated)         /* modals, dropdowns */

/* Text */
var(--text-primary)        /* body text */
var(--text-secondary)      /* labels, captions */
var(--text-muted)          /* placeholders, very low priority */

/* Semantic */
var(--success-color)       /* ✅ green */
var(--error-color)         /* ❌ red */
var(--warning-color)       /* ⚠️ gold/amber */
var(--info-color)          /* ℹ️ blue */
var(--accent-color)        /* primary accent (blue) */

/* Semantic text */
var(--text-success)        /* green text */
var(--text-danger)         /* red text */
var(--text-warning)        /* gold text */
var(--text-info)           /* blue text */

/* Semantic backgrounds (10% opacity) */
var(--success-bg)
var(--error-bg)
var(--warning-bg)
var(--info-bg)
var(--accent-bg)

/* Borders */
var(--border-color)        /* standard borders */
var(--border-subtle)       /* very light dividers */

/* Accents */
var(--accent-blue), var(--accent-green), var(--accent-purple)
var(--accent-gold), var(--accent-rose)
```

### ❌ Hardcoded colors — never use these

```css
/* Replace ALL of these with CSS vars */
#4caf50  →  var(--success-color)   or  var(--text-success)
#f44336  →  var(--error-color)     or  var(--text-danger)
#ff9800  →  var(--warning-color)   or  var(--text-warning)
#2196f3  →  var(--info-color)      or  var(--text-info)
#ef4444  →  var(--error-color)
"white"  →  var(--btn-primary-fg)
"#fff"   →  var(--btn-primary-fg)
```

### Spacing

```css
--space-1: 4px     /* xs — icon gaps, tight spacing */
--space-2: 8px     /* sm — between related elements */
--space-3: 12px    /* md — card padding, row gaps */
--space-4: 16px    /* lg — section padding */
--space-5: 20px    /* xl — content padding */
--space-6: 24px    /* 2xl — large section spacing */
--space-8: 32px    /* 3xl — empty state padding */
```

### Font Sizes

```css
--font-size-xs:   10px   /* timestamps, badges, row numbers */
--font-size-sm:   11px   /* labels, secondary info */
--font-size-base: 12px   /* default panel body text */
--font-size-md:   13px   /* primary content text */
--font-size-lg:   14px   /* section headings */
--font-size-xl:   15px   /* panel headings */
--font-size-2xl:  18px   /* key metric values */
--font-size-3xl:  24px   /* large stat numbers */
```

### Border Radius

```css
--radius-xs: 3px    /* tags, small badges */
--radius-sm: 6px    /* buttons, cards, inputs */
--radius-md: 10px   /* larger cards */
--radius-lg: 14px   /* panels, modals */
--radius-xl: 20px   /* pill badges */
```

### Transitions

```css
--transition-fast:   0.15s cubic-bezier(0.4, 0, 0.2, 1)
--transition-smooth: 0.25s cubic-bezier(0.4, 0, 0.2, 1)
--transition-spring: 0.35s cubic-bezier(0.34, 1.56, 0.64, 1)
```

---

## 3. CSS Utility Classes

Use these instead of inline styles. See `App.css` for full definitions.

### Panel Structure

```tsx
<div className="panel-container">   /* flex col, height fill, overflow hidden */
<div className="panel-header">      /* flex row, border-bottom, padding 8 12 */
<div className="panel-body">        /* flex 1, overflow-y auto, padding 8 12 */
<div className="panel-footer">      /* flex row, border-top, padding 8 12 */
<div className="panel-card">        /* bg-secondary card with border + radius */
<div className="panel-section">     /* flex col, gap 10, margin-bottom 12 */
<div className="panel-row">         /* flex row, align-center, gap 8 */
```

### Buttons

Always combine a size + variant:

```tsx
<button className="panel-btn panel-btn-primary">Action</button>
<button className="panel-btn panel-btn-secondary">Cancel</button>
<button className="panel-btn panel-btn-danger">Delete</button>

/* Size variants */
<button className="panel-btn panel-btn-xs panel-btn-secondary">Tiny</button>
<button className="panel-btn panel-btn-sm panel-btn-primary">Small</button>
<button className="panel-btn panel-btn-primary">Default</button>
<button className="panel-btn panel-btn-lg panel-btn-primary">Large</button>
```

### Inputs

```tsx
<input className="panel-input" />
<input className="panel-input panel-input-full" />   /* full width */
<select className="panel-select" />
<textarea className="panel-input panel-input-full panel-textarea" />
```

### States

```tsx
/* Empty state — centered, muted, padding 40px */
<div className="panel-empty">No items found. Try adding one above.</div>

/* Loading state */
<div className="panel-loading">Loading...</div>

/* Error state */
<div className="panel-error">
  {errorMessage}
  <button onClick={clearError}>✕</button>
</div>
```

### Typography

```tsx
<h3 className="panel-heading">Section Title</h3>
<span className="panel-label">Input Label</span>
<span className="panel-value">Primary value</span>
<code className="panel-mono">path/to/file.rs</code>
```

### Badges & Tags

```tsx
<span className="panel-tag panel-tag-info">pending</span>
<span className="panel-tag panel-tag-success">passed</span>
<span className="panel-tag panel-tag-warning">slow</span>
<span className="panel-tag panel-tag-danger">critical</span>

/* Larger badges (with icon support) */
<span className="panel-badge badge-success">3 passed</span>
<span className="panel-badge badge-error">1 failed</span>
```

### Stats & Metrics

```tsx
/* Horizontal stat row */
<div className="panel-stats">
  <div className="panel-stat">
    <div className="panel-stat-value" style={{ color: "var(--text-success)" }}>42</div>
    <div className="panel-stat-label">Tests Passed</div>
  </div>
  <div className="panel-stat">
    <div className="panel-stat-value" style={{ color: "var(--error-color)" }}>3</div>
    <div className="panel-stat-label">Failed</div>
  </div>
</div>

/* 3-column grid */
<div className="panel-stats-grid-3">
  <div className="panel-stat"> ... </div>
</div>
```

### Progress Bars

```tsx
<div className="progress-bar">
  <div className="progress-bar-fill progress-bar-success" style={{ width: "72%" }} />
</div>

/* Dynamic color based on value */
<div className="progress-bar">
  <div
    className={`progress-bar-fill ${score > 80 ? "progress-bar-success" : score > 60 ? "progress-bar-warning" : "progress-bar-danger"}`}
    style={{ width: `${score}%` }}
  />
</div>
```

### Tables

```tsx
<table className="panel-table">
  <thead>
    <tr>
      <th>Name</th>
      <th>Value</th>
    </tr>
  </thead>
  <tbody>
    <tr>
      <td>...</td>
      <td>...</td>
    </tr>
  </tbody>
</table>
```

### Tab Bars

```tsx
<div className="panel-tab-bar">
  <button className={`panel-tab ${active === "a" ? "active" : ""}`} onClick={() => setActive("a")}>Tab A</button>
  <button className={`panel-tab ${active === "b" ? "active" : ""}`} onClick={() => setActive("b")}>Tab B</button>
</div>
```

### Dividers

```tsx
<div className="panel-divider" />
```

---

## 4. Standard Component Patterns

### Loading + Error + Empty (the three states)

**Always handle all three states for any async data:**

```tsx
{loading && <div className="panel-loading">Loading...</div>}
{error && (
  <div className="panel-error">
    {error}
    <button onClick={() => setError(null)}>✕</button>
  </div>
)}
{!loading && !error && items.length === 0 && (
  <div className="panel-empty">No items found. {/* helpful action hint */}</div>
)}
```

### Scan/Refresh Button with Loading State

```tsx
<button
  className="panel-btn panel-btn-primary"
  onClick={handleScan}
  disabled={loading}
>
  {loading ? "Scanning…" : metrics ? "↻ Re-scan" : "Scan"}
</button>
```

### Form Card (input + action)

```tsx
<div className="panel-card" style={{ marginBottom: 10 }}>
  <div className="panel-label">Intent</div>
  <input
    className="panel-input panel-input-full"
    value={value}
    onChange={e => setValue(e.target.value)}
    placeholder="Describe your goal..."
    style={{ marginBottom: 8 }}
  />
  <button
    className="panel-btn panel-btn-primary"
    onClick={handleSubmit}
    disabled={loading || !value}
  >
    {loading ? "…" : "Submit"}
  </button>
</div>
```

### Metric Row in a Card

```tsx
<div className="panel-card" style={{ marginBottom: 8 }}>
  <div className="panel-row" style={{ marginBottom: 6 }}>
    <span style={{ fontWeight: 600, fontSize: 12 }}>{d.name}</span>
    <span style={{ marginLeft: "auto", color: scoreColor(d.score), fontWeight: 600 }}>
      {d.score}/100
    </span>
  </div>
  <div className="progress-bar">
    <div
      className="progress-bar-fill"
      style={{ width: `${d.score}%`, background: scoreColor(d.score) }}
    />
  </div>
  <div className="panel-label" style={{ marginTop: 6 }}>{d.details}</div>
</div>
```

### Confidence Bar (for AI predictions)

```tsx
const confColor = (c: number) =>
  c > 0.85 ? "var(--text-success)" : c > 0.7 ? "var(--text-warning)" : "var(--text-danger)";

<div style={{ display: "flex", alignItems: "center", gap: 8 }}>
  <div className="progress-bar" style={{ flex: 1 }}>
    <div
      className="progress-bar-fill"
      style={{ width: `${c * 100}%`, background: confColor(c) }}
    />
  </div>
  <span style={{ fontSize: 10, color: confColor(c), fontWeight: 600, minWidth: 32 }}>
    {(c * 100).toFixed(0)}%
  </span>
</div>
```

### Status badge for step/item status

```tsx
const statusTag = (status: string) => {
  if (status.includes("Completed") || status === "passed") return "panel-tag panel-tag-success";
  if (status.includes("InProgress") || status === "running") return "panel-tag panel-tag-info";
  if (status.includes("Failed") || status === "failed") return "panel-tag panel-tag-danger";
  return "panel-tag";
};
<span className={statusTag(s.status)}>{s.status}</span>
```

---

## 5. Anti-Patterns — Never Do These

### Layout

```tsx
// ❌ — height:100% collapses in WebKit flex contexts
<div style={{ height: "100%", overflow: "auto" }}>

// ❌ — same problem in a flex parent
<div style={{ height: "100%", display: "flex", flexDirection: "column" }}>

// ✅ — always use flex:1 for flex children
<div style={{ flex: 1, minHeight: 0, display: "flex", flexDirection: "column" }}>
// or just:
<div className="panel-container">
```

### Colors

```tsx
// ❌ — hardcoded colors break themes
color: "#4caf50"
color: "#f44336"
background: "white"
color: "#fff"
borderLeft: "3px solid #4caf50"

// ✅ — always CSS vars
color: "var(--success-color)"
color: "var(--error-color)"
background: "var(--btn-primary-fg)"
borderLeft: `3px solid var(--success-color)`
```

### Inline style objects

```tsx
// ❌ — per-panel style object, repeated in every file
const cardStyle: React.CSSProperties = {
  background: "var(--bg-secondary)", borderRadius: 6, padding: 12,
  marginBottom: 10, border: "1px solid var(--border-color)"
};
const btnStyle: React.CSSProperties = { padding: "6px 14px", ... };

// ✅ — use CSS classes
<div className="panel-card" style={{ marginBottom: 8 }}>
<button className="panel-btn panel-btn-primary">
```

### Magic spacing numbers

```tsx
// ❌ — magic numbers
padding: 24
gap: 14
marginBottom: 10

// ✅ — use tokens or stick to 4px grid (4, 8, 12, 16, 20, 24, 32)
padding: "var(--space-6)"   // 24px
gap: "var(--space-2)"       // 8px  
marginBottom: 10             // ok if there's no clean token match
```

### Inconsistent empty states

```tsx
// ❌ — varies every panel
<div style={{ padding: "16px", color: "...", fontSize: "11px" }}>
  No data.
</div>

// ✅ — always use panel-empty
<div className="panel-empty">No data yet. Try adding some above.</div>
```

### Missing disabled/loading states

```tsx
// ❌ — button always enabled, no feedback
<button onClick={fetchData}>Load</button>

// ✅ — always disable during loading, show feedback
<button
  className="panel-btn panel-btn-primary"
  onClick={fetchData}
  disabled={loading}
>
  {loading ? "Loading…" : "Load"}
</button>
```

---

## 6. Inline Style Rules

Use **inline styles only** for dynamic values (computed from data/state). Everything static should be a class.

```tsx
// ✅ Dynamic color based on data — inline is correct
style={{ color: scoreColor(d.score) }}
style={{ width: `${percent}%` }}
style={{ opacity: isActive ? 1 : 0.5 }}
style={{ marginLeft: "auto" }}     // layout trick, ok inline

// ❌ Static value — use a class instead  
style={{ fontSize: 12, color: "var(--text-secondary)" }}  // → className="panel-label"
style={{ background: "var(--bg-secondary)", borderRadius: 6, padding: 12, border: "1px solid var(--border-color)" }}  // → className="panel-card"
style={{ padding: "6px 14px", borderRadius: 4, border: "none", background: "var(--accent-color)", color: "#fff", cursor: "pointer", fontSize: 12 }}  // → className="panel-btn panel-btn-primary"
```

---

## 7. Code Example — Well-Structured Panel

```tsx
import { useState, useEffect, useCallback } from "react";
import { invoke } from "@tauri-apps/api/core";

interface Item { id: string; name: string; status: string; score: number; }

export default function ExamplePanel() {
  const [items, setItems] = useState<Item[]>([]);
  const [loading, setLoading] = useState(false);
  const [error, setError] = useState<string | null>(null);

  const load = useCallback(async () => {
    setLoading(true);
    setError(null);
    try {
      const result = await invoke<Item[]>("get_items");
      setItems(result);
    } catch (e) {
      setError(String(e));
    } finally {
      setLoading(false);
    }
  }, []);

  useEffect(() => { load(); }, [load]);

  const scoreColor = (s: number) =>
    s >= 80 ? "var(--success-color)" : s >= 60 ? "var(--warning-color)" : "var(--error-color)";

  return (
    <div className="panel-container">
      <div className="panel-header">
        <h3>Example Panel</h3>
        <div style={{ marginLeft: "auto", display: "flex", gap: 8, alignItems: "center" }}>
          <span className="panel-tag panel-tag-info">{items.length} items</span>
          <button
            className="panel-btn panel-btn-primary panel-btn-sm"
            onClick={load}
            disabled={loading}
          >
            {loading ? "Loading…" : "↻ Refresh"}
          </button>
        </div>
      </div>

      <div className="panel-body">
        {loading && <div className="panel-loading">Loading items…</div>}

        {error && (
          <div className="panel-error">
            {error}
            <button onClick={() => setError(null)}>✕</button>
          </div>
        )}

        {!loading && !error && items.length === 0 && (
          <div className="panel-empty">No items yet. Click Refresh to load.</div>
        )}

        {items.map(item => (
          <div key={item.id} className="panel-card" style={{ marginBottom: 8 }}>
            <div className="panel-row" style={{ marginBottom: 6 }}>
              <span style={{ fontWeight: 600, fontSize: 13 }}>{item.name}</span>
              <span className={`panel-tag panel-tag-${item.status === "ok" ? "success" : "warning"}`}
                style={{ marginLeft: 8 }}>
                {item.status}
              </span>
              <span style={{ marginLeft: "auto", color: scoreColor(item.score), fontWeight: 600 }}>
                {item.score}/100
              </span>
            </div>
            <div className="progress-bar">
              <div
                className="progress-bar-fill"
                style={{ width: `${item.score}%`, background: scoreColor(item.score) }}
              />
            </div>
          </div>
        ))}
      </div>
    </div>
  );
}
```

---

## 8. Checklist for New Panels

Before shipping a panel, verify:

- [ ] Root uses `className="panel-container"` (not `height: "100%"`)
- [ ] Scrollable content is in `className="panel-body"` (not a div with `overflow: auto`)
- [ ] Header uses `className="panel-header"` with `<h3>` title
- [ ] All three states handled: loading (`.panel-loading`), error (`.panel-error`), empty (`.panel-empty`)
- [ ] Buttons use `className="panel-btn panel-btn-{variant}"` 
- [ ] No hardcoded colors (`#4caf50`, `#f44336`, `#ff9800`, `#2196f3`, etc.)
- [ ] No `panelStyle`, `headingStyle`, `cardStyle`, `btnStyle` objects — use classes
- [ ] Disabled state on all buttons during loading (`disabled={loading}`)
- [ ] Loading button text shows progress (`"Loading…"` not `"..."`)
- [ ] Cards use `className="panel-card"`
- [ ] Progress bars use `.progress-bar` + `.progress-bar-fill` + `.progress-bar-{color}`
- [ ] Font family is inherited (don't set `fontFamily: "var(--font-family)"` on root)
- [ ] `fontSize: 13` is inherited from `.panel-container` (don't re-set on root)
