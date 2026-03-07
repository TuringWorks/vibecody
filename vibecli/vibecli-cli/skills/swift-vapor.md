---
triggers: ["Vapor", "vapor swift", "fluent", "vapor routing", "swift server side", "swift-nio", "hummingbird swift"]
tools_allowed: ["read_file", "write_file", "bash"]
requires_bins: ["swift"]
category: swift
---

# Swift Vapor Framework

When working with Swift Vapor:

1. Structure the project with `configure.swift` for setup, `routes.swift` for route registration, and separate controller files; use `app.routes.grouped("api", "v1")` for versioned route groups.
2. Define models as `final class` conforming to `Model` and `Content`: specify `@ID(key: .id) var id: UUID?` and `@Field(key: "name") var name: String` for Fluent ORM mapping.
3. Create migrations for schema changes: implement `AsyncMigration` with `prepare(on:)` and `revert(on:)` methods; register with `app.migrations.add(CreateUser())` and run with `app.autoMigrate().wait()` or CLI.
4. Use `Content` protocol for request/response coding: define `struct CreateUserRequest: Content { let name: String; let email: String }` and decode with `let dto = try req.content.decode(CreateUserRequest.self)`.
5. Implement authentication with `ModelAuthenticatable` for basic auth, `ModelTokenAuthenticatable` for bearer tokens, and JWT with `app.jwt.signers.use(.hs256(key:))` for stateless auth.
6. Use `req.db` for database access within route handlers; leverage Fluent's query builder: `User.query(on: req.db).filter(\.$email == email).with(\.$posts).first()` for eager loading and filtering.
7. Handle errors with `Abort(.notFound, reason: "User not found")` for HTTP errors; implement `AbortError` on custom error types for structured error responses with correct status codes.
8. Use middleware for cross-cutting concerns: create struct conforming to `AsyncMiddleware` with `respond(to:chainingTo:)` method; register globally with `app.middleware.use(MyMiddleware())` or per-group.
9. Leverage async/await throughout: Vapor 4.x supports async route handlers natively; use `async` closures in `app.get("users") { req async throws -> [UserDTO] in }` for clean concurrent code.
10. Configure database connections in `configure.swift`: use `app.databases.use(.postgres(configuration:), as: .psql)` with connection pooling; read credentials from environment variables, not hardcoded values.
11. Write tests using `XCTVapor`: create `Application` in setUp, call `app.test(.GET, "/api/users") { res in XCTAssertEqual(res.status, .ok) }` for integration tests without a running server.
12. Deploy with Docker using the official Swift slim image; configure `app.http.server.configuration.hostname = "0.0.0.0"` for container networking, set `app.environment` to `.production`, and enable `app.logger.logLevel = .notice` for production log levels.
