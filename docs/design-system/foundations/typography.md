---
layout: page
title: Typography — Design System
permalink: /design-system/foundations/typography/
---

# Typography

VibeUI uses **Inter** for UI text and **JetBrains Mono** for code, paths, and numeric values that need alignment. Both are accessed via CSS variables — never hardcode font family strings.

---

## Font Families

```css
--font-family: 'Inter', -apple-system, BlinkMacSystemFont, 'Segoe UI', system-ui, sans-serif
--font-mono:   'JetBrains Mono', 'Monaco', 'Menlo', monospace
```

Both are inherited from `body` — panels get them automatically. Only set `fontFamily` when overriding to mono.

```tsx
// ✅ Inherit sans automatically (do nothing)
<span>Body text</span>

// ✅ Explicitly mono for paths/code/numbers
<span style={{ fontFamily: "var(--font-mono)" }}>src/main.rs:42</span>
<span className="panel-mono">src/main.rs:42</span>   // same, via utility class
```

---

## Type Scale

```
Token               Value   Use case
--font-size-xs      10px    Timestamps, row numbers, badge text
--font-size-sm      11px    Labels, captions, secondary metadata
--font-size-base    12px    Panel body text (default for .panel-container)
--font-size-md      13px    Primary content, descriptions
--font-size-lg      14px    Section headings, card titles
--font-size-xl      15px    Panel heading (inside .panel-header h3)
--font-size-2xl     18px    Key metric values, large numbers
--font-size-3xl     24px    Hero stats (overall score, total counts)
```

`body` sets `font-size: 13px` globally. `.panel-container` overrides to `12px` for compact panel UIs.

---

## Font Weight Scale

```
Token               Value   Use case
--font-normal       400     Body text, descriptions
--font-medium       500     Labels, button text (panel-btn uses 500)
--font-semibold     600     Card titles, section headings, key values
--font-bold         700     Panel heading (h3 in .panel-header), stat numbers
```

---

## Hierarchy — Visual Levels

### Level 1 — Panel heading
```tsx
// Inside .panel-header — rendered as h3
<h3>Panel Title</h3>
// Resolved: 14px, 700, --text-primary
// CSS: .panel-header h3 { font-size: 14px; font-weight: 700; margin: 0; }
```

### Level 2 — Section heading
```tsx
<div className="panel-heading">Section Title</div>
// Resolved: 14px, 700, --text-primary
// Use inside .panel-body to introduce a section
```

### Level 3 — Card title / item name
```tsx
<span style={{ fontSize: "var(--font-size-base)", fontWeight: "var(--font-semibold)" }}>
  Item Name
</span>
// Resolved: 12px, 600, --text-primary
```

### Level 4 — Label / field name
```tsx
<div className="panel-label">Field name</div>
// Resolved: 11px, 400, --text-secondary
// Always immediately above an input or value
```

### Level 5 — Body value
```tsx
<span className="panel-value">Some content</span>
// Resolved: 13px, 500, --text-primary
```

### Level 6 — Metadata / secondary
```tsx
<span style={{ fontSize: "var(--font-size-xs)", color: "var(--text-secondary)" }}>
  2 minutes ago
</span>
```

---

## Monospaced Use Cases

Use `var(--font-mono)` or `.panel-mono` for:

| Content | Example |
|---------|---------|
| File paths | `src/auth/session.rs` |
| Function names | `handle_request()` |
| Line numbers | `L42`, `42` |
| Hashes / IDs | `ast-1712345678` |
| Code snippets | `fn main() {` |
| Numeric stats needing alignment | `1,234`, `92.3%` |
| Q-Table / model values | `0.999`, `0.15` |

```tsx
<span className="panel-mono">src/main.rs:156</span>

// For stat numbers — mono keeps columns aligned
<div className="panel-stat">
  <div className="panel-mono panel-stat-value">10,535</div>
  <div className="panel-stat-label">Tests</div>
</div>
```

---

## Text Truncation

For file paths in tight spaces:

```tsx
<span
  className="panel-mono"
  style={{ overflow: "hidden", textOverflow: "ellipsis", whiteSpace: "nowrap" }}
  title={fullPath}
>
  {shortPath}
</span>
```

Wrap this in a container with `minWidth: 0` if inside a flex row — without this, the span ignores `overflow: hidden`.

```tsx
<div style={{ display: "flex", minWidth: 0 }}>
  <span
    className="panel-mono"
    style={{ flex: 1, overflow: "hidden", textOverflow: "ellipsis", whiteSpace: "nowrap" }}
    title={path}
  >
    {path}
  </span>
  <span style={{ flexShrink: 0, color: "var(--text-secondary)" }}>128 lines</span>
</div>
```

---

## Line Height

Body inherits `line-height: 1.5` from `body`. For compact panel rows (< 14px font), consider `1.4`. For stat numbers, use `line-height: 1`.

```tsx
// Stat numbers — tight
<div style={{ fontSize: "var(--font-size-3xl)", fontWeight: "var(--font-bold)", lineHeight: 1 }}>
  {score}
</div>
```

---

## Rules

### ✅ Do
- Inherit font-family from body (no explicit `fontFamily` unless switching to mono)
- Use `--font-size-*` tokens for all font sizes
- Use `--font-*` weight tokens for all font weights
- Use `.panel-label` class for input labels
- Use `.panel-mono` class for paths, IDs, code

### ❌ Don't
```tsx
fontSize: 12           // use "var(--font-size-base)"
fontWeight: 600        // use "var(--font-semibold)"
fontFamily: "monospace"   // use "var(--font-mono)"
fontFamily: "var(--font-family)"  // just remove it, it's inherited
```
