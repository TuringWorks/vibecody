---
layout: page
title: Badge & Tag — Design System
permalink: /design-system/components/badge-tag/
---

# Badge & Tag

Two inline label components: **tags** (inside panels) and **badges** (global utility classes).

---

## Panel Tags

Use `panel-tag` + intent modifier inside `.panel-container`:

```tsx
<span className="panel-tag panel-tag-success">Passing</span>
<span className="panel-tag panel-tag-danger">Failed</span>
<span className="panel-tag panel-tag-warning">Slow</span>
<span className="panel-tag panel-tag-info">Running</span>
<span className="panel-tag panel-tag-neutral">Unknown</span>
```

| Class | Color |
|---|---|
| `panel-tag-success` | Green — pass, active, healthy |
| `panel-tag-danger` | Red — fail, error, critical |
| `panel-tag-warning` | Gold — warn, slow, degraded |
| `panel-tag-info` | Blue — info, running, neutral+ |
| `panel-tag-neutral` | Grey — unknown, inactive |

---

## Badge Utility Classes

Global classes for use anywhere (including outside panels):

```tsx
<span className="badge-success">OK</span>
<span className="badge-error">Error</span>
<span className="badge-warning">Warn</span>
<span className="badge-info">Info</span>
<span className="badge-neutral">N/A</span>
```

---

## Status Tag Helper

```tsx
const statusTag = (status: string) => {
  const s = status.toLowerCase();
  if (s.includes("pass") || s.includes("ok") || s.includes("complete") || s.includes("success"))
    return "panel-tag panel-tag-success";
  if (s.includes("warn") || s.includes("progress") || s.includes("slow"))
    return "panel-tag panel-tag-warning";
  if (s.includes("fail") || s.includes("error") || s.includes("critical"))
    return "panel-tag panel-tag-danger";
  if (s.includes("info") || s.includes("run"))
    return "panel-tag panel-tag-info";
  return "panel-tag panel-tag-neutral";
};

<span className={statusTag(item.status)}>{item.status}</span>
```

---

## Rules

- Use `panel-tag` inside panels, `badge-*` everywhere else
- Tags are inline — don't add block-level styles
- Never use `--accent-purple`, `--accent-gold`, `--accent-rose` for status — use semantic intent classes
