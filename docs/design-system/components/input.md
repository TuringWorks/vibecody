---
render_with_liquid: false
layout: page
title: Input — Design System
permalink: /design-system/components/input/
---

# Input

VibeUI provides CSS classes for text inputs, textareas, and selects. All form elements inherit global base styles from `App.css` automatically. The `panel-input` class adds panel-specific refinements.

---

## Text Input

```tsx
{/* Basic */}
<input className="panel-input" value={val} onChange={e => setVal(e.target.value)} />

{/* Full width — add panel-input-full */}
<input
  className="panel-input panel-input-full"
  value={val}
  onChange={e => setVal(e.target.value)}
  placeholder="Enter a value..."
/>
```

```css
.panel-input {
  background: var(--input-bg);
  color: var(--text-primary);
  border: 1px solid var(--input-border);
  border-radius: var(--radius-sm);
  padding: 5px 8px;
  font-size: 12px;
  font-family: inherit;
  transition: border-color var(--transition-fast);
}
.panel-input:focus {
  border-color: var(--accent-color);
  outline: none;
}
.panel-input-full {
  width: 100%;
  box-sizing: border-box;
}
```

### Focus ring

On `:focus-visible` the global style applies `box-shadow: 0 0 0 1px var(--accent-blue)` in addition to the border change. This is automatic.

---

## Textarea

```tsx
<textarea
  className="panel-input panel-input-full panel-textarea"
  value={code}
  onChange={e => setCode(e.target.value)}
  rows={8}
  style={{ fontFamily: "var(--font-mono)" }}
  placeholder="Paste code here…"
/>
```

```css
.panel-textarea {
  resize: vertical;
  min-height: 80px;
}
```

For code input, always add `style={{ fontFamily: "var(--font-mono)" }}` — `panel-textarea` does not enforce mono by default since some textareas contain prose.

---

## Select

```tsx
<select
  className="panel-select"
  value={provider}
  onChange={e => setProvider(e.target.value)}
>
  <option value="openai">OpenAI</option>
  <option value="anthropic">Anthropic</option>
</select>
```

```css
.panel-select {
  background: var(--input-bg);
  color: var(--text-primary);
  border: 1px solid var(--input-border);
  border-radius: var(--radius-sm);
  padding: 5px 8px;
  font-size: 12px;
}
```

---

## States

### Default
Border: `--border-color`, background: `--bg-tertiary` (dark) / `#ffffff` (light).

### Focus
Border-color transitions to `--accent-color`. Box-shadow adds a 1px ring. Automatic from global CSS.

### Error
Add inline border override — no dedicated class (keep it simple):
```tsx
<input
  className="panel-input panel-input-full"
  style={{ borderColor: "var(--error-color)" }}
/>
<div className="panel-label" style={{ color: "var(--text-danger)", marginTop: 4, marginBottom: 0 }}>
  This field is required.
</div>
```

### Disabled
```tsx
<input className="panel-input" disabled style={{ opacity: 0.5, cursor: "not-allowed" }} />
```

---

## Form Patterns

### Label → Input (vertical)

```tsx
<div style={{ marginBottom: 8 }}>
  <div className="panel-label">Workspace Path</div>
  <input
    className="panel-input panel-input-full"
    value={path}
    onChange={e => setPath(e.target.value)}
    placeholder="/Users/me/project"
  />
</div>
```

The `.panel-label` class provides `margin-bottom: 4px` automatically — do not add extra spacing between label and input.

### Label + Input inline (search-like row)

```tsx
<div className="panel-row">
  <span className="panel-label" style={{ marginBottom: 0, whiteSpace: "nowrap" }}>Filter</span>
  <input className="panel-input panel-input-full" value={filter} onChange={e => setFilter(e.target.value)} />
</div>
```

Override `.panel-label`'s `margin-bottom: 4px` with `marginBottom: 0` when using it inline.

### Input + Button inline

```tsx
<div className="panel-row">
  <input className="panel-input panel-input-full" value={query} onChange={e => setQuery(e.target.value)} placeholder="Search…" />
  <button className="panel-btn panel-btn-primary" style={{ flexShrink: 0 }}>Search</button>
</div>
```

### Full form card

```tsx
<div className="panel-card" style={{ marginBottom: 10 }}>
  <div className="panel-label">Review Title</div>
  <input
    className="panel-input panel-input-full"
    value={title}
    onChange={e => setTitle(e.target.value)}
    placeholder="Review: auth refactor"
    style={{ marginBottom: 8 }}
  />

  <div className="panel-label">Files (comma-separated)</div>
  <input
    className="panel-input panel-input-full"
    value={files}
    onChange={e => setFiles(e.target.value)}
    placeholder="src/auth.rs, src/session.rs"
    style={{ marginBottom: 8 }}
  />

  <button
    className="panel-btn panel-btn-primary"
    onClick={handleSubmit}
    disabled={loading || !title}
  >
    {loading ? "Starting…" : "Start Review"}
  </button>
</div>
```

---

## Rules

### ✅ Do
- Use `panel-input` for all panel inputs (never raw `<input>` without class)
- Add `panel-input-full` for full-width inputs
- Use `panel-textarea` + `panel-input-full` for textareas
- Add `fontFamily: "var(--font-mono)"` for code/path textareas
- Place `panel-label` immediately above inputs
- Override `marginBottom: 0` on labels used inline

### ❌ Don't
```tsx
// Inline style for input
<input style={{ padding: "4px 8px", borderRadius: 4, border: "1px solid var(--border-color)", background: "var(--bg-tertiary)", color: "var(--text-primary)", fontSize: 12 }} />

// Box-sizing missing on full-width
<input style={{ width: "100%" }} />  // may overflow — always add box-sizing: border-box or use panel-input-full

// Mono without explicit font
<textarea className="panel-textarea" />  // code textarea needs fontFamily: "var(--font-mono)"
```
