---
triggers: ["fasthttp", "go fasthttp", "gnet", "go high performance http"]
tools_allowed: ["read_file", "write_file", "bash"]
requires_bins: ["go"]
category: go
---

# Go fasthttp and High-Performance HTTP

When working with fasthttp:

1. Never store references to `ctx.Request` or `ctx.Response` beyond the handler's return; fasthttp reuses request contexts from a pool, so copy any needed data with `string(ctx.PostBody())` or `append(dst, ctx.Path()...)`.
2. Use `fasthttp.Server` configuration tuning: set `Concurrency` (default 256K), `ReadBufferSize`, `WriteBufferSize`, and `MaxRequestBodySize` based on your workload profile; benchmark with `wrk` or `bombardier` after each change.
3. Route requests with `fasthttp/router` (valyala/fasthttp) for path-parameter support; avoid string-matching in a raw `RequestHandler` as it quickly becomes unmaintainable and misses edge cases.
4. Reuse `fasthttp.Client` instances across goroutines for upstream calls; configure `MaxConnsPerHost`, `ReadTimeout`, and `WriteTimeout` to prevent connection exhaustion under load.
5. Use `ctx.Response.SetBodyStreamWriter()` for streaming large responses (SSE, file downloads) instead of buffering into `ctx.Response.SetBody()` which copies the entire payload into memory.
6. Implement middleware as handler wrappers: `func authMiddleware(next fasthttp.RequestHandler) fasthttp.RequestHandler` that return a new handler, enabling composable chains without framework overhead.
7. Use `fasthttp.AcquireArgs()` / `fasthttp.ReleaseArgs()` and similar acquire/release patterns for URI, headers, and cookies to minimize GC pressure in hot paths.
8. Prefer `ctx.Request.Header.PeekBytes()` over `Peek()` to avoid string allocations when comparing header values; use `bytes.Equal()` for comparisons in performance-critical middleware.
9. For JSON handling, use `encoding/json` for correctness or `jsoniter`/`sonic` for speed; call `ctx.SetContentType("application/json")` explicitly since fasthttp does not auto-detect content types.
10. Write benchmarks with `testing.B` and use `go test -bench=. -benchmem` to verify zero-allocation goals; profile with `pprof` to identify remaining allocations in handler hot paths.
11. Integrate with gnet or nbio for extreme connection-count scenarios (100K+ concurrent); use fasthttp as the HTTP parser on top of the custom event loop for maximum throughput.
12. Implement graceful shutdown with `server.Shutdown()` and drain connections; in Kubernetes, add a `/healthz` readiness endpoint that returns 503 once SIGTERM is received, giving the load balancer time to deregister.
