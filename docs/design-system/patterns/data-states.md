---
layout: page
title: Data States — Design System
permalink: /design-system/patterns/data-states/
---

# Data States

Every panel that loads async data has three possible states: **loading**, **error**, and **empty**. All three must be handled. Missing any one creates a broken user experience.

---

## The Three States

```
State      When                         What to show
─────────────────────────────────────────────────────────────────────
Loading    Fetch is in-flight           Centered text, spinner, or skeleton
Error      Fetch threw or rejected      Dismissable error box with message
Empty      Fetch succeeded, 0 results   Centered text with helpful hint
```

These are mutually exclusive in order: if `loading`, show loading; else if `error`, show error; else if empty, show empty; else show content.

---

## Standard Pattern

```tsx
const [items, setItems] = useState<Item[]>([]);
const [loading, setLoading] = useState(false);
const [error, setError] = useState<string | null>(null);

const load = useCallback(async () => {
  setLoading(true);
  setError(null);
  try {
    setItems(await invoke<Item[]>("get_items"));
  } catch (e) {
    setError(String(e));
  } finally {
    setLoading(false);
  }
}, []);

useEffect(() => { load(); }, [load]);

// In render:
<div className="panel-body">
  {loading && <div className="panel-loading">Loading…</div>}

  {error && (
    <div className="panel-error">
      {error}
      <button onClick={() => setError(null)}>✕</button>
    </div>
  )}

  {!loading && !error && items.length === 0 && (
    <div className="panel-empty">No items found. Click Refresh to reload.</div>
  )}

  {!loading && items.map(item => (
    <div key={item.id} className="panel-card" style={{ marginBottom: 8 }}>
      {item.name}
    </div>
  ))}
</div>
```

---

## Loading State

```tsx
<div className="panel-loading">Loading…</div>
```

```css
.panel-loading {
  text-align: center;
  padding: 32px 20px;
  color: var(--text-secondary);
  font-size: var(--font-size-base);
}
```

### Loading with context
```tsx
<div className="panel-loading">Scanning workspace…</div>
<div className="panel-loading">Analyzing codebase…</div>
<div className="panel-loading">Loading predictions…</div>
```

Always be specific about what's loading. "Loading…" by itself is the minimum; prefer `"Scanning workspace…"` etc.

### Initial load vs refresh

```tsx
// Initial — panel body shows only loading state
{loading && <div className="panel-loading">Loading…</div>}

// Refresh — keep existing content visible, show loading in button
<button className="panel-btn panel-btn-primary panel-btn-sm" disabled={loading}>
  {loading ? "Refreshing…" : "↻ Refresh"}
</button>
// Don't replace content with loading state on refresh — use button state only
```

---

## Error State

```tsx
<div className="panel-error">
  {error}
  <button onClick={() => setError(null)}>✕</button>
</div>
```

```css
.panel-error {
  background: var(--error-bg);
  border: 1px solid var(--error-color);
  border-radius: var(--radius-sm);
  padding: 8px 12px;
  color: var(--text-danger);
  font-size: 12px;
  display: flex;
  align-items: center;
  justify-content: space-between;
  gap: 8px;
}
.panel-error button {
  background: none;
  border: none;
  color: var(--text-danger);
  cursor: pointer;
  font-size: 14px;
  padding: 0 4px;
}
```

### Error with retry

```tsx
<div className="panel-error">
  <span>{error}</span>
  <div style={{ display: "flex", gap: 8, alignItems: "center" }}>
    <button
      className="panel-btn panel-btn-secondary panel-btn-xs"
      style={{ color: "var(--text-danger)" }}
      onClick={load}
    >
      Retry
    </button>
    <button onClick={() => setError(null)}>✕</button>
  </div>
</div>
```

### Error placement

Always place the error box inside `panel-body`, above the content (or where content would be). Never show error inside a card or below content.

```tsx
<div className="panel-body">
  {error && (
    <div className="panel-error" style={{ marginBottom: 10 }}>
      {error}
      <button onClick={() => setError(null)}>✕</button>
    </div>
  )}
  {/* content below */}
</div>
```

---

## Empty State

```tsx
<div className="panel-empty">No items found. Click Refresh to reload.</div>
```

```css
.panel-empty {
  text-align: center;
  padding: 40px 20px;
  color: var(--text-muted);
  font-size: 13px;
}
```

### Empty state copy guidelines

Always answer two questions:
1. What's missing? ("No predictions yet")
2. What should the user do? ("Edit some files to generate predictions")

```tsx
// ✅ Good — explains what and why
<div className="panel-empty">No predictions yet. Edit some files to generate predictions.</div>
<div className="panel-empty">Click Scan to analyze your codebase health.</div>
<div className="panel-empty">No pending AST edits. Use the AI to suggest refactors.</div>

// ❌ Unhelpful
<div className="panel-empty">No data.</div>
<div className="panel-empty">Nothing here.</div>
```

### Empty with action button

```tsx
<div className="panel-empty">
  <div style={{ marginBottom: 12 }}>No sessions found.</div>
  <button className="panel-btn panel-btn-primary" onClick={handleCreate}>
    Start First Session
  </button>
</div>
```

---

## Combined State Logic

The render order for state priority:

```tsx
{/* Priority 1: Loading — replace content */}
{loading && <div className="panel-loading">Loading…</div>}

{/* Priority 2: Error — show above content */}
{error && (
  <div className="panel-error" style={{ marginBottom: error && !loading ? 10 : 0 }}>
    {error}
    <button onClick={() => setError(null)}>✕</button>
  </div>
)}

{/* Priority 3: Empty — only when data loaded and empty */}
{!loading && !error && items.length === 0 && (
  <div className="panel-empty">No items yet.</div>
)}

{/* Priority 4: Content — only when loaded */}
{!loading && items.map(item => (
  <div key={item.id} className="panel-card" style={{ marginBottom: 8 }}>
    {item.name}
  </div>
))}
```

---

## Tab-Specific Empty States

When a panel has tabs, each tab needs its own empty state:

```tsx
{tab === "predictions" && !loading && predictions.length === 0 && !error && (
  <div className="panel-empty">No predictions yet. Edit some files to generate predictions.</div>
)}

{tab === "patterns" && !loading && patterns.length === 0 && !error && (
  <div className="panel-empty">No patterns detected yet. Patterns emerge as you edit files.</div>
)}
```

---

## Optimistic Updates

For accept/reject/delete operations, update the UI immediately and revert on failure:

```tsx
const handleAction = async (id: string) => {
  // 1. Optimistic update
  setItems(prev => prev.map(item => item.id === id ? { ...item, status: "accepted" } : item));

  try {
    await invoke("apply_action", { id });
    // 2. Success — optionally refresh
    await refresh();
  } catch (e) {
    // 3. Revert on failure
    setItems(prev => prev.map(item => item.id === id ? { ...item, status: "pending" } : item));
    setError(String(e));
  }
};
```

---

## Rules

### ✅ Always
- Handle all three states: loading, error, empty
- Dismiss errors with a `✕` button
- Use `finally { setLoading(false) }` — never set loading in catch/try alone
- Reset `error` to null at start of every fetch (`setError(null)`)
- Write helpful empty state copy (what + why + action)

### ❌ Never
- Leave loading state stuck (always use `finally`)
- Show empty state while loading is true
- Show content while error is present (show error above, then content below if applicable)
- Use `"..."` as loading label in empty state (use "Loading…" with ellipsis `…`)
- Show raw error objects (`String(e)` — always convert)
