---
triggers: ["next.js", "nextjs", "App Router", "Server Component", "SSR", "SSG", "API route next", "use server"]
tools_allowed: ["read_file", "write_file", "bash"]
requires_bins: ["node"]
category: typescript
---

# TypeScript Next.js

When building with Next.js App Router:

1. Default to Server Components — add `"use client"` only when you need interactivity or browser APIs
2. Use `app/` directory with `page.tsx`, `layout.tsx`, `loading.tsx`, `error.tsx` conventions
3. Fetch data directly in Server Components — no need for `useEffect` or `getServerSideProps`
4. Use `"use server"` for Server Actions — form mutations without API routes
5. Use `generateStaticParams()` for static generation of dynamic routes
6. Implement `loading.tsx` for Suspense boundaries and `error.tsx` for error boundaries
7. Use `next/image` for automatic image optimization and lazy loading
8. Use `next/link` for client-side navigation — prefetches linked pages automatically
9. Route handlers in `app/api/*/route.ts` — export `GET`, `POST`, etc. as named functions
10. Use `cookies()` and `headers()` from `next/headers` in Server Components
11. Metadata API: export `metadata` object or `generateMetadata()` function for SEO
12. Use `revalidatePath()` / `revalidateTag()` for on-demand ISR cache invalidation
