---
render_with_liquid: false
layout: page
title: Elevation — Design System
permalink: /design-system/foundations/elevation/
---

# Elevation

Elevation communicates depth and hierarchy through shadows, backgrounds, and blur effects. VibeUI has three shadow levels, a glass surface system, and a Z-index layer map.

---

## Shadow Tokens

```
Level   Token           Value                                          Use
──────────────────────────────────────────────────────────────────────────────
0       (none)          —                                              Flat surfaces, row backgrounds
1       --elevation-1   0 1px 2px rgba(0,0,0,0.30)                   Cards, buttons (resting)
2       --elevation-2   0 4px 12px rgba(0,0,0,0.35)                  Buttons (hover), popovers
3       --elevation-3   0 8px 30px rgba(0,0,0,0.45)                  Modals, drawers, tooltips
─       --glow-accent   0 0 20px rgba(108,140,255,0.15)              Button hover glow
─       --card-shadow   0 1px 3px rgba(0,0,0,0.4),                   Heavy card emphasis
                        0 4px 16px rgba(0,0,0,0.25)
```

---

## Background Depth Hierarchy

Higher value = further from the canvas = visually "higher".

```
Depth  Token            Dark value    Light value    Use
─────────────────────────────────────────────────────────────────────
0      --bg-primary     #0f1117       #fafbfd        Page canvas, deepest bg
1      --bg-secondary   #161821       #f0f1f5        Panel body, card surfaces
2      --bg-tertiary    #1c1f2b       #e6e8ef        Inputs, hover states, sub-cards
3      --bg-elevated    #222638       #ffffff        Modals, dropdowns, tooltips
```

Rules:
- A card uses `--bg-secondary` when sitting on `--bg-primary`
- An input inside a card uses `--bg-tertiary` (one step above the card)
- A modal uses `--bg-elevated` (always highest)
- Never place a `--bg-secondary` element inside another `--bg-secondary` element (they merge visually)

```tsx
// ✅ Correct layering
// Panel body (bg-primary) → card (bg-secondary) → input inside card (bg-tertiary)
<div style={{ background: "var(--bg-primary)" }}>         {/* panel body */}
  <div className="panel-card">                            {/* bg-secondary */}
    <input className="panel-input" />                    {/* bg-tertiary (from CSS) */}
  </div>
</div>

// ❌ Wrong — card inside card, same level
<div className="panel-card">
  <div className="panel-card"> ... </div>   {/* both bg-secondary — no visual separation */}
</div>

// ✅ Correct — nested distinction
<div className="panel-card">
  <div style={{ background: "var(--bg-primary)", borderRadius: "var(--radius-xs)", padding: 8 }}>
    inner content
  </div>
</div>
```

---

## Glass / Frosted Surfaces

For surfaces that need a translucent, blurred effect (e.g. floating panels, headers).

```
--glass-bg:     rgba(22,24,33,0.75)        Semi-transparent bg
--glass-border: rgba(255,255,255,0.08)     Subtle edge
--glass-blur:   16px                        backdrop-filter value
```

```tsx
// Glass card
<div style={{
  background: "var(--glass-bg)",
  border: "1px solid var(--glass-border)",
  backdropFilter: "blur(var(--glass-blur))",
  borderRadius: "var(--radius-md)",
}}>
  ...
</div>
```

Use sparingly — only for surfaces that float over content (headers, tooltips, command palettes). Do not use glass inside panel bodies.

---

## Radius Scale

```
Token        Value   Use
--radius-xs  3px     Tags, tiny badges, micro elements
--radius-sm  6px     Buttons, cards, inputs (primary default)
--radius-md  10px    Large cards, modals, popover containers
--radius-lg  14px    Panels, drawers, large surfaces
--radius-xl  20px    Pill shapes, large chips
```

```tsx
// Most elements — use radius-sm
<div className="panel-card">               {/* uses --radius-sm */}
<button className="panel-btn">             {/* uses --radius-sm */}
<input className="panel-input">            {/* uses --radius-sm */}

// Tags — extra small
<span className="panel-tag">              {/* uses --radius-xs (3px) */}

// Larger containers
<div style={{ borderRadius: "var(--radius-md)" }}>   {/* 10px */}
<div style={{ borderRadius: "var(--radius-lg)" }}>   {/* 14px */}
```

---

## Z-Index Layers

```
Layer           Value     Use
────────────────────────────────────────────────────────────
Base content    0–9       Panel body, cards, rows
Panel header    10–49     Sticky panel headers
Sidebar/nav     50–99     Activity bar, sidebar
App header      100       Global header bar
Overlays        200–999   Popovers, tooltips, dropdowns
Modals          1000–9999 Dialog boxes, drawers
Toast/alerts    5000       Notification toasts
Tour            10000+    Onboarding overlays
```

---

## Button Elevation

Buttons use elevation to signal interactivity:

| State | Shadow |
|-------|--------|
| Resting | `--elevation-1` |
| Hover | `--elevation-2` + `--glow-accent` |
| Active | `--elevation-1` |
| Disabled | none |

This is handled automatically by `.btn-primary` and `.btn-secondary`. For `.panel-btn`, transitions are simpler (opacity-based) for panel density.
