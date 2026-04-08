---
layout: page
title: Motion — Design System
permalink: /design-system/foundations/motion/
---

# Motion

Motion in VibeUI is intentional and subtle. Transitions communicate state changes. Animations are never decorative — they carry meaning.

---

## Transition Tokens

```
Token                Value                                    Use
─────────────────────────────────────────────────────────────────────────────────
--transition-fast    0.15s cubic-bezier(0.4, 0, 0.2, 1)     Hover, focus, button press
--transition-smooth  0.25s cubic-bezier(0.4, 0, 0.2, 1)     Progress bar fill, tab switch
--transition-spring  0.35s cubic-bezier(0.34, 1.56, 0.64, 1) Entrance animations, dialogs
```

The easing (`0.4, 0, 0.2, 1`) is Material Design's "standard" easing — fast out of a state, decelerates into the new one. The spring variant overshoots slightly, communicating elasticity.

---

## When to Animate

| Trigger | Transition | Duration |
|---------|-----------|----------|
| Button hover/active | opacity, transform | fast (0.15s) |
| Button hover glow | box-shadow | fast (0.15s) |
| Input focus ring | border-color, box-shadow | fast (0.15s) |
| Progress bar fill | width | smooth (0.25s) |
| Tab switch | background, border-color | fast (0.15s) |
| Modal/drawer open | transform, opacity | spring (0.35s) |
| Theme toggle | background-color, color | smooth (0.25s) |
| Sidebar resize | width | smooth (0.25s) |

## When NOT to Animate

- Data loading (use a loading indicator, not an animation)
- List item mount/unmount (adds visual noise in dense panels)
- Scroll position
- Error messages (show immediately)

---

## Button Transitions

`.panel-btn` uses:
```css
transition: opacity var(--transition-fast), background var(--transition-fast);
```

`.btn-primary` / `.btn-secondary` use:
```css
transition: all var(--transition-fast);
/* hover: translateY(-1px) + elevation-2 + glow */
/* active: translateY(0) + elevation-1 */
```

---

## Progress Bar Animation

All progress bar fills use `--transition-smooth`:

```css
.progress-bar-fill {
  transition: width var(--transition-smooth);
}
```

When the value updates in React, the bar animates automatically. No JS needed.

---

## Loading Spinner

The `.spin` class provides a CSS rotation animation:

```tsx
<span className="spin" style={{ display: "inline-block", fontSize: 14 }}>⟳</span>

// Or with a Lucide icon
<Icon name="loader" size={14} className="spin" />
```

```css
@keyframes spin {
  to { transform: rotate(360deg); }
}
.spin { animation: spin 1s linear infinite; }
```

---

## Theme Transition

Global theme changes (`body`, `.header`, `.sidebar`, etc.) use `--transition-smooth` on `background-color`, `color`, and `border-color`. This is set in `App.css` globally — panels inherit it automatically.

---

## Reduced Motion

Always respect user preference. Progress bars, loading indicators, and any CSS animation should respect:

```css
@media (prefers-reduced-motion: reduce) {
  .progress-bar-fill { transition: none; }
  .spin { animation: none; }
}
```

Currently respected at the global level. When adding new CSS animations, add a `prefers-reduced-motion` override.
