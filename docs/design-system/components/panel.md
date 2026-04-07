---
layout: page
title: Panel — Design System
permalink: /design-system/components/panel/
---

# Panel

The core layout primitive. Every panel in VibeUI uses this structure.

---

## Structure

```tsx
<div className="panel-container">     // flex:1, minHeight:0, overflow:hidden
  <div className="panel-header">      // sticky top bar
    <h3>Panel Title</h3>
    <button className="panel-btn panel-btn-primary panel-btn-sm">Action</button>
  </div>

  <div className="panel-body">        // flex:1, overflow-y:auto, padding:12px
    {/* content here */}
  </div>

  <div className="panel-footer">      // optional bottom bar
    {/* footer actions */}
  </div>
</div>
```

---

## Rules

1. **Root**: `flex: 1; min-height: 0` — never `height: 100%`
2. **Overflow**: only `.panel-body` scrolls — container and header never scroll
3. **Header**: always has an `<h3>` title; actions go right-aligned with `marginLeft: "auto"`
4. **Footer**: optional; use for submit buttons or pagination

---

## Async States

Every panel that loads data must handle all three states:

```tsx
{loading && <div className="panel-loading">Loading…</div>}
{error   && <div className="panel-error">{error}</div>}
{!loading && !error && items.length === 0 && (
  <div className="panel-empty">No data yet.</div>
)}
```

| Class | Purpose |
|---|---|
| `.panel-loading` | Spinner + message |
| `.panel-error` | Red tinted error message |
| `.panel-empty` | Centered empty state |

---

## Tabs

```tsx
<div className="panel-tab-bar">
  <button className={`panel-tab ${tab === "a" ? "active" : ""}`} onClick={() => setTab("a")}>
    Tab A
  </button>
  <button className={`panel-tab ${tab === "b" ? "active" : ""}`} onClick={() => setTab("b")}>
    Tab B
  </button>
</div>
```

---

## Section Headers

```tsx
<div className="panel-section-title">Section Name</div>
```
