---
layout: page
title: Button — Design System
permalink: /design-system/components/button/
---

# Button

Two button systems exist: **app-level** (shell chrome) and **panel-level** (inside panels). Never mix them.

---

## Panel Buttons

Used inside `.panel-container`. All use `panel-btn` base class.

```tsx
// Primary — one per panel view
<button className="panel-btn panel-btn-primary">Save</button>

// Secondary — supporting actions
<button className="panel-btn panel-btn-secondary">Cancel</button>

// Danger — destructive actions
<button className="panel-btn panel-btn-danger">Delete</button>

// Ghost — subtle actions
<button className="panel-btn panel-btn-ghost">More</button>

// Sizes
<button className="panel-btn panel-btn-primary panel-btn-sm">Small</button>
<button className="panel-btn panel-btn-primary panel-btn-lg">Large</button>

// Disabled
<button className="panel-btn panel-btn-primary" disabled>Loading…</button>
```

---

## App-Level Buttons

Used in the app shell (activity bar, editor toolbar). Do not use inside panels.

```tsx
<button className="btn-primary">Primary</button>
<button className="btn-secondary">Secondary</button>
<button className="btn-icon" title="Settings"><Icon name="settings" size={16} /></button>
```

---

## Icon Buttons

```tsx
// App shell — icon only
<button className="btn-icon" title="Refresh">
  <Icon name="refresh-cw" size={16} />
</button>

// Panel — icon only  
<button className="panel-btn panel-btn-ghost panel-btn-icon" title="Refresh">
  <Icon name="refresh-cw" size={14} />
</button>
```

---

## Rules

1. **One primary per view** — only one `panel-btn-primary` visible at a time
2. **Disabled state** — always use the `disabled` attribute, not opacity hacks
3. **Loading state** — replace text with "Loading…" and add `disabled`
4. **Never mix systems** — `btn-primary` inside a panel, or `panel-btn` in the shell, breaks visual hierarchy
