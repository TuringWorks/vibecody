---
triggers: ["WCAG", "accessibility test", "axe-core", "screen reader", "a11y", "aria", "keyboard navigation"]
tools_allowed: ["read_file", "write_file", "bash"]
category: testing
---

# Accessibility Testing

When testing for accessibility:

1. Use `axe-core` for automated WCAG 2.1 AA compliance checking — integrate into CI
2. Test keyboard navigation: Tab order, Enter/Space activation, Escape to close, arrow keys in lists
3. Every interactive element must have an accessible name — use `aria-label` or visible text
4. Test with screen readers: VoiceOver (macOS), NVDA (Windows), Orca (Linux)
5. Color contrast: 4.5:1 minimum for normal text, 3:1 for large text (WCAG AA)
6. Images: use descriptive `alt` text; decorative images get `alt=""`
7. Forms: associate `<label>` with inputs via `for`/`id`; use `aria-describedby` for help text
8. Use semantic HTML: `<button>` not `<div onclick>`, `<nav>`, `<main>`, `<header>`
9. Focus management: move focus to new content (modals, notifications); restore on close
10. Test zoom: content must remain usable at 200% zoom without horizontal scrolling
11. Use `role`, `aria-expanded`, `aria-selected`, `aria-live` for dynamic content updates
12. Playwright/Cypress: use `@axe-core/playwright` for integrated accessibility testing in E2E
