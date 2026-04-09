---
render_with_liquid: false
layout: page
title: Badge & Tag — Design System
permalink: /design-system/components/badge-tag/
---

# Badge & Tag

VibeUI has two label systems: **tags** (small, pill-like, inline) for status and intent, and **badges** (medium, filled or outlined) for counts and emphasis. Choose based on size and semantic weight.

---

## Tags (small — panel use)

Tags are compact inline labels. Use them for status, category, intent, and count inside panel rows.

```tsx
{/* Intent variants */}
<span className="panel-tag panel-tag-info">pending</span>
<span className="panel-tag panel-tag-success">passed</span>
<span className="panel-tag panel-tag-warning">slow</span>
<span className="panel-tag panel-tag-danger">critical</span>
<span className="panel-tag panel-tag-neutral">unknown</span>
```

```css
.panel-tag {
  display: inline-flex;
  align-items: center;
  padding: 1px 7px;
  border-radius: var(--radius-xs);   /* 3px */
  font-size: var(--font-size-xs);    /* 10px */
  font-weight: var(--font-medium);   /* 500 */
  white-space: nowrap;
}

.panel-tag-info    { background: var(--info-bg);    color: var(--text-info); }
.panel-tag-success { background: var(--success-bg); color: var(--text-success); }
.panel-tag-warning { background: var(--warning-bg); color: var(--text-warning); }
.panel-tag-danger  { background: var(--error-bg);   color: var(--text-danger); }
.panel-tag-neutral { background: var(--bg-tertiary); color: var(--text-secondary); }
```

### Tag with icon

```tsx
<span className="panel-tag panel-tag-success" style={{ gap: 4 }}>
  <Icon name="check" size={9} />
  passed
</span>
```

---

## Badges (medium — app shell / prominent)

Badges are larger, with full background fill. Used in the activity bar, navigation, and for count indicators.

```tsx
<span className="panel-badge badge-success">3 passed</span>
<span className="panel-badge badge-error">1 failed</span>
<span className="panel-badge badge-warning">2 slow</span>
<span className="panel-badge badge-info">5 pending</span>
<span className="panel-badge badge-neutral">draft</span>
```

```css
.panel-badge {
  display: inline-flex;
  align-items: center;
  gap: 4px;
  padding: 2px 8px;
  border-radius: 12px;   /* pill */
  font-size: 11px;
  font-weight: 500;
}
.badge-success { background: var(--success-color); color: var(--btn-primary-fg); }
.badge-error   { background: var(--error-color);   color: var(--btn-error-fg); }
.badge-warning { background: var(--warning-color); color: var(--btn-primary-fg); }
.badge-info    { background: var(--info-color);    color: var(--btn-primary-fg); }
.badge-neutral { background: var(--bg-tertiary);   color: var(--text-secondary); }
```

---

## Decision: Tag vs Badge

| Situation | Use |
|-----------|-----|
| Status inside a panel row (1–2 words) | `panel-tag` |
| Count in panel header | `panel-tag` |
| Accept-rate or percentage in header | `panel-tag` |
| Category / type chip on a card | `panel-tag` |
| Count on activity bar nav item | `panel-badge` |
| Prominent status in a summary view | `panel-badge` |
| Large emphasis label (standalone) | `panel-badge` |

---

## Dynamic Tag from Status String

```tsx
// Generic status → tag class
const statusTag = (s: string): string => {
  const l = s.toLowerCase();
  if (l.includes("pass") || l.includes("ok") || l.includes("complete") || l.includes("success"))
    return "panel-tag panel-tag-success";
  if (l.includes("warn") || l.includes("slow") || l.includes("pending") || l.includes("progress"))
    return "panel-tag panel-tag-warning";
  if (l.includes("fail") || l.includes("error") || l.includes("critical") || l.includes("reject"))
    return "panel-tag panel-tag-danger";
  if (l.includes("run") || l.includes("info") || l.includes("active"))
    return "panel-tag panel-tag-info";
  return "panel-tag panel-tag-neutral";
};

<span className={statusTag(item.status)}>{item.status}</span>
```

### Priority → tag class

```tsx
const priorityTag = (p: string): string =>
  p === "Critical" ? "panel-tag panel-tag-danger"
  : p === "High"   ? "panel-tag panel-tag-warning"
  : p === "Medium" ? "panel-tag panel-tag-info"
  : "panel-tag panel-tag-neutral";

<span className={priorityTag(item.priority)}>{item.priority}</span>
```

### Confidence → tag class

```tsx
const confTag = (c: number): string =>
  c > 0.85 ? "panel-tag panel-tag-success"
  : c > 0.70 ? "panel-tag panel-tag-warning"
  : "panel-tag panel-tag-danger";

<span className={confTag(prediction.confidence)}>
  {(prediction.confidence * 100).toFixed(0)}%
</span>
```

---

## Tags in Context

### In a panel header (count)

```tsx
<div className="panel-header">
  <h3>Edit Predictions</h3>
  <span className="panel-tag panel-tag-neutral" style={{ marginLeft: "auto" }}>
    {(model.acceptanceRate * 100).toFixed(0)}% accept rate
  </span>
</div>
```

### In a card row (status)

```tsx
<div className="panel-row" style={{ marginBottom: 6 }}>
  <span style={{ fontWeight: "var(--font-semibold)" }}>{item.name}</span>
  <span className={statusTag(item.status)} style={{ marginLeft: 8 }}>{item.status}</span>
  <span style={{ marginLeft: "auto", color: "var(--text-secondary)" }}>42 lines</span>
</div>
```

### As operation/type chip

```tsx
<span className="panel-tag panel-tag-info">{edit.operation}</span>   {/* "rename", "extract" */}
<span className="panel-tag panel-tag-neutral">{prediction.pattern}</span>
```

### Auto-fixable indicator

```tsx
{item.autoFixable && (
  <span className="panel-tag panel-tag-success" style={{ marginLeft: 8 }}>auto-fixable</span>
)}
```

---

## Rules

### ✅ Do
- Always use intent variants (`panel-tag-{intent}`) — never `panel-tag` alone
- Use `panel-tag` inside panel rows and headers
- Use `panel-badge` for navigation counts and prominent standalone labels
- Use semantic functions (`statusTag()`, `priorityTag()`, `confTag()`) for dynamic values

### ❌ Don't
```tsx
// Missing intent variant
<span className="panel-tag">pending</span>

// Hardcoded color badge
<span style={{ padding: "2px 6px", borderRadius: 3, background: "#f44336", color: "#fff", fontSize: 10 }}>
  critical
</span>

// Hardcoded priority badge
<span style={{
  background: priority === "Critical" ? "#f44336" : "#ff9800",
  color: "#fff", padding: "2px 6px", fontSize: 11, borderRadius: 3
}}>
  {priority}
</span>
// → use priorityTag() function + panel-tag classes
```
