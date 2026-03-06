---
triggers: ["Tailwind", "CSS", "responsive design", "dark mode", "animation CSS", "utility classes", "flexbox", "grid CSS"]
tools_allowed: ["read_file", "write_file", "bash"]
category: frontend
---

# CSS & Tailwind

When styling with CSS and Tailwind:

1. Tailwind: use utility classes directly in markup — `className="flex items-center gap-4 p-4"`
2. Responsive: mobile-first with breakpoint prefixes — `sm:`, `md:`, `lg:`, `xl:`
3. Dark mode: use `dark:` variant — `className="bg-white dark:bg-gray-900"`
4. Flexbox: `flex` for one-dimensional layouts — `justify-center`, `items-center`, `gap-*`
5. Grid: `grid` for two-dimensional layouts — `grid-cols-3`, `col-span-2`, `gap-4`
6. Custom values: use `[]` brackets — `w-[350px]`, `text-[#1a1a1a]`, `grid-cols-[1fr_2fr]`
7. Extract components: use `@apply` in CSS files or create React components for repeated patterns
8. Animations: use `transition-all duration-300` for smooth state changes; `animate-*` for keyframes
9. Use CSS custom properties for theming: `--color-primary: oklch(0.7 0.2 250);`
10. Container queries: `@container` for responsive components (not just viewport)
11. Avoid: `!important`, deeply nested selectors, styling by element type (prefer classes)
12. Performance: use `will-change` sparingly, prefer `transform`/`opacity` for animations (GPU-accelerated)
