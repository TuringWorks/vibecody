---
triggers: ["Sanic", "sanic async", "sanic blueprint", "sanic middleware"]
tools_allowed: ["read_file", "write_file", "bash"]
requires_bins: ["python3"]
category: python
---

# Sanic Async Framework

When working with Sanic:

1. Define the app with `Sanic("MyApp")` and use `app.config.update_config("config.py")` or environment variables with `SANIC_` prefix for configuration; access via `app.config.DB_URL` throughout the application.
2. Organize routes into Blueprints with `Blueprint("api", url_prefix="/api", version="v1")` and group them with `Blueprint.group(bp1, bp2, url_prefix="/service")` for clean API versioning.
3. Use `@app.middleware("request")` and `@app.middleware("response")` for cross-cutting concerns; middleware executes in registration order on request and reverse order on response, so register auth before business logic middleware.
4. Leverage Sanic's built-in async: use `httpx.AsyncClient` for upstream calls, `asyncpg` for PostgreSQL, and `aioredis` for Redis; never use blocking libraries like `requests` or `psycopg2` which stall the event loop.
5. Initialize shared resources in `@app.before_server_start` listeners (connection pools, HTTP clients) and close them in `@app.after_server_stop`; store on `app.ctx` for access across handlers.
6. Stream responses with `response.stream()` and an async generator for large payloads or SSE; use `await response.send(chunk)` inside the streaming callback for incremental delivery.
7. Handle WebSocket connections with `@app.websocket("/ws")` and use `await ws.recv()` / `await ws.send()` in a loop; wrap in try/except for `ConnectionClosed` and use `asyncio.wait_for` with timeouts.
8. Write tests using `app.asgi_client` (powered by httpx) for async integration tests: `_, response = await app.asgi_client.get("/endpoint")` runs the full middleware stack without starting a server.
9. Use Sanic's signal system with `@app.signal("http.lifecycle.request")` for event-driven patterns; define custom signals with `app.dispatch("my.custom.event", context={...})` for decoupled communication.
10. Enable auto-reload in development with `app.run(auto_reload=True)` and use `--debug` flag; in production, run with `sanic app:create_app --host 0.0.0.0 --port 8000 --workers 4 --fast` for optimal multiprocess performance.
11. Implement structured error handling with `@app.exception(SanicException)` and custom exception classes that carry status codes and JSON-serializable context for consistent API error responses.
12. Deploy behind nginx with proxy_pass and set `app.config.PROXIES_COUNT` to correctly resolve client IPs; use `--access-log` in dev only and switch to structured JSON logging with `sanic-ext` or custom log formatters in production.
