---
triggers: ["DOM", "event listener", "querySelector", "fetch API", "Web Worker", "browser API", "localStorage", "addEventListener"]
tools_allowed: ["read_file", "write_file", "bash"]
category: javascript
---

# JavaScript DOM & Browser APIs

When working with DOM and browser APIs:

1. Use `document.querySelector()` / `querySelectorAll()` — avoid `getElementById` for consistency
2. Use event delegation: attach one listener to parent, check `event.target` — avoids memory leaks
3. Prefer `addEventListener` over inline `onclick` — supports multiple handlers and options
4. Use `{ passive: true }` for scroll/touch listeners to improve performance
5. Use `IntersectionObserver` for lazy loading and infinite scroll — avoid scroll event polling
6. Use `MutationObserver` to watch for DOM changes — avoid polling with timers
7. `fetch()` best practices: check `response.ok`, use `AbortController` for cancellation and timeouts
8. Use `structuredClone()` for deep copying objects — faster and more correct than JSON.parse/stringify
9. Use `Web Workers` for CPU-intensive computation — keep the main thread responsive
10. Use `requestAnimationFrame()` for visual updates — never use `setInterval` for animations
11. `localStorage`/`sessionStorage`: store strings only, JSON.stringify for objects, handle QuotaExceededError
12. Use `ResizeObserver` for responsive components instead of window resize events
