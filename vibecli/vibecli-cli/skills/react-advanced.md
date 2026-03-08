---
triggers: ["React Server Components", "RSC", "react suspense", "react error boundary", "react custom hook", "react performance", "react concurrent", "react form actions", "react testing library"]
tools_allowed: ["read_file", "write_file", "bash"]
requires_bins: ["node"]
category: frontend
---

# React Advanced Patterns

When working with advanced React patterns:

1. Use React Server Components (RSC) for data-fetching and heavy rendering logic that does not need interactivity; keep `'use client'` boundaries as low in the tree as possible to maximize the server-rendered payload and minimize client bundle size.
2. Wrap async data-fetching components in `<Suspense fallback={<Skeleton />}>` to show meaningful loading states; nest Suspense boundaries at different granularities so independent sections can stream in without blocking each other.
3. Implement Error Boundaries as class components with `getDerivedStateFromError` and `componentDidCatch`; place them at route and feature boundaries, provide a recovery UI with a retry action, and log errors to your monitoring service inside `componentDidCatch`.
4. Extract reusable stateful logic into custom hooks (`useDebounce`, `usePagination`, `useLocalStorage`) that return stable references; prefix with `use`, keep hooks composable by calling other hooks internally, and avoid side effects outside `useEffect`.
5. Apply `React.memo` to components that receive stable props and render expensively; combine with `useMemo` for derived computations and `useCallback` for event handlers passed to memoized children, but profile first to confirm the optimization is needed.
6. Optimize Context by splitting large contexts into focused providers (ThemeContext, AuthContext, ToastContext), co-locating state with its provider, and using `useSyncExternalStore` or selector patterns to prevent re-renders in consumers that only need a subset of the value.
7. Leverage concurrent features with `useTransition` for non-urgent state updates (tab switches, filter changes) and `useDeferredValue` for expensive derived renders (search results lists), keeping the UI responsive during heavy computation.
8. Use form actions with `useActionState` (or `useFormState`) for server-side form processing; return validation errors as structured objects, display field-level feedback, and use `useFormStatus` in submit buttons to show pending states without manual loading state.
9. Implement streaming SSR with `renderToPipeableStream` (Node) or `renderToReadableStream` (edge), set `bootstrapScripts` for hydration, and use `onShellReady` to start streaming the shell while Suspense boundaries resolve asynchronously.
10. Write tests with React Testing Library using `render`, `screen.getByRole`/`getByText`, and `userEvent` for interactions; test behavior and accessibility roles instead of implementation details, use `waitFor` for async assertions, and avoid querying by test IDs when semantic queries work.
11. Structure component files with collocated test (`Component.test.tsx`), types, and styles; use barrel exports sparingly to avoid tree-shaking issues, and prefer named exports over default exports for better refactoring support and IDE discoverability.
12. Handle data mutations with `useOptimisticUpdate` patterns: update the UI immediately, fire the async action, and roll back on failure; combine with `startTransition` to keep the optimistic state non-blocking and revalidate server data after successful mutation.
