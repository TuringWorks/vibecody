---
triggers: ["D language web", "vibe.d", "vibed", "dlang web", "dlang server"]
tools_allowed: ["read_file", "write_file", "bash"]
category: d
---

# D Language Web (vibe.d)

When working with D and vibe.d for web development:

1. Define route handlers by creating a class that inherits from `URLRouter` interface or use the functional style: `router.get("/users/:id", &handleUser)` with `void handleUser(scope HTTPServerRequest req, scope HTTPServerResponse res)`.
2. Use vibe.d's built-in fiber-based concurrency — every request runs in its own fiber, so blocking I/O (database, file) automatically yields to other fibers without async/await syntax.
3. Parse JSON request bodies with `req.readJson()` which returns a `Json` value; access fields with `json["field"].get!string` and use `deserializeJson!MyStruct(json)` for typed deserialization.
4. Configure the server in `dub.json` or `dub.sdl` for dependencies, and use `HTTPServerSettings` to set port, bind address, worker threads, and TLS: `settings.tlsEncryption = TLSEncryption.tls12`.
5. Serve static files with `router.get("/static/*", serveStaticFiles("public/"))` — vibe.d handles caching headers, MIME types, and range requests automatically.
6. Use vibe.d's built-in connection pool for database access: `auto pool = new ConnectionPool!PGConnection(4, connStr)` and acquire connections with `pool.lockConnection()`.
7. Implement middleware using `router.any("*", &middlewareHandler)` registered before route handlers — check auth, set headers, or log requests and call `next()` to continue the chain.
8. Use D's `mixin` and compile-time features for route generation: `mixin GenRoutes!(UserController)` can auto-generate REST endpoints from a class's public methods with CTFE.
9. Handle WebSocket connections with `router.get("/ws", handleWebSockets(&onWebSocket))` and process messages in `void onWebSocket(scope WebSocket socket)` using `socket.receive()` and `socket.send()`.
10. Use Diet templates (vibe.d's template engine) for HTML rendering: `res.render!("page.dt", name, items)` — Diet compiles templates to D code at compile time for maximum performance.
11. Build for production with `dub build --build=release` which enables LDC optimizations; deploy the single static binary with `dub build --build=release --compiler=ldc2` for best performance.
12. Write tests using D's built-in `unittest` blocks and vibe.d's `requestHTTP` for integration tests: `requestHTTP("http://localhost:8080/api", (scope req) {}, (scope res) { assert(res.statusCode == 200); })`.
