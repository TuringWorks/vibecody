---
triggers: ["lambda function", "function composition", "point-free", "combinators", "lambda calculus", "closure patterns", "partial application"]
tools_allowed: ["read_file", "write_file", "bash"]
category: architecture
---

# Lambda and Function Composition Patterns

When working with lambda functions and composition patterns:

1. Build pipelines with function composition — chain small transformations left-to-right using `pipe` or right-to-left using `compose`: `const pipeline = pipe(trim, toLowerCase, slugify)` reads as a sequence of steps applied in order.

2. Use partial application to specialize generic functions: `const fetchJson = partial(fetch, {headers: {"Content-Type": "application/json"}})` — this creates reusable, preconfigured variants without wrapper functions.

3. Write point-free style when it improves clarity — `users.filter(isActive)` is cleaner than `users.filter(u => isActive(u))`. Avoid point-free when it obscures intent or requires contorted combinators.

4. Apply the identity combinator (`I = x => x`) as a default transformer or no-op callback. Use the constant combinator (`K = x => _ => x`) to ignore arguments: `array.map(K(defaultValue))` fills with a constant.

5. Use closures to capture configuration and create factory functions: `function makeLogger(level) { return (msg) => console.log(\`[\${level}] \${msg}\`); }` — the returned function closes over `level` without exposing it.

6. Implement the flip combinator (`flip = f => (a, b) => f(b, a)`) to adapt function argument order for composition — useful when piping data through functions that expect the data argument in different positions.

7. Apply the S combinator pattern for functions that need the same argument twice: `const on = (f, g) => (a, b) => f(g(a), g(b))` — e.g., `const compareByAge = on(subtract, prop("age"))` for sorting.

8. Use thunks (zero-argument lambdas) to defer computation: `const lazyExpensive = () => computeExpensiveResult()` — evaluate only when needed, useful for conditional initialization and lazy sequences.

9. Build recursive patterns with Y-combinator or trampolining to avoid stack overflow: `const trampoline = fn => { let result = fn(); while (typeof result === "function") result = result(); return result; }`.

10. Leverage closure-based module pattern for encapsulation without classes: `const counter = (() => { let n = 0; return { inc: () => ++n, get: () => n }; })()` — state is private, interface is explicit.

11. Compose predicates using logical combinators: `const both = (f, g) => x => f(x) && g(x)`, `const either = (f, g) => x => f(x) || g(x)`, `const complement = f => x => !f(x)` — build complex filters from simple checks.

12. Apply memoization as a higher-order function wrapping pure functions: `const memoize = fn => { const cache = new Map(); return (...args) => { const key = JSON.stringify(args); if (!cache.has(key)) cache.set(key, fn(...args)); return cache.get(key); }; }` — only safe for pure functions with serializable arguments.
