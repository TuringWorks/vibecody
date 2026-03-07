---
triggers: ["Echo framework", "echo golang", "echo middleware", "echo group routes"]
tools_allowed: ["read_file", "write_file", "bash"]
requires_bins: ["go"]
category: go
---

# Go Echo Framework

When working with Echo:

1. Structure applications using `echo.New()` in a factory function that accepts dependencies (DB, logger, config), registers middleware and routes, and returns the configured `*echo.Echo` instance for testability.
2. Use Echo's built-in middleware stack in order: `middleware.Recover()`, `middleware.RequestID()`, `middleware.Logger()`, then CORS and auth, so panics are caught and every request is traced.
3. Bind and validate requests in one step by implementing `echo.Validator` with `go-playground/validator` and registering it via `e.Validator = &CustomValidator{}`; then call `c.Bind(&req)` followed by `c.Validate(req)`.
4. Group routes with `e.Group("/api", authMiddleware)` and nest sub-groups for versioning; attach group-specific middleware rather than checking auth inside each handler.
5. Return errors from handlers as `echo.NewHTTPError(code, message)` and register a custom `HTTPErrorHandler` on the Echo instance to format all error responses consistently as JSON.
6. Use `c.Set(key, value)` and `c.Get(key)` for request-scoped data passing between middleware and handlers; create typed helper functions to avoid string key typos and unsafe assertions.
7. Serve WebSocket connections using the `labstack/echo/v4/middleware` websocket upgrader or integrate `gorilla/websocket` via a handler; always defer `ws.Close()` and handle read/write deadlines.
8. Write handler tests by creating `httptest.NewRequest`, `httptest.NewRecorder`, then `e.NewContext(req, rec)` to get a fully functional context without starting a server.
9. Enable gzip compression with `middleware.GzipWithConfig(middleware.GzipConfig{Level: 5})` and set `middleware.BodyLimit("2M")` to protect against oversized payloads.
10. Use `e.Pre(middleware.RemoveTrailingSlash())` to normalize URLs and prevent duplicate route registrations that differ only by trailing slash.
11. Implement graceful shutdown by running `e.Start()` in a goroutine, listening for OS signals with `signal.NotifyContext`, and calling `e.Shutdown(ctx)` with a 10-second deadline.
12. Deploy behind a reverse proxy by setting `e.IPExtractor = echo.ExtractIPFromXFFHeader()` and enabling `middleware.Secure()` for HSTS, XSS protection, and content-type nosniff headers.
