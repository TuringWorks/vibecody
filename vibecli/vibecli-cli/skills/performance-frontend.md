---
triggers: ["bundle size", "lazy loading", "Core Web Vitals", "code splitting", "tree shaking", "LCP", "CLS", "FID"]
tools_allowed: ["read_file", "write_file", "bash"]
category: performance
---

# Frontend Performance

When optimizing frontend performance:

1. Core Web Vitals targets: LCP < 2.5s, FID/INP < 200ms, CLS < 0.1
2. Code splitting: use dynamic `import()` for route-based splitting — load pages on demand
3. Tree shaking: use ES modules (`import/export`) — bundlers eliminate unused exports
4. Images: use `<img loading="lazy">`, `srcset` for responsive, WebP/AVIF formats, proper sizing
5. Fonts: use `font-display: swap`, preload critical fonts, subset to needed characters
6. Bundle analysis: use `webpack-bundle-analyzer` or `vite-bundle-visualizer` to find bloat
7. Minimize main-thread work: defer non-critical JS with `<script defer>` or `requestIdleCallback`
8. Use CSS containment (`contain: content`) for isolated components — helps layout performance
9. Virtualize long lists: `react-window` or `tanstack-virtual` — only render visible items
10. Preload critical resources: `<link rel="preload" as="script" href="critical.js">`
11. Avoid layout shifts: set explicit `width`/`height` on images, reserve space for dynamic content
12. Use `Lighthouse` in CI to track performance regressions — set score thresholds
