---
triggers: ["Hono", "hono framework", "hono middleware", "hono cloudflare", "hono bun", "hono deno"]
tools_allowed: ["read_file", "write_file", "bash"]
requires_bins: ["node"]
category: javascript
---

# Hono Framework (Multi-Runtime)

When working with Hono:

1. Create the app with the appropriate adapter: `new Hono()` for generic use, or import from `hono/bun`, `hono/cloudflare-workers`, `hono/deno` to get runtime-specific types and optimizations.
2. Define routes with chaining: `app.get('/users/:id', (c) => c.json(data))` — use `c.req.param('id')` for path params, `c.req.query('q')` for query strings, and `await c.req.json()` for body parsing.
3. Use Hono's built-in middleware stack: `app.use('*', logger())`, `app.use('*', cors())`, `app.use('/api/*', bearerAuth({ token }))` — middleware applies to matching paths in registration order.
4. Leverage `hono/validator` with Zod for typed validation: `app.post('/users', zValidator('json', userSchema), (c) => { const data = c.req.valid('json') })` — invalid requests get automatic 400 responses.
5. Group related routes with `app.route('/api/v1', apiRouter)` to mount sub-applications — each group can have its own middleware without affecting other route groups.
6. Access Cloudflare Workers bindings via `c.env`: `c.env.MY_KV`, `c.env.MY_D1`, `c.env.MY_R2` — type them with `type Bindings = { MY_KV: KVNamespace }` and pass as `Hono<{ Bindings: Bindings }>`.
7. Use `c.header()` to set response headers, `c.status(201)` for status codes, and return with `c.json()`, `c.text()`, `c.html()`, or `c.body()` for different content types.
8. Implement custom middleware as `async (c, next) => { /* before */ await next() /* after */ }` — access the response after `next()` for logging, timing, or response transformation.
9. Use RPC mode with `hc` client for end-to-end type safety: export route types with `export type AppType = typeof route` and create typed clients with `hc<AppType>('http://localhost')`.
10. Handle errors globally with `app.onError((err, c) => c.json({ error: err.message }, 500))` and use `HTTPException` for controlled error responses: `throw new HTTPException(404, { message: 'Not found' })`.
11. Serve static files with `app.use('/static/*', serveStatic({ root: './' }))` — import from `hono/bun`, `hono/cloudflare-workers`, or `@hono/node-server/serve-static` depending on runtime.
12. Write tests using the app directly: `const res = await app.request('/users', { method: 'GET' })` — this works without any server startup and returns standard `Response` objects for assertion.
