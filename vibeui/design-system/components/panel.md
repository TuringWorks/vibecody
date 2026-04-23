# Panel

The Panel is the core layout primitive in VibeUI. Every panel component must follow this structure. It ensures correct height filling inside Tauri's WebKit renderer and consistent visual framing.

---

## Structure

```
┌─────────────────────────────────────────────────────┐
│ .panel-container                                    │
│   display: flex; flex-direction: column;            │
│   flex: 1; min-height: 0; overflow: hidden          │
│                                                     │
│ ┌─────────────────────────────────────────────────┐ │
│ │ .panel-header                                   │ │
│ │   padding: 8px 12px                             │ │
│ │   border-bottom: 1px solid --border-color       │ │
│ │   display: flex; align-items: center; gap: 8px  │ │
│ │   flex-shrink: 0                                │ │
│ └─────────────────────────────────────────────────┘ │
│                                                     │
│ ┌─────────────────────────────────────────────────┐ │
│ │ .panel-body                                     │ │
│ │   flex: 1; overflow-y: auto; padding: 8px 12px  │ │
│ │   (this is the scrollable zone)                 │ │
│ └─────────────────────────────────────────────────┘ │
│                                                     │
│ ┌─────────────────────────────────────────────────┐ │
│ │ .panel-footer  (optional)                       │ │
│ │   padding: 8px 12px                             │ │
│ │   border-top: 1px solid --border-color          │ │
│ │   flex-shrink: 0                                │ │
│ └─────────────────────────────────────────────────┘ │
└─────────────────────────────────────────────────────┘
```

---

## CSS Class Definitions

```css
.panel-container {
  display: flex;
  flex-direction: column;
  height: 100%;          /* resolves against parent block height */
  overflow: hidden;
  color: var(--text-primary);
  font-size: 13px;
}

.panel-header {
  padding: 8px 12px;
  border-bottom: 1px solid var(--border-color);
  display: flex;
  align-items: center;
  gap: 8px;
}

.panel-header h3 {
  font-size: 14px;
  font-weight: 700;
  margin: 0;
}

.panel-body {
  flex: 1;
  overflow-y: auto;
  padding: 8px 12px;
}

.panel-footer {
  padding: 8px 12px;
  border-top: 1px solid var(--border-color);
  flex-shrink: 0;
  display: flex;
  align-items: center;
  gap: 8px;
}
```

---

## The Height Rule

**The single most important rule in VibeUI panels:**

```tsx
// ✅ CORRECT — fills flex parent properly
<div className="panel-container">

// ✅ CORRECT — explicit inline equivalent
<div style={{ display: "flex", flexDirection: "column", flex: 1, minHeight: 0, overflow: "hidden" }}>

// ❌ WRONG — height: 100% does NOT resolve against a flex parent in WebKit (Tauri)
<div style={{ height: "100%", display: "flex", flexDirection: "column" }}>

// ❌ WRONG — scrollable wrapper with height: 100%
<div style={{ height: "100%", overflow: "auto", padding: 16 }}>
```

**Why:** Tauri uses WebKit on macOS. When a `flex: 1` parent has no explicit `height` value, `height: 100%` on a child doesn't resolve — it computes as `auto` (zero height when there's no intrinsic content). `flex: 1` + `minHeight: 0` is the correct pattern because it participates in the flex algorithm directly.

---

## Template

```tsx
export default function MyPanel() {
  return (
    <div className="panel-container">

      {/* ── Header ─────────────────────────────────────── */}
      <div className="panel-header">
        <h3>Panel Title</h3>
        {/* Right-side controls */}
        <div style={{ marginLeft: "auto", display: "flex", gap: 6, alignItems: "center" }}>
          <span className="panel-tag panel-tag-info">12 items</span>
          <button className="panel-btn panel-btn-primary panel-btn-sm">Action</button>
        </div>
      </div>

      {/* ── Body (scrollable) ──────────────────────────── */}
      <div className="panel-body">
        {/* content */}
      </div>

      {/* ── Footer (optional) ──────────────────────────── */}
      <div className="panel-footer">
        <span className="panel-label" style={{ marginBottom: 0 }}>Status: ready</span>
        <button className="panel-btn panel-btn-secondary panel-btn-sm" style={{ marginLeft: "auto" }}>
          Save
        </button>
      </div>

    </div>
  );
}
```

---

## Variants

### Panel with sub-tabs

When a panel has internal tabs (e.g. Files / Edits / Preview), place the tab bar between the header and body — it becomes a sticky section below the header:

```tsx
<div className="panel-container">
  <div className="panel-header">
    <h3>AST Edit</h3>
    <span className="panel-tag panel-tag-neutral" style={{ marginLeft: "auto" }}>
      {edits.length} pending
    </span>
  </div>

  {/* Sub-tab bar — NOT in panel-body so it stays sticky */}
  <div className="panel-tab-bar">
    <button className={`panel-tab ${tab === "files" ? "active" : ""}`} onClick={() => setTab("files")}>Files</button>
    <button className={`panel-tab ${tab === "edits" ? "active" : ""}`} onClick={() => setTab("edits")}>Edits</button>
  </div>

  <div className="panel-body">
    {tab === "files" && <FilesView />}
    {tab === "edits" && <EditsView />}
  </div>
</div>
```

