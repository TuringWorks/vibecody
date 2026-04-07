---
layout: page
title: Motion — Design System
permalink: /design-system/foundations/motion/
---

# Motion

Motion should be purposeful and fast. VibeUI uses three transition speeds covering most interactive states.

---

## Transition Tokens

| Token | Value | Use |
|---|---|---|
| `--transition-fast` | `0.15s cubic-bezier(0.4,0,0.2,1)` | Hover states, color changes |
| `--transition-smooth` | `0.25s cubic-bezier(0.4,0,0.2,1)` | Panel open/close, tabs |
| `--transition-spring` | `0.35s cubic-bezier(0.34,1.56,0.64,1)` | Scale interactions, bouncy |

---

## Usage

```css
/* Hover state */
transition: background var(--transition-fast);

/* Panel slide */
transition: transform var(--transition-smooth);

/* Button press (spring) */
transition: transform var(--transition-spring);
```

---

## Principles

1. **Fast by default** — most hover/focus transitions use `--transition-fast`
2. **No layout animation** — never animate `width`, `height`, or `margin` (causes reflow)
3. **Respect reduced motion** — always wrap decorative animation in `@media (prefers-reduced-motion: no-preference)`
4. **Spring for delight** — use `--transition-spring` sparingly for scale transforms on buttons and icons

---

## Reduced Motion

```css
@media (prefers-reduced-motion: no-preference) {
  .animated-element {
    animation: slideIn 0.3s var(--transition-smooth);
  }
}
```
