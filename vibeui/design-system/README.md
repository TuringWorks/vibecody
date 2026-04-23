# VibeUI Design System

A complete, token-based design system for VibeUI panels and components. Inspired by Material Design 3, GitHub Primer, and Shopify Polaris. All tokens are CSS custom properties defined in `design-system/tokens.css`. All components are CSS classes, usable with `className` in React/TSX.

---

## Table of Contents

### Foundations
| File | What it covers |
|------|----------------|
| [Color](./foundations/color.md) | Palette, semantic tokens, dark/light, usage rules |
| [Typography](./foundations/typography.md) | Font scale, weights, hierarchy, mono vs sans |
| [Spacing](./foundations/spacing.md) | 4px grid, space tokens, layout vs component spacing |
| [Elevation](./foundations/elevation.md) | Shadows, glass surfaces, z-index layers |
| [Motion](./foundations/motion.md) | Transition tokens, animation principles |

### Components
| File | What it covers |
|------|----------------|
| [Panel](./components/panel.md) | The core layout primitive — container/header/body/footer |
| [Button](./components/button.md) | All button variants, sizes, states |
| [Input](./components/input.md) | Text input, textarea, select, states |
| [Card](./components/card.md) | Surface hierarchy, card variants |
| [Badge & Tag](./components/badge-tag.md) | Tags (small), badges (medium), chips |
| [Progress](./components/progress.md) | Progress bars, sizes, semantic colors |
| [Table](./components/table.md) | Data tables, column alignment |
| [Tabs](./components/tabs.md) | Tab bar and tab variants |

### Patterns
| File | What it covers |
|------|----------------|
| [Data States](./patterns/data-states.md) | Loading, empty, error — the three async states |
| [Forms](./patterns/forms.md) | Form layout, validation, submit feedback |

---

## Quick Reference

### The rules every panel must follow

```
1.  Root: flex: 1, minHeight: 0            (never height: 100%)
2.  Structure: container → header → body → footer
3.  4-sided margins: all scrollable content inside .panel-body
    (NEVER put content as a direct sibling of .panel-container — it renders
     flush against the edges. See design-system/components/panel.md § Margin Rule)
4.  Colors: CSS vars only                  (never #4caf50, #fff, etc.)
5.  Spacing: multiples of 4px              (4, 8, 12, 16, 20, 24, 32)
6.  Buttons: panel-btn + panel-btn-{variant}
7.  Empty state: className="panel-empty"
8.  Loading: className="panel-loading"
9.  Error: className="panel-error"
10. Cards: className="panel-card"
11. Progress: className="progress-bar" + progress-bar-fill + progress-bar-{color}
12. Tags: className="panel-tag panel-tag-{intent}"
```

### Color quick-pick

| Need | Token |
|------|-------|
| Success / pass / green | `var(--success-color)` · `var(--text-success)` · `var(--success-bg)` |
| Error / fail / red | `var(--error-color)` · `var(--text-danger)` · `var(--error-bg)` |
| Warning / amber | `var(--warning-color)` · `var(--text-warning)` · `var(--warning-bg)` |
| Info / blue | `var(--info-color)` · `var(--text-info)` · `var(--info-bg)` |
| Accent (brand blue) | `var(--accent-color)` · `var(--accent-blue)` |
| Body text | `var(--text-primary)` |
| Secondary text | `var(--text-secondary)` |
| Disabled / placeholder | `var(--text-muted)` |
| Card background | `var(--bg-secondary)` |
| Page background | `var(--bg-primary)` |
| Border | `var(--border-color)` |

### Font size quick-pick

| Use case | Token | Value |
|----------|-------|-------|
| Timestamp, badge | `--font-size-xs` | 10px |
| Label, caption | `--font-size-sm` | 11px |
| Panel body | `--font-size-base` | 12px |
| Primary content | `--font-size-md` | 13px |
| Section heading | `--font-size-lg` | 14px |
| Panel heading | `--font-size-xl` | 15px |
| Key metric | `--font-size-2xl` | 18px |
| Hero stat | `--font-size-3xl` | 24px |

### Spacing quick-pick

| Use case | Token | Value |
|----------|-------|-------|
| Icon gap, tight | `--space-1` | 4px |
| Related elements | `--space-2` | 8px |
| Card padding | `--space-3` | 12px |
| Section padding | `--space-4` | 16px |
| Large spacing | `--space-5` | 20px |
| Section gap | `--space-6` | 24px |
| Empty state pad | `--space-8` | 32px |

---

## Minimal Panel Template

Copy this to start every new panel:

```tsx
import { useState, useCallback } from "react";
import { invoke } from "@tauri-apps/api/core";

interface Item { id: string; name: string; }

export default function MyPanel() {
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

  return (
    <div className="panel-container">
      <div className="panel-header">
        <h3>My Panel</h3>
        <button
          className="panel-btn panel-btn-primary panel-btn-sm"
          style={{ marginLeft: "auto" }}
          onClick={load}
          disabled={loading}
        >
          {loading ? "Loading…" : "↻ Refresh"}
        </button>
      </div>

      <div className="panel-body">
        {loading && <div className="panel-loading">Loading…</div>}
        {error  && <div className="panel-error">{error}<button onClick={() => setError(null)}>✕</button></div>}
        {!loading && !error && items.length === 0 && (
          <div className="panel-empty">No items yet.</div>
        )}
        {items.map(item => (
          <div key={item.id} className="panel-card" style={{ marginBottom: 8 }}>
            {item.name}
          </div>
        ))}
      </div>
    </div>
  );
}
```
