---
triggers: ["axum", "axum router", "axum tower", "axum extractors", "axum state"]
tools_allowed: ["read_file", "write_file", "bash"]
requires_bins: ["cargo"]
category: rust
---

# Axum Framework

When working with Axum:

1. Use `State(state): State<AppState>` extractor for shared application state — ensure `AppState` is `Clone` and wrap interior mutability in `Arc<Mutex<T>>` or `Arc<RwLock<T>>`.
2. Order extractors carefully: `body-consuming` extractors like `Json<T>` must come last in handler parameters since the body can only be read once.
3. Compose routers with `Router::merge()` for sibling route groups and `Router::nest("/prefix", sub_router)` for hierarchical nesting.
4. Leverage Tower middleware ecosystem directly — use `ServiceBuilder` with `.layer()` to stack timeout, rate-limiting, compression, and tracing layers.
5. Implement `IntoResponse` on custom types to standardize response formatting; return `(StatusCode, Json<ErrorBody>)` tuples for error responses.
6. Use `FromRequestParts` for custom extractors that read headers, query params, or extensions without consuming the body (e.g., auth user extraction).
7. Add `TraceLayer` from `tower-http` as the outermost layer to get structured request/response logging with span context for every endpoint.
8. Use `axum::extract::Path<(String, u64)>` with tuple destructuring for multi-segment path parameters; prefer `Path<IdParams>` with a named struct for clarity.
9. Handle graceful shutdown by passing a `tokio::signal::ctrl_c()` future to `axum::serve(...).with_graceful_shutdown(signal)`.
10. Prefer returning `Result<Json<T>, AppError>` from handlers where `AppError` implements `IntoResponse`, keeping handler bodies clean with `?` propagation.
11. Use `axum::middleware::from_fn` to write async middleware as plain functions rather than implementing the Tower `Service` trait manually.
12. Test handlers directly by calling them as async functions with constructed extractors, or use `TestClient` from `axum-test` for full integration tests.
