---
triggers: ["Deno", "deno serve", "oak deno", "deno deploy", "fresh deno", "deno permissions"]
tools_allowed: ["read_file", "write_file", "bash"]
requires_bins: ["deno"]
category: typescript
---

# Deno and Oak Framework

When working with Deno and its web ecosystem:

1. Use `Deno.serve()` for the built-in HTTP server: `Deno.serve({ port: 8000 }, (req) => new Response("OK"))` — this is the fastest option and uses Deno's optimized Rust-backed HTTP stack.
2. Apply the principle of least privilege with permissions: run with `--allow-net=0.0.0.0:8000 --allow-read=./public --allow-env=DATABASE_URL` — never use `--allow-all` in production.
3. In Oak, create middleware with `app.use(async (ctx, next) => { await next(); })` — use the context object for `ctx.request`, `ctx.response`, and `ctx.state` for request-scoped data.
4. Define Oak routes with the Router class: `router.get('/users/:id', async (ctx) => { ctx.response.body = await getUser(ctx.params.id) })` and register with `app.use(router.routes()).use(router.allowedMethods())`.
5. In Fresh, define routes as files in `routes/`: `routes/api/users/[id].ts` exports handlers like `export const handler: Handlers = { GET(req, ctx) { return new Response(JSON.stringify(data)) } }`.
6. Use Fresh islands for interactive components: place them in `islands/` and they hydrate on the client while the rest of the page stays server-rendered — this minimizes client-side JavaScript.
7. Import dependencies via URLs or `deno.json` imports map: `{ "imports": { "oak": "jsr:@oak/oak@^17", "zod": "npm:zod@^3" } }` — use `jsr:` for Deno-native packages and `npm:` for Node compatibility.
8. Use `Deno.KV` for built-in key-value storage: `const kv = await Deno.openKv()`, `await kv.set(["users", id], user)`, `const entry = await kv.get(["users", id])` — works locally and on Deno Deploy.
9. Handle environment config with `Deno.env.get("KEY")` and use `.env` files with `import "jsr:@std/dotenv/load"` — Deno requires explicit `--allow-env` permission for each variable or a blanket flag.
10. Write tests with `Deno.test()`: `Deno.test("handler returns 200", async () => { const res = await handler(new Request("http://localhost/")); assertEquals(res.status, 200); })` — run with `deno test`.
11. Deploy to Deno Deploy with `deployctl deploy --project=myapp --entrypoint=main.ts` — the platform supports `Deno.serve`, `Deno.KV`, and `BroadcastChannel` for edge-native applications.
12. Use `deno compile --output=server --allow-net main.ts` to produce a single self-contained executable with embedded V8 — distribute without requiring Deno installed on the target machine.
