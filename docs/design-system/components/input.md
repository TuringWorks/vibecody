---
layout: page
title: Input — Design System
permalink: /design-system/components/input/
---

# Input

Text inputs, textareas, and selects inherit global styles from `App.css`. Use the `panel-input` class inside panels for consistent sizing.

---

## Text Input

```tsx
<input
  className="panel-input"
  type="text"
  placeholder="Search…"
  value={value}
  onChange={e => setValue(e.target.value)}
/>
```

## Textarea

```tsx
<textarea
  className="panel-input"
  rows={4}
  placeholder="Describe a task…"
  value={value}
  onChange={e => setValue(e.target.value)}
/>
```

## Select

```tsx
<select className="panel-input" value={val} onChange={e => setVal(e.target.value)}>
  <option value="a">Option A</option>
  <option value="b">Option B</option>
</select>
```

---

## States

| State | How |
|---|---|
| Default | Uses `--bg-tertiary` background, `--border-color` border |
| Focus | Blue border (`--accent-blue`) + glow ring |
| Disabled | `opacity: 0.5; cursor: not-allowed` |
| Error | Red border (`--error-color`) + error message below |

```tsx
// Error state
<input
  className="panel-input"
  style={{ borderColor: hasError ? "var(--error-color)" : undefined }}
/>
{hasError && <span className="text-error" style={{ fontSize: "var(--font-size-sm)", marginTop: 4 }}>Required</span>}
```

---

## Label Pattern

```tsx
<div style={{ marginBottom: "var(--space-3)" }}>
  <label style={{ display: "block", fontSize: "var(--font-size-sm)", color: "var(--text-secondary)", marginBottom: "var(--space-1)" }}>
    API Key
  </label>
  <input className="panel-input" type="password" />
</div>
```
