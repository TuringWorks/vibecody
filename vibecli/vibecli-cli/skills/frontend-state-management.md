---
triggers: ["Redux", "Zustand", "Jotai", "TanStack Query", "state management", "React state", "global state"]
tools_allowed: ["read_file", "write_file", "bash"]
category: frontend
---

# Frontend State Management

When managing state in React applications:

1. Start with local state (`useState`) — lift up only when siblings need the same data
2. Server state: use TanStack Query (React Query) — handles caching, refetching, loading states
3. Zustand: lightweight global store — `create((set) => ({ count: 0, inc: () => set(s => ({ count: s.count + 1 })) }))`
4. Jotai: atomic state — bottom-up approach, each atom is independent
5. Redux Toolkit: for complex state with strict patterns — slices, reducers, async thunks
6. Context API: use for infrequently-changing values (theme, auth, locale) — NOT for high-frequency updates
7. TanStack Query: separate server state from client state — don't duplicate API data in Redux
8. URL state: use URL params/search params for shareable, bookmarkable state (filters, pagination)
9. Form state: use `react-hook-form` or `formik` — don't manage form state manually
10. Avoid prop drilling: use composition (children), context, or state library — not 5-level prop chains
11. Derived state: compute from existing state (`useMemo`) — don't store computed values
12. Immutable updates: never mutate state directly — use spread, `structuredClone`, or Immer
