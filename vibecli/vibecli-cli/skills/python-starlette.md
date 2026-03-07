---
triggers: ["Starlette", "starlette ASGI", "starlette middleware", "starlette websocket"]
tools_allowed: ["read_file", "write_file", "bash"]
requires_bins: ["python3"]
category: python
---

# Starlette and ASGI

When working with Starlette:

1. Define routes using `Route`, `Mount`, and `WebSocketRoute` in a declarative list passed to `Starlette(routes=[...])` rather than decorator-based registration for explicit, testable route configuration.
2. Use dependency injection through `request.state` set in middleware or by passing services to endpoint classes via `__init__`; avoid global mutable state to keep endpoints testable in isolation.
3. Implement middleware as pure ASGI callables or use `BaseHTTPMiddleware` for simple cases; for performance-critical middleware (logging, metrics), write raw ASGI middleware that avoids the `BaseHTTPMiddleware` body consumption overhead.
4. Handle WebSocket connections with `async with websocket` context manager pattern; always wrap in try/except for `WebSocketDisconnect` and clean up resources in a finally block.
5. Use `Starlette(on_startup=[...], on_shutdown=[...])` lifecycle hooks to initialize and close database pools, HTTP clients, and cache connections; prefer `lifespan` context manager in newer versions for cleaner resource management.
6. Return responses with `JSONResponse`, `HTMLResponse`, or `StreamingResponse` explicitly; use `StreamingResponse` with an async generator for SSE or large file downloads to avoid buffering entire payloads in memory.
7. Write tests using `httpx.AsyncClient` with `ASGITransport(app=app)` for async test coverage; this avoids starting a real server and enables full integration testing including middleware and auth.
8. Use `BackgroundTask` for fire-and-forget operations (sending emails, logging analytics) attached to a response; for longer jobs, delegate to a task queue like Celery or arq.
9. Validate request bodies by integrating Pydantic models manually: parse `await request.json()` into a Pydantic model and catch `ValidationError` to return structured 422 responses.
10. Serve static files with `StaticFiles(directory="static")` mounted on a path and configure cache headers via middleware; in production, serve statics from a CDN or nginx instead.
11. Deploy with `uvicorn app:app --workers 4 --host 0.0.0.0` behind nginx or Caddy; set `--limit-concurrency` and `--timeout-keep-alive` to match your infrastructure capacity.
12. Add OpenTelemetry instrumentation with `opentelemetry-instrumentation-asgi` to trace requests end-to-end; export spans to Jaeger or Tempo for production observability without modifying application code.
