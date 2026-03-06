---
triggers: ["react component", "useState", "useEffect", "tsx", "react hook"]
tools_allowed: ["read_file", "write_file", "bash"]
requires_bins: ["node"]
category: typescript
---

# React + TypeScript Patterns

1. Use functional components with explicit prop types: `const Foo: React.FC<Props> = ({ ... }) =>`
2. Prefer `useState` with explicit type: `useState<string>("")`
3. Use `useCallback` for handlers passed to child components
4. Use `useMemo` for expensive computations, not for simple values
5. Custom hooks should start with `use` and return a tuple or object
6. Avoid `any` — use `unknown` and narrow with type guards
7. Use `React.memo()` only when profiling shows re-render issues
8. Event handlers: `React.MouseEvent<HTMLButtonElement>`, `React.ChangeEvent<HTMLInputElement>`
