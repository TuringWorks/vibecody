---
layout: page
title: Typography — Design System
permalink: /design-system/foundations/typography/
---

# Typography

VibeUI uses **Inter** for UI text and **JetBrains Mono** for code, paths, and numeric values. Both are CSS variables — never hardcode font family strings.

---

## Font Families

```css
--font-family: 'Inter', -apple-system, BlinkMacSystemFont, 'Segoe UI', system-ui, sans-serif
--font-mono:   'JetBrains Mono', 'Monaco', 'Menlo', monospace
```

Panels inherit these from `body` automatically. Only set `fontFamily` when switching to mono.

```tsx
// ✅ Inherit sans (do nothing)
<span>Body text</span>

// ✅ Mono for paths, code, numbers
<span style={{ fontFamily: "var(--font-mono)" }}>src/main.rs:42</span>
```

---

## Type Scale

| Token | Value | Use case |
|---|---|---|
| `--font-size-xs` | 10px | Timestamps, row numbers, badge text |
| `--font-size-sm` | 11px | Labels, captions, secondary metadata |
| `--font-size-base` | 12px | Panel body text (default) |
| `--font-size-md` | 13px | Primary content, descriptions |
| `--font-size-lg` | 14px | Section headings, card titles |
| `--font-size-xl` | 15px | Panel heading (`h3` in `.panel-header`) |
| `--font-size-2xl` | 18px | Key metric values |
| `--font-size-3xl` | 24px | Hero stats, overall scores |

---

## Font Weight Scale

| Token | Value | Use case |
|---|---|---|
| `--font-normal` | 400 | Body text |
| `--font-medium` | 500 | Button text |
| `--font-semibold` | 600 | Card titles, section headings |
| `--font-bold` | 700 | Panel headings, critical labels |

---

## Usage Rules

```tsx
// ✅ Use tokens
fontSize: "var(--font-size-sm)"
fontWeight: "var(--font-semibold)"

// ❌ Never hardcode
fontSize: 11        // use var(--font-size-sm)
fontWeight: 600     // use var(--font-semibold)
fontFamily: "Inter" // use var(--font-family)
```

---

## Hierarchy Example

```
Panel heading       15px / 600   (.panel-header h3)
Section heading     14px / 600
Card title          13px / 600
Body / value        13px / 400   (most content)
Label / caption     11px / 400
Badge / timestamp   10px / 600
```
