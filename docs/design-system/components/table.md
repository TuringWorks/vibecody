---
layout: page
title: Table — Design System
permalink: /design-system/components/table/
---

# Table

Data tables inside panels. Always wrap in `.panel-card` for proper surface background.

---

## Basic Table

```tsx
<div className="panel-card">
  <table className="panel-table">
    <thead>
      <tr>
        <th>Name</th>
        <th>Status</th>
        <th style={{ textAlign: "right" }}>Score</th>
      </tr>
    </thead>
    <tbody>
      {rows.map(r => (
        <tr key={r.id}>
          <td>{r.name}</td>
          <td><span className={statusTag(r.status)}>{r.status}</span></td>
          <td style={{ textAlign: "right", fontFamily: "var(--font-mono)" }}>{r.score}</td>
        </tr>
      ))}
    </tbody>
  </table>
</div>
```

---

## Column Alignment

| Content type | Alignment |
|---|---|
| Names, labels, text | Left (default) |
| Numbers, values, sizes | Right (`textAlign: "right"`) |
| Status tags, badges | Left |
| Actions | Right |

---

## Numeric Values

Always render numbers in mono font:

```tsx
<td style={{ textAlign: "right", fontFamily: "var(--font-mono)", fontSize: "var(--font-size-base)" }}>
  {value.toFixed(2)}
</td>
```

---

## Empty Table State

```tsx
{rows.length === 0 && (
  <tr>
    <td colSpan={3} style={{ textAlign: "center", padding: "var(--space-8)", color: "var(--text-secondary)" }}>
      No data
    </td>
  </tr>
)}
```
