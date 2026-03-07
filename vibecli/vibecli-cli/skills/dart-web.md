---
triggers: ["Dart backend", "dart server", "shelf dart", "dart_frog", "angel3", "dart http server"]
tools_allowed: ["read_file", "write_file", "bash"]
requires_bins: ["dart"]
category: dart
---

# Dart Backend (Shelf, Dart Frog, Angel)

When working with Dart backend frameworks:

1. Use `shelf` as the foundation — build handlers as `FutureOr<Response> Function(Request)` and compose middleware with `Pipeline()..addMiddleware(logRequests())..addHandler(router)`.
2. In Dart Frog, organize routes using the file-system convention: `routes/api/users/index.dart` maps to `/api/users`, and `routes/api/users/[id].dart` handles dynamic segments automatically.
3. Define middleware in Dart Frog by creating a `_middleware.dart` file in the route directory — it applies to all routes in that directory and its children, keeping cross-cutting concerns colocated.
4. Use `shelf_router` annotations (`@Route.get('/path')`) for declarative routing and run `dart run build_runner build` to generate the router glue code.
5. Implement dependency injection in Dart Frog via `provider()` middleware — register services in `_middleware.dart` and read them in handlers with `context.read<MyService>()`.
6. Serialize request and response bodies with `json_serializable` or `freezed`; annotate model classes with `@JsonSerializable()` and use `fromJson`/`toJson` factories for type-safe parsing.
7. Handle errors with a top-level shelf middleware that catches exceptions and returns structured JSON error responses with appropriate HTTP status codes — never leak stack traces in production.
8. For Angel3, register controllers with `app.container.registerSingleton<MyController>(MyController())` and use `@Expose('/path')` annotations for route mapping.
9. Compile to a native AOT binary with `dart compile exe bin/server.dart -o server` for production — this yields fast startup, low memory, and a single deployable artifact.
10. Use `dart:io`'s `HttpServer.bind` with `shared: true` to enable multi-isolate serving on the same port for parallel request processing across CPU cores.
11. Write handler tests with `package:test` and `shelf`'s in-memory request: create `Request('GET', Uri.parse('/path'))`, pass it to the handler, and assert on `Response` status and body.
12. Configure hot reload during development with `dart run --enable-vm-service` and use `dart_frog dev` for automatic restart on file changes to maintain a fast feedback loop.
