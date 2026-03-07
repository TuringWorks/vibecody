---
triggers: ["actix-web", "actix", "actix handler", "actix middleware", "actix extractors"]
tools_allowed: ["read_file", "write_file", "bash"]
requires_bins: ["cargo"]
category: rust
---

# Actix-web Framework

When working with Actix-web:

1. Use extractors (`web::Path`, `web::Query`, `web::Json`, `web::Data`) as handler parameters for type-safe request parsing instead of manually reading the request body.
2. Register shared state with `App::app_data(web::Data::new(state))` and extract it in handlers via `web::Data<T>` — never use global mutable statics.
3. Implement custom error types with `ResponseError` trait to map domain errors to proper HTTP status codes and JSON error bodies.
4. Compose middleware using `wrap()` on `App` or `Scope` — order matters, as middleware executes outside-in on request and inside-out on response.
5. Use `web::scope("/api")` to group related routes under a common prefix and apply scope-specific middleware like authentication guards.
6. Prefer `actix_web::test` module for integration tests — `TestRequest::get().to(handler).await` for unit-testing individual handlers without spinning up a server.
7. Configure connection pools (sqlx, diesel, deadpool) as `web::Data` and inject them into handlers; never create a new pool per request.
8. Use `#[actix_web::main]` on `main()` to bootstrap the Actix runtime; avoid mixing with a separate `tokio::main` to prevent runtime conflicts.
9. Stream large responses with `HttpResponse::Ok().streaming(byte_stream)` instead of buffering entire payloads in memory.
10. Apply `web::Json<T>` with `#[derive(Deserialize, Validate)]` and call `.validate()` early in handlers to reject malformed input before business logic runs.
11. Set `HttpServer::workers()` explicitly in production to match available CPU cores; use `keep_alive()` and `client_timeout()` to tune connection behavior.
12. Use `Guard` trait for custom route matching (e.g., header-based routing, feature flags) instead of duplicating conditional logic inside handlers.
