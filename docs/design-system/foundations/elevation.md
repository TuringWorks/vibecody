---
layout: page
title: Elevation — Design System
permalink: /design-system/foundations/elevation/
---

# Elevation

Elevation creates visual hierarchy through shadows and background layering. VibeUI uses four background levels and three shadow levels.

---

## Background Layers

```
--bg-primary    Page canvas (deepest)
--bg-secondary  Cards, panel surfaces
--bg-tertiary   Inputs, hover backgrounds
--bg-elevated   Modals, dropdowns, tooltips (highest)
```

Rule: elements should always sit on a background **at least one level lighter** than their parent.

---

## Shadow Tokens

| Token | Value | Use |
|---|---|---|
| `--elevation-1` | `0 1px 2px rgba(0,0,0,0.3)` | Subtle — list items, row hover |
| `--elevation-2` | `0 4px 12px rgba(0,0,0,0.35)` | Standard — cards, dropdowns |
| `--elevation-3` | `0 8px 30px rgba(0,0,0,0.45)` | Deep — modals, command palette |
| `--card-shadow` | Combined 1px + 16px | Default card shadow |
| `--glow-accent` | `0 0 20px rgba(108,140,255,0.15)` | Accent button hover glow |

---

## Glass / Frosted Surface

```css
background: var(--glass-bg);    /* rgba(22,24,33,0.75) */
border: 1px solid var(--glass-border);
backdrop-filter: blur(var(--glass-blur));
```

Use for floating surfaces that should show content behind them (sidebars, overlays).

---

## Z-Index Layers

| Layer | Z-index | Examples |
|---|---|---|
| Base content | 0 | Panels, cards |
| Sticky elements | 10 | Panel headers on scroll |
| Dropdowns | 100 | Select menus, autocomplete |
| Modals | 1000 | Settings, dialogs |
| Toasts | 9999 | Notifications |
