---
layout: page
title: Data States — Design System
permalink: /design-system/patterns/data-states/
---

# Data States

Every async panel must handle three states: **loading**, **error**, and **empty**. Skipping any of these creates confusing UX.

---

## The Three States

```tsx
// Loading
{loading && <div className="panel-loading">Loading…</div>}

// Error
{error && (
  <div className="panel-error">
    {error}
    <button className="panel-btn panel-btn-ghost panel-btn-sm" onClick={() => setError(null)}>✕</button>
  </div>
)}

// Empty (only shown when not loading and no error)
{!loading && !error && items.length === 0 && (
  <div className="panel-empty">
    No items yet. Click Refresh to load.
  </div>
)}

// Data
{items.map(item => (
  <div key={item.id} className="panel-card">{item.name}</div>
))}
```

---

## Complete Async Pattern

```tsx
export function MyPanel() {
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

  return (
    <div className="panel-container">
      <div className="panel-header">
        <h3>Items</h3>
        <button className="panel-btn panel-btn-ghost panel-btn-sm"
          style={{ marginLeft: "auto" }} onClick={load} disabled={loading}>
          ↻
        </button>
      </div>
      <div className="panel-body">
        {loading && <div className="panel-loading">Loading items…</div>}
        {error   && <div className="panel-error">{error}</div>}
        {!loading && !error && items.length === 0 && (
          <div className="panel-empty">No items found.</div>
        )}
        {items.map(item => (
          <div key={item.id} className="panel-card">{item.name}</div>
        ))}
      </div>
    </div>
  );
}
```

---

## Rules

1. **Always show loading first** — never show empty while still loading
2. **Never suppress errors** — always surface Tauri errors to the user
3. **Dismissible errors** — always provide a way to clear the error
4. **Retry on error** — the refresh button should work even when there's an error
5. **Don't clear data on reload** — keep stale data visible while refreshing (show loading indicator instead of blanking the view)
