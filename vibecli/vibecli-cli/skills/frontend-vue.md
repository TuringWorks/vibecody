---
triggers: ["Vue", "vue", "vue3", "composition api", "Pinia", "Nuxt", "vue router", "vue composable", "vue reactive"]
tools_allowed: ["read_file", "write_file", "bash"]
requires_bins: ["node"]
category: frontend
---

# Vue.js

When working with Vue:

1. Use `<script setup>` with Composition API as the default; define reactive state with `ref()` for primitives and `reactive()` for objects, and access refs in templates without `.value`.
2. Extract reusable logic into composables (`use*.ts` files) that return refs and functions; composables replace mixins and enable clean, testable, and tree-shakable shared logic.
3. Manage global state with Pinia stores using `defineStore`; prefer `setup` syntax (`ref` + `computed` + functions) over options syntax, and use `storeToRefs()` to destructure without losing reactivity.
4. Configure routing with Vue Router's `createRouter`; use named routes, lazy-load pages with `() => import('./pages/Page.vue')`, and implement navigation guards (`beforeEach`) for auth checks.
5. Build full-stack apps with Nuxt 3; use `pages/` for file-based routing, `server/api/` for API endpoints, `composables/` for auto-imported composables, and `useFetch`/`useAsyncData` for SSR-safe data loading.
6. Provide deeply shared dependencies with `provide`/`inject` pattern; define typed injection keys with `InjectionKey<T>` and provide values at the app or component level to avoid prop drilling.
7. Configure Vite as the build tool (default for Vue 3); customize `vite.config.ts` with path aliases (`@/` to `src/`), env variables via `.env` files, and optimize deps with `optimizeDeps.include`.
8. Enable SSR with Nuxt or manual `@vue/server-renderer`; use `onServerPrefetch` for server-only data fetching, `useHead` for SEO meta tags, and `ClientOnly` component for browser-only content.
9. Write unit tests with Vitest and `@vue/test-utils`; mount components with `mount()`, assert emitted events with `wrapper.emitted()`, test composables by calling them inside `withSetup` helpers.
10. Integrate TypeScript by adding `lang="ts"` to `<script setup>`; use `defineProps<{ title: string }>()` for typed props, `defineEmits<{ (e: 'update', id: number): void }>()` for typed events.
11. Optimize rendering with `v-memo` for expensive list items, `shallowRef` for large objects that replace rather than mutate, and `defineAsyncComponent` for code-splitting heavy components.
12. Use Vue DevTools browser extension for debugging; inspect component tree, Pinia state, router history, and timeline events; use `console.log(toRaw(reactiveObj))` to inspect unwrapped reactive state.