### Panel with summary stats

```tsx
<div className="panel-container">
  <div className="panel-header">
    <h3>Code Metrics</h3>
    <button className="panel-btn panel-btn-primary panel-btn-sm" style={{ marginLeft: "auto" }}>
      Scan
    </button>
  </div>

  {/* Summary row — not in body so it's always visible */}
  {metrics && (
    <div className="panel-stats" style={{ padding: "8px 12px", borderBottom: "1px solid var(--border-color)" }}>
      <div className="panel-stat">
        <div className="panel-stat-value">{metrics.total_files}</div>
        <div className="panel-stat-label">Files</div>
      </div>
      <div className="panel-stat">
        <div className="panel-stat-value">{metrics.total_lines.toLocaleString()}</div>
        <div className="panel-stat-label">Lines</div>
      </div>
    </div>
  )}

  <div className="panel-body">
    {/* scrollable detail */}
  </div>
</div>
```

### Header-less panel (body only)

Rare — use only when the panel title comes from the composite tab bar.

```tsx
<div className="panel-container">
  <div className="panel-body">
    {/* content fills entire space */}
  </div>
</div>
```

---

## The Margin Rule (4-sided padding on every panel)

**Every panel MUST inset its scrollable content from the container edges on all four sides.** Content flush against a panel edge reads as unfinished and crowds adjacent panels in multi-pane layouts.

The canonical way to satisfy this is: put content inside `.panel-body`, which already sets `padding: 8px 12px` on all four sides.

```tsx
// ✅ CORRECT — content is inside panel-body, inset on all 4 sides
<div className="panel-container">
  <div className="panel-header">…</div>
  <div className="panel-tab-bar">…</div>   {/* tab-bar spans full width by design */}
  <div className="panel-body">
    {/* all scrollable content lives here */}
    {tab === "a" && renderA()}
    {tab === "b" && renderB()}
  </div>
</div>

// ❌ WRONG — render output becomes a direct sibling of panel-container,
//    first card sits flush against the left/right/bottom edges
<div className="panel-container">
  <div className="panel-header">…</div>
  <div className="panel-tab-bar">…</div>
  {tab === "a" && renderA()}
  {tab === "b" && renderB()}
</div>

// ❌ WRONG — h2 and cards are direct children with no horizontal padding
<div className="panel-container">
  <h2>Dashboard</h2>
  <div className="panel-card">…</div>
</div>
```

Why this is a separate rule even though `.panel-body` already has padding: the `.panel-container > div:last-child` catchall in `App.css` gives unwrapped panels auto-scroll behavior but **does not** add padding. Panels that relied on the catchall ended up flush against the edges. The only way to guarantee 4-sided padding is to wrap content in `.panel-body` (or an equivalent padded wrapper).

Exceptions:
- `.panel-header` / `.panel-tab-bar` / `.panel-footer` are intentionally full-bleed — their border-bottom/top must span edge-to-edge. They each carry their own internal padding (`8px 12px`), so their content (title text, tab labels) stays inset.

---

## Header Anatomy

```tsx
<div className="panel-header">
  {/* Left: title (required) */}
  <h3>Panel Name</h3>

  {/* Middle: optional subtitle or breadcrumb */}
  <span className="panel-label" style={{ marginBottom: 0 }}>subtitle</span>

  {/* Spacer */}
  <div style={{ flex: 1 }} />

  {/* Right: controls (at most 2–3 actions) */}
  <span className="panel-tag panel-tag-info">42 items</span>
  <button className="panel-btn panel-btn-secondary panel-btn-sm">Export</button>
  <button className="panel-btn panel-btn-primary panel-btn-sm">↻ Refresh</button>
</div>
```

Rules for header:
- Always have an `<h3>` title — no exceptions
- Maximum 3 right-side controls
- Use `panel-btn-sm` for header buttons (keeps header compact at 38px)
- Use `panel-tag` for count/status indicators (not full buttons)
- Use `style={{ marginLeft: "auto" }}` on first right-side element to push everything right

---

## Nested Panels (Composites)

When a panel is rendered inside a `TabbedPanel` / `createComposite`, the parent wraps it in a flex column with `flex: 1; minHeight: 0`. The panel's `panel-container` (`height: 100%`) resolves against this. The chain is:

```
App layout div
  └─ flex:1 PanelHost wrapper
      └─ KeepAlivePanel (display:contents)
          └─ TabbedPanel root (height:100%, flex col)
              └─ TabbedPanel content (flex:1, overflow:auto, flex col)
                  └─ Tab pane div (flex:1, minHeight:0, flex col)
                      └─ panel-container ← HERE (height:100% resolves correctly)
                          └─ panel-header (flex-shrink:0)
                          └─ panel-body (flex:1, overflow-y:auto)
```
