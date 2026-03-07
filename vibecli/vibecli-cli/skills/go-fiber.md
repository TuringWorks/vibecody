---
triggers: ["Fiber", "gofiber", "fiber v2", "fiber middleware", "fiber handler"]
tools_allowed: ["read_file", "write_file", "bash"]
requires_bins: ["go"]
category: go
---

# Go Fiber Framework

When working with Fiber:

1. Use `app.Use()` for middleware chaining and apply rate limiting, CORS, and recovery middleware from `gofiber/contrib` before route registration to ensure consistent request processing.
2. Return errors from handlers with `c.Status(code).JSON(fiber.Map{...})` instead of panicking; use a custom `ErrorHandler` in `fiber.Config` for centralized error formatting.
3. Parse and validate request bodies with `c.BodyParser(&struct)` combined with `go-playground/validator`; define validation tags on struct fields and call `validate.Struct()` immediately after parsing.
4. Group routes with `app.Group("/api/v1")` and attach group-specific middleware (auth, logging) to keep route definitions clean and versioned.
5. Use `c.Locals(key, value)` to pass data between middleware and handlers (e.g., authenticated user), and type-assert carefully on retrieval to avoid runtime panics.
6. Configure `Prefork: true` in `fiber.Config` for multi-process mode in production, but disable it during development and testing since it forks the process and breaks debuggers.
7. Serve static files with `app.Static("/", "./public", fiber.Static{Compress: true, CacheDuration: 10 * time.Minute})` and set `MaxAge` headers for browser caching in production.
8. Use `fasthttp.RequestCtx` awareness: Fiber runs on fasthttp, so do not hold references to `c.Body()` or `c.Params()` after the handler returns; copy values with `utils.CopyString()` if needed asynchronously.
9. Write tests using `app.Test(httptest.NewRequest(...))` which returns `*http.Response` directly, avoiding the need for a running server and enabling fast unit tests.
10. Implement graceful shutdown with `signal.NotifyContext` and call `app.ShutdownWithContext(ctx)` to drain active connections before exiting.
11. Use Fiber's built-in `Monitor` middleware at a `/metrics` endpoint during development and switch to Prometheus middleware (`fiberprometheus`) in production for proper observability.
12. Set `fiber.Config{BodyLimit: 4 * 1024 * 1024}` and other sensible defaults explicitly; tune `ReadBufferSize`, `Concurrency`, and `IdleTimeout` based on load testing with `vegeta` or `hey`.
