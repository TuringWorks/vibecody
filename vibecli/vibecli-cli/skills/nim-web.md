---
triggers: ["Nim", "nim lang", "jester", "prologue nim", "httpbeast", "karax"]
tools_allowed: ["read_file", "write_file", "bash"]
requires_bins: ["nim"]
category: nim
---

# Nim Web Development

When working with Nim web frameworks:

1. Use Jester for straightforward web APIs — define routes with `get "/users/@id"` blocks and access parameters via `@"id"` string interpolation inside the route handler.
2. Use Prologue for full-featured applications — it provides middleware, static file serving, session management, and template rendering out of the box with `newApp(settings)`.
3. Leverage `httpbeast` as a high-performance HTTP server for custom solutions — it uses OS-level async I/O and can serve as the backend for Jester in production.
4. Use Nim's `async`/`await` with `asyncdispatch` for non-blocking I/O — all Jester and Prologue handlers run on the async event loop, so never block with `sleep()`.
5. Define JSON serialization with `std/json` module — use `%*` for creating JSON nodes and `to()` for parsing; prefer `jsony` or `json_serialization` packages for automatic struct mapping.
6. Use Nim's strong type system with `distinct` types and `object variants` (sum types) to model domain entities — the compiler enforces exhaustive case handling in `case` statements.
7. Manage dependencies with Nimble — specify requirements in `*.nimble` file with `requires "jester >= 0.6.0"` and run `nimble install` to resolve and fetch packages.
8. Add middleware in Prologue with `app.use(loggingMiddleware)` — write middleware as procs that take `(ctx: Context, next: MiddlewareHandler)` and call `await next(ctx)` to continue.
9. Use Nim's `db_postgres` or `db_sqlite` modules for database access — prepare statements with `db.prepare("SELECT * FROM users WHERE id = $1")` to prevent SQL injection.
10. Compile for production with `nim c -d:release -d:lto --opt:speed src/app.nim` — enable link-time optimization and strip debug symbols for minimal binary size.
11. Use Karax for single-page applications compiled to JavaScript — share type definitions between server (compiled to C) and client (compiled to JS) via common modules.
12. Test HTTP handlers using Nim's `unittest` module — create an `AsyncHttpClient`, send requests to the running test server, and assert on response code and body content.
