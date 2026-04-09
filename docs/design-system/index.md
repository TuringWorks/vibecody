---
render_with_liquid: false
layout: page
title: Design System
permalink: /design-system/
---

# VibeCody Design System

A token-based design system used across all VibeCody UI surfaces. Inspired by Material Design 3, GitHub Primer, and Shopify Polaris.

---

## How It Works

All visual decisions — colors, spacing, typography, shadows, motion — are expressed as **CSS custom properties** (tokens). Component classes built on top of those tokens ensure consistent rendering across panels and themes.

| Layer | What | Where |
|---|---|---|
| **Tokens** | CSS custom properties | `vibeui/design-system/tokens.css` |
| **Components** | CSS class system | `vibeui/src/App.css` |
| **Flutter** | Dart constants | `vibemobile/lib/theme/vibe_tokens.dart` |
| **Docs** | This site | `docs/design-system/` |

---

## Foundations

| | |
|---|---|
| [Color](./foundations/color/) | Palette, semantic tokens, dark/light modes, utility classes |
| [Typography](./foundations/typography/) | Font scale, weights, hierarchy, mono vs sans |
| [Spacing](./foundations/spacing/) | 4px grid, space tokens, layout vs component spacing |
| [Elevation](./foundations/elevation/) | Shadows, glass surfaces, z-index layers |
| [Motion](./foundations/motion/) | Transition tokens, animation principles |

## Components

| | |
|---|---|
| [Panel](./components/panel/) | The core layout primitive — container/header/body/footer |
| [Button](./components/button/) | All button variants, sizes, states |
| [Input](./components/input/) | Text input, textarea, select, states |
| [Card](./components/card/) | Surface hierarchy, card variants |
| [Badge & Tag](./components/badge-tag/) | Tags (small), badges (medium), chips |
| [Progress](./components/progress/) | Progress bars, sizes, semantic colors |
| [Table](./components/table/) | Data tables, column alignment |
| [Tabs](./components/tabs/) | Tab bar and tab variants |

## Patterns

| | |
|---|---|
| [Data States](./patterns/data-states/) | Loading, empty, error — the three async states |
| [Forms](./patterns/forms/) | Form layout, validation, submit feedback |

---

## Quick Reference

### The 10 panel rules

```
1.  Root: flex: 1, minHeight: 0           (never height: 100%)
2.  Structure: container → header → body → footer
3.  Colors: CSS vars only                 (never #4caf50, #fff, etc.)
4.  Spacing: multiples of 4px             (4, 8, 12, 16, 20, 24, 32)
5.  Empty state: className="panel-empty"
6.  Loading: className="panel-loading"
7.  Error: className="panel-error"
8.  Cards: className="panel-card"
9.  Buttons: panel-btn + panel-btn-{variant}
10. Tags: className="panel-tag panel-tag-{intent}"
```

### Color quick-pick

| Need | Token |
|---|---|
| Success / pass / green | `var(--success-color)` · `var(--text-success)` · `var(--success-bg)` |
| Error / fail / red | `var(--error-color)` · `var(--text-danger)` · `var(--error-bg)` |
| Warning / amber | `var(--warning-color)` · `var(--text-warning)` · `var(--warning-bg)` |
| Info / blue | `var(--info-color)` · `var(--text-info)` · `var(--info-bg)` |
| Brand accent | `var(--accent-color)` · `var(--accent-blue)` |
| Body text | `var(--text-primary)` |
| Secondary text | `var(--text-secondary)` |
| Placeholder / disabled | `var(--text-muted)` |
| Card background | `var(--bg-secondary)` |
| Page background | `var(--bg-primary)` |

### Font size quick-pick

| Use case | Token | Value |
|---|---|---|
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
|---|---|---|
| Icon gap | `--space-1` | 4px |
| Related elements | `--space-2` | 8px |
| Card padding | `--space-3` | 12px |
| Section padding | `--space-4` | 16px |
| Large gap | `--space-5` | 20px |
| Section gap | `--space-6` | 24px |
| Empty state | `--space-8` | 32px |

---

## Minimal Panel Template

```tsx
import { useState, useCallback } from "react";
import { invoke } from "@tauri-apps/api/core";

interface Item { id: string; name: string; }

export default function MyPanel() {
  const [items, setItems] = useState<Item[]>([]);
  const [loading, setLoading] = useState(false);
  const [error, setError] = useState<string | null>(null);

  const load = useCallback(async () => {
    setLoading(true); setError(null);
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
        <button className="panel-btn panel-btn-primary panel-btn-sm"
          style={{ marginLeft: "auto" }} onClick={load} disabled={loading}>
          {loading ? "Loading…" : "↻ Refresh"}
        </button>
      </div>
      <div className="panel-body">
        {loading && <div className="panel-loading">Loading…</div>}
        {error   && <div className="panel-error">{error}</div>}
        {!loading && !error && items.length === 0 && (
          <div className="panel-empty">No items yet.</div>
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

## Cross-Platform Token Map

The same design tokens are translated to each platform:

| Token | CSS | Flutter (Dart) |
|---|---|---|
| `--bg-primary` | `#0f1117` | `VibeDarkColors.bgPrimary` |
| `--accent-blue` | `#6c8cff` | `VibeDarkColors.accentBlue` |
| `--success-color` | `var(--accent-green)` | `VibeDarkColors.successColor` |
| `--space-4` | `16px` | `VibeSpacing.s4` |
| `--font-size-md` | `13px` | `VibeFontSize.md` |
| `--radius-sm` | `6px` | `VibeRadius.sm` |
| `--transition-fast` | `0.15s ease` | `VibeDuration.fast` |
