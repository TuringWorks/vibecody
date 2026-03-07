---
triggers: ["Gin", "gin-gonic", "gin router", "gin middleware", "gin handler"]
tools_allowed: ["read_file", "write_file", "bash"]
requires_bins: ["go"]
category: go
---

# Go Gin Framework

When working with Gin:

1. Set `gin.SetMode(gin.ReleaseMode)` in production and use `gin.SetMode(gin.TestMode)` in tests to suppress debug logging and get clean test output.
2. Use `router.Group("/api/v1")` with middleware attached at the group level; chain `Use()` calls for auth, rate-limiting, and logging so handlers stay focused on business logic.
3. Bind request data with `c.ShouldBindJSON(&req)` (not `c.BindJSON` which auto-aborts); check the returned error and respond with `c.JSON(400, gin.H{"error": err.Error()})` for full control over error responses.
4. Register custom validators via `binding.Validator.Engine().(*validator.Validate).RegisterValidation(...)` to add domain-specific validation tags usable directly in struct field annotations.
5. Use `c.Set("user", userObj)` in auth middleware and retrieve with `c.MustGet("user").(User)` in handlers; define typed getter functions to avoid key mismatches and assertion panics across the codebase.
6. Implement a centralized error handler by defining a custom error type, returning it from handlers via a wrapper function, and using `c.Error(err)` with a `gin.HandleErrorsMiddleware`-style function to format responses uniformly.
7. Use `c.Stream()` for server-sent events and large file downloads instead of buffering entire responses in memory; set `Content-Type` and flush headers before streaming.
8. Write tests with `httptest.NewRecorder()` and `gin.CreateTestContext(w)` or use `router.ServeHTTP(w, req)` for full integration tests that exercise middleware and routing together.
9. Serve static assets with `router.Static("/assets", "./public")` and enable `router.Use(gzip.Gzip(gzip.DefaultCompression))` via `gin-contrib/gzip` for automatic response compression.
10. Use `c.ShouldBindUri(&params)` for path parameter validation with struct tags, ensuring type safety on route params like `/users/:id` without manual `strconv` conversions.
11. Configure `trusted proxies` with `router.SetTrustedProxies([]string{"10.0.0.0/8"})` when behind a load balancer to correctly resolve client IPs from `X-Forwarded-For`.
12. Implement graceful shutdown by running `srv.ListenAndServe()` in a goroutine with `http.Server{Handler: router}`, then calling `srv.Shutdown(ctx)` on SIGTERM with a 15-second context deadline.
