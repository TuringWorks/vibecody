---
layout: page
title: Button — Design System
permalink: /design-system/components/button/
---

{% raw %}
# Button

VibeUI has two button systems: **app-level buttons** (`btn-primary`, `btn-secondary`, `icon-button`) for the main shell UI, and **panel buttons** (`panel-btn` + modifiers) for use inside panels. Always use panel buttons inside `panel-container`.

---

## Button Systems

### App-Level Buttons (`btn-primary`, `btn-secondary`)
Used in the app shell (header, sidebar, editor toolbar). More prominent — elevation, hover animation, glow.

### Panel Buttons (`panel-btn` + modifier)
Used inside panels. Compact, opacity-based transitions, no elevation. Always combine a base + variant + optional size.

### Icon Button (`icon-button`, `btn-icon`)
Transparent, icon-only. Used in header row, toolbars.

---

## Panel Buttons

### Base + Variant (required combination)

```tsx
// Primary — the one main action per section
<button className="panel-btn panel-btn-primary">Save Changes</button>

// Secondary — alternate, cancel, navigate
<button className="panel-btn panel-btn-secondary">Cancel</button>

// Danger — destructive (delete, reject, reset)
<button className="panel-btn panel-btn-danger">Delete</button>
```

### Size modifiers (optional — default is medium)

```tsx
// Extra small — inside dense rows, table cells
<button className="panel-btn panel-btn-primary panel-btn-xs">Apply</button>

// Small — in panel headers, compact cards
<button className="panel-btn panel-btn-primary panel-btn-sm">Run</button>

// Default (no size class) — standalone card actions
<button className="panel-btn panel-btn-primary">Generate Plan</button>

// Large — prominent standalone CTA
<button className="panel-btn panel-btn-primary panel-btn-lg">Scan Workspace</button>
```

### Size reference

```
Variant     Padding          Font size    Height approx
panel-btn-xs  2px 8px         10px         22px
panel-btn-sm  4px 10px        11px         26px
(default)     5px 12px        12px         28px
panel-btn-lg  8px 18px        13px         34px
```

---

## CSS Definitions

```css
.panel-btn {
  padding: 5px 12px;
  border: none;
  border-radius: var(--radius-sm);     /* 6px */
  cursor: pointer;
  font-size: 12px;
  font-weight: 500;
  font-family: inherit;
  transition: opacity var(--transition-fast), background var(--transition-fast);
}
.panel-btn:hover    { opacity: 0.85; }
.panel-btn:disabled { opacity: 0.5; cursor: not-allowed; }

.panel-btn-primary   { background: var(--accent-color); color: var(--btn-primary-fg); }
.panel-btn-secondary { background: var(--bg-tertiary); color: var(--text-primary); border: 1px solid var(--border-color); }
.panel-btn-danger    { background: var(--error-color); color: var(--btn-error-fg); }

.panel-btn-xs { padding: 2px 8px;  font-size: 10px; border-radius: var(--radius-xs); }
.panel-btn-sm { padding: 4px 10px; font-size: 11px; }
.panel-btn-lg { padding: 8px 18px; font-size: 13px; }
```

---

## States

### Loading state

Always disable the button during async work and update the label:

```tsx
<button
  className="panel-btn panel-btn-primary"
  onClick={handleScan}
  disabled={loading}
>
  {loading ? "Scanning…" : metrics ? "↻ Re-scan" : "Scan"}
</button>
```

Labels for loading: `"Loading…"`, `"Scanning…"`, `"Saving…"`, `"Analyzing…"` — always trailing `…` (not `...`).

### Disabled state (non-loading)

```tsx
<button
  className="panel-btn panel-btn-primary"
  onClick={handleSubmit}
  disabled={!value || loading}
>
  Submit
</button>
```

The `disabled` attribute automatically applies `opacity: 0.5; cursor: not-allowed` via CSS.

### Danger with confirmation

For destructive actions, make the danger intent clear in both the button text and context:

```tsx
<button
  className="panel-btn panel-btn-danger panel-btn-sm"
  onClick={() => handleDelete(item.id)}
>
  Delete
</button>
```

---

## Button Groups

```tsx
{/* Inline group — gap: 6px */}
<div style={{ display: "flex", gap: 6 }}>
  <button className="panel-btn panel-btn-secondary">Reject</button>
  <button className="panel-btn panel-btn-primary">Accept</button>
</div>

{/* Header group — pushed to right */}
<div className="panel-header">
  <h3>Panel</h3>
  <div style={{ marginLeft: "auto", display: "flex", gap: 6 }}>
    <button className="panel-btn panel-btn-secondary panel-btn-sm">Export</button>
    <button className="panel-btn panel-btn-primary panel-btn-sm">↻ Refresh</button>
  </div>
</div>

{/* Tab-switcher buttons in header */}
{(["scan", "remediate"] as Tab[]).map(t => (
  <button
    key={t}
    className={`panel-btn ${activeTab === t ? "panel-btn-primary" : "panel-btn-secondary"}`}
    onClick={() => setActiveTab(t)}
  >
    {t === "scan" ? "Scan" : "Remediate"}
  </button>
))}
```

---

## Icon Buttons (app shell)

```tsx
// In headers, toolbars
<button className="icon-button" title="Close">
  <Icon name="x" size={14} />
</button>

// btn-icon is an alias for the same thing
<button className="btn-icon" title="Settings">
  <Icon name="settings" size={16} />
</button>
```

CSS: transparent background, `--text-secondary`, hover → `--bg-tertiary` + `--text-primary`, active → `scale(0.95)`.

---

## App-Level Buttons (shell use only)

```tsx
// Primary CTA — prominent, elevation + glow on hover
<button className="btn-primary">
  <Icon name="plus" size={14} /> New Project
</button>

// Secondary — bg-tertiary, border, same hover animation
<button className="btn-secondary">Cancel</button>
```

Do not use `btn-primary` / `btn-secondary` inside panel bodies. They're too large and prominent for compact panel UI.

---

## Rules

### ✅ Do
- Always pair `panel-btn` with a variant (`panel-btn-primary`, etc.)
- Add `disabled={loading}` during async operations
- Show loading progress in button text (`"Scanning…"`)
- Use `panel-btn-sm` in panel headers
- Use `panel-btn-xs` in dense table rows or prediction cards
- Limit to one `panel-btn-primary` per visible section

### ❌ Don't
```tsx
// Missing variant — has no color
<button className="panel-btn">Submit</button>

// Inline button style — use classes
<button style={{ padding: "6px 14px", background: "var(--accent-color)", color: "#fff", border: "none", borderRadius: 4, cursor: "pointer" }}>

// Using btn-primary inside a panel
<button className="btn-primary">Action</button>

// Loading state without feedback
<button onClick={run} disabled={loading}>Run</button>  // no label change
```
{% endraw %}
