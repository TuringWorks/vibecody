---
triggers: ["Litestar", "litestar framework", "starlite", "litestar dto", "litestar guards"]
tools_allowed: ["read_file", "write_file", "bash"]
requires_bins: ["python3"]
category: python
---

# Litestar Framework

When working with Litestar:

1. Define route handlers as typed functions with `@get("/items")`, `@post("/items")`, etc., using full type annotations on parameters and return types; Litestar uses these types to auto-generate OpenAPI docs and perform validation.
2. Use DTOs (Data Transfer Objects) with `DTOConfig(exclude={"password"}, rename_fields={"id": "item_id"})` on `DataclassDTO` or `SQLAlchemyDTO` to control serialization without leaking internal model details to API consumers.
3. Implement dependency injection with `Provide(get_db_session)` in the `dependencies` parameter of the app or router; Litestar resolves the dependency graph automatically and supports sync, async, generator, and class-based providers.
4. Use Guards for authorization by defining `async def auth_guard(connection, handler) -> None` that raises `NotAuthorizedException`; apply guards at the app, router, or handler level for layered security.
5. Leverage Litestar's Middleware system with `DefineMiddleware(CORSMiddleware, config=cors_config)` for built-in middleware or create custom ASGI middleware; use `AbstractMiddleware` for request/response-level processing with access to the resolved handler.
6. Use the `Litestar(lifespan=[db_lifespan])` context manager pattern for resource lifecycle management; yield the resource in the async generator and Litestar handles startup/shutdown cleanly.
7. Configure SQLAlchemy integration with `SQLAlchemyPlugin(config=SQLAlchemyAsyncConfig(connection_string=...))` for automatic session management, repository pattern support, and pagination helpers.
8. Write tests using `create_test_client(route_handlers=[my_handler])` for unit tests on individual handlers, or `TestClient(app=full_app)` for integration tests; both provide synchronous `.get()`, `.post()` methods.
9. Use `@controller` decorator to group related handlers into a class with shared path prefix, dependencies, and guards; keep controller classes focused on a single resource for maintainability.
10. Implement pagination with `AbstractSyncOffsetPaginator` or `AbstractAsyncCursorPaginator` for consistent, typed paginated responses; integrate with SQLAlchemy repositories for automatic query pagination.
11. Use Litestar's built-in OpenAPI support to customize schema generation: set `OpenAPIConfig(title="My API", version="1.0")` and access docs at `/schema/swagger`, `/schema/redoc`, or `/schema/elements` with zero additional configuration.
12. Deploy with `uvicorn` or `granian` for best performance: `granian --interface asgi app:app --workers 4 --threads 2`; use Litestar's response caching with `@get(cache=120)` for expensive endpoints and configure a Redis cache store for multi-worker deployments.
