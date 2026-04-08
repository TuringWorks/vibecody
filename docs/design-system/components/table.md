---
layout: page
title: Table — Design System
permalink: /design-system/components/table/
---

# Table

Use `panel-table` for structured data with columns. For simple item lists, prefer cards with `panel-card`. Use tables when data has 2+ aligned columns and benefits from visual column comparison.

---

## Basic Table

```tsx
<table className="panel-table">
  <thead>
    <tr>
      <th>Name</th>
      <th style={{ textAlign: "right", width: 80 }}>Lines</th>
      <th style={{ textAlign: "right", width: 60 }}>Lang</th>
    </tr>
  </thead>
  <tbody>
    {items.map((item, i) => (
      <tr key={item.id}>
        <td>{item.name}</td>
        <td style={{ textAlign: "right", fontFamily: "var(--font-mono)" }}>{item.lines.toLocaleString()}</td>
        <td style={{ textAlign: "right" }}>{item.lang}</td>
      </tr>
    ))}
  </tbody>
</table>
```

```css
.panel-table {
  width: 100%;
  border-collapse: collapse;
  font-size: 12px;
}
.panel-table th {
  text-align: left;
  padding: 6px 8px;
  font-weight: 600;
  color: var(--text-secondary);
  border-bottom: 1px solid var(--border-color);
  font-size: 11px;
  text-transform: uppercase;
  letter-spacing: 0.5px;
}
.panel-table td {
  padding: 6px 8px;
  border-bottom: 1px solid var(--border-subtle);
}
.panel-table tr:hover { background: var(--bg-hover); }
```

---

## Column Patterns

### Index column (row number)

```tsx
<td style={{ color: "var(--text-muted)", width: 32, textAlign: "right" }}>{i + 1}.</td>
```

### File path column (truncated mono)

```tsx
<td
  className="panel-mono"
  style={{ overflow: "hidden", textOverflow: "ellipsis", whiteSpace: "nowrap", maxWidth: 0 }}
  title={item.path}
>
  <span style={{ color: "var(--text-muted)", marginRight: 6 }}>{i + 1}.</span>
  {item.path}
</td>
```

`maxWidth: 0` on the `<td>` with `table-layout: fixed` makes `overflow: hidden` work. Or use `minWidth: 0` on the table element.

### Numeric column (right-aligned mono)

```tsx
<td className="panel-mono" style={{ textAlign: "right" }}>
  {value.toLocaleString()}
</td>
```

### Status column (badge)

```tsx
<td>
  <span className={statusTag(item.status)}>{item.status}</span>
</td>
```

### Colored language column

```tsx
<td style={{ textAlign: "right", color: LANG_COLORS[item.language] ?? "var(--accent-blue)", fontSize: "var(--font-size-xs)" }}>
  {item.language}
</td>
```

### Action column (buttons)

```tsx
<td style={{ textAlign: "right", whiteSpace: "nowrap" }}>
  <button className="panel-btn panel-btn-danger panel-btn-xs" onClick={() => handleDelete(item.id)}>
    Delete
  </button>
</td>
```

---

## Row States

### Clickable rows

```tsx
<tr
  style={{ cursor: "pointer" }}
  onClick={() => handleSelect(item.id)}
>
  <td>...</td>
</tr>
```

Hover background is automatic via `.panel-table tr:hover { background: var(--bg-hover) }`.

### Selected row

```tsx
<tr style={{ background: isSelected ? "var(--accent-bg)" : undefined }}>
  <td>...</td>
</tr>
```

---

## Table with Note / Caption

```tsx
<div style={{ fontSize: "var(--font-size-xs)", color: "var(--text-secondary)", padding: "4px 8px 8px", fontStyle: "italic" }}>
  Complexity = count of branch-inducing keywords (if/for/while/match/&&/||…)
</div>
<table className="panel-table">
  ...
</table>
```

---

## Responsive Overflow

Tables inside `panel-body` scroll horizontally if they overflow:

```tsx
<div style={{ overflowX: "auto" }}>
  <table className="panel-table" style={{ minWidth: 500 }}>
    ...
  </table>
</div>
```

---

## When to Use Table vs Cards

| Scenario | Use |
|----------|-----|
| 2+ columns of aligned data | Table |
| File list with size/lang columns | Table |
| Ranked list with scores | Table |
| Items with variable content / actions | Cards |
| Items that expand or collapse | Cards |
| Items with progress bars | Cards |
| Single column content | Cards or plain rows |

---

## Rules

### ✅ Do
- Use `className="panel-table"` — never build a table inline
- Right-align numeric columns
- Set fixed `width` on narrow columns (counts, percentages)
- Use `panel-mono` on numeric columns
- Add `title={fullValue}` on truncated cells

### ❌ Don't
```tsx
// Inline table styling
<table style={{ width: "100%", borderCollapse: "collapse", fontSize: 12 }}>
  <th style={{ padding: "4px 8px", color: "var(--text-secondary)", fontSize: 10 }}>

// Text-align center on data columns (use right for numbers, left for text)
<td style={{ textAlign: "center" }}>{item.lines}</td>
```
