---
triggers: ["Cloudflare Workers", "workers", "cloudflare pages", "durable objects", "KV store", "R2", "D1 database"]
tools_allowed: ["read_file", "write_file", "bash"]
category: devops
---

# Cloudflare Workers and Edge Computing

When working with Cloudflare Workers and edge computing:

1. Use `wrangler init` to scaffold projects and `wrangler dev` for local development with hot reload. Configure `wrangler.toml` with account ID, routes, and bindings: `[[kv_namespaces]]`, `[[r2_buckets]]`, `[[d1_databases]]`.

2. Write handlers using the standard Fetch API: `export default { async fetch(request, env, ctx) { return new Response("OK"); } }` — Workers run on the V8 isolate model, not Node.js, so avoid Node-specific APIs.

3. Use KV for read-heavy, eventually-consistent key-value storage (caching, config, feature flags): `await env.MY_KV.put("key", "value", {expirationTtl: 3600})`. KV writes propagate globally within 60 seconds.

4. Use Durable Objects for strongly-consistent, stateful edge computation — each object has a single-threaded execution context and transactional storage: ideal for counters, rate limiters, WebSocket coordination, and collaborative editing.

5. Store large files in R2 (S3-compatible, zero egress fees): `await env.BUCKET.put("path/file.pdf", body)`. Use presigned URLs for direct client uploads and downloads to avoid Worker CPU time limits.

6. Use D1 (SQLite at the edge) for relational data that needs SQL queries: `const result = await env.DB.prepare("SELECT * FROM users WHERE id = ?").bind(userId).first()`. Run migrations with `wrangler d1 migrations apply`.

7. Respect the 128MB memory limit and 30-second CPU time limit (paid plan). Use `ctx.waitUntil(promise)` for background work (logging, analytics) that should not block the response.

8. Implement caching with the Cache API for expensive computations: `const cache = caches.default; let response = await cache.match(request); if (!response) { response = await generateResponse(); ctx.waitUntil(cache.put(request, response.clone())); }`.

9. Deploy Cloudflare Pages for full-stack apps — static assets are served from the CDN, and `functions/` directory maps to Workers routes automatically: `functions/api/users/[id].ts` becomes `GET /api/users/:id`.

10. Use Worker-to-Worker service bindings for microservice architectures — call other Workers without HTTP overhead: `const response = await env.AUTH_SERVICE.fetch(request)`. Bindings are configured in `wrangler.toml`.

11. Handle secrets with `wrangler secret put SECRET_NAME` — never commit secrets to `wrangler.toml`. Access via `env.SECRET_NAME` in the handler. Use `.dev.vars` for local development secrets.

12. Test Workers with Miniflare (local simulator) or Vitest with `@cloudflare/vitest-pool-workers` for unit tests that run against real bindings: `const { env } = getMiniflareBindings(); await env.KV.put("test", "value"); expect(await env.KV.get("test")).toBe("value")`.
