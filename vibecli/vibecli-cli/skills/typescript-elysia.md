---
triggers: ["Elysia", "elysia bun", "elysia plugin", "elysia eden", "bun web framework"]
tools_allowed: ["read_file", "write_file", "bash"]
requires_bins: ["bun"]
category: typescript
---

# Elysia (Bun) Framework

When working with Elysia on Bun:

1. Define routes with full type inference: `app.get('/users/:id', ({ params: { id } }) => getUserById(id))` — Elysia infers parameter types, request body, and response types end-to-end without manual annotations.
2. Use `t` (TypeBox) for request validation: `app.post('/users', ({ body }) => createUser(body), { body: t.Object({ name: t.String(), email: t.String({ format: 'email' }) }) })` — invalid requests return 422 automatically.
3. Create reusable plugins with `new Elysia().decorate('db', database).macro(...)` and apply them with `app.use(myPlugin)` — plugins encapsulate state, decorators, and routes into composable units.
4. Use Eden Treaty for end-to-end type-safe client calls: `const client = treaty<typeof app>('localhost:3000')` then `const { data } = await client.users({ id: '1' }).get()` — types flow from server to client.
5. Implement authentication with the `derive` lifecycle: `app.derive(({ headers }) => { return { user: verifyToken(headers.authorization) } })` — derived values are available in all downstream handlers with full type safety.
6. Use `guard` to apply validation and hooks to groups of routes: `app.guard({ beforeHandle: authCheck, body: baseSchema }, (app) => app.get('/admin', handler))` — all routes inside inherit the guard's configuration.
7. Handle errors with `onError`: `app.onError(({ code, error }) => { if (code === 'VALIDATION') return { error: error.message } })` — use `code` to distinguish between validation, not-found, and internal errors.
8. Use `state` for mutable shared state: `app.state('counter', 0)` then access with `({ store: { counter } }) => store.counter++` — state is typed and tracked across the application lifecycle.
9. Group routes with `app.group('/api/v1', (app) => app.get('/users', handler).post('/users', handler))` — groups share a prefix and can have their own middleware and guards.
10. Leverage Bun's native performance: use `Bun.file()` for static file serving, `Bun.write()` for file uploads, and `Bun.sql` for database queries — these bypass Node.js compatibility layers.
11. Use lifecycle hooks in order: `onRequest` (raw), `onParse` (body), `onTransform` (before validation), `beforeHandle` (auth), handler, `afterHandle` (transform response), `onResponse` (logging/cleanup).
12. Test with Bun's built-in test runner: `const res = await app.handle(new Request('http://localhost/users'))` then `expect(res.status).toBe(200)` — no server startup needed, tests run at native speed.
