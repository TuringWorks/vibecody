---
triggers: ["Ktor", "ktor", "ktor routing", "ktor plugin", "ktor client", "ktor server"]
tools_allowed: ["read_file", "write_file", "bash"]
requires_bins: ["java"]
category: kotlin
---

# Kotlin Ktor Framework

When working with Ktor:

1. Structure the application with `embeddedServer(Netty, port = 8080) { module() }` or `EngineMain` with `application.conf`; extract feature installation into extension functions on `Application` for modularity.
2. Organize routing with `routing { }` blocks and split into separate files using extension functions: `fun Route.userRoutes()` keeps route definitions manageable as the API grows.
3. Use the plugin (formerly feature) system for cross-cutting concerns: install `ContentNegotiation` with `json()`, `CallLogging`, `StatusPages`, and `CORS` in the application module.
4. Handle errors centrally with `StatusPages`: register `exception<NotFoundException> { call, _ -> call.respond(HttpStatusCode.NotFound, ErrorResponse("Not found")) }` for consistent error responses.
5. Use `call.receive<T>()` with kotlinx.serialization data classes for request parsing and `call.respond(HttpStatusCode.OK, response)` for type-safe responses; annotate DTOs with `@Serializable`.
6. Implement authentication with the `Authentication` plugin: configure `jwt { }` or `bearer { }` blocks and protect routes with `authenticate("auth-jwt") { route("/api") { } }`.
7. Use Ktor's `HttpClient` for outbound calls with the same serialization setup: `HttpClient(CIO) { install(ContentNegotiation) { json() } }`; share a single client instance and close it on shutdown.
8. Leverage coroutines throughout: Ktor handlers are suspend functions by default; use `withContext(Dispatchers.IO)` for blocking I/O and `async/await` for concurrent upstream calls within a handler.
9. Configure the server via `application.conf` (HOCON) or `application.yaml` for environment-specific settings; access values with `environment.config.property("ktor.deployment.port").getString()`.
10. Write tests with `testApplication { }` DSL: configure the application module, then use `client.get("/api/users")` and assert on `response.status` and `response.body<T>()` without starting a real server.
11. Use route-scoped plugins for middleware-like behavior: `route("/admin") { install(RateLimit) { }; get { } }` applies configuration only to matching routes instead of globally.
12. Deploy as a fat JAR with the `ktor-server-netty` engine and the Shadow Gradle plugin; configure `application.conf` to read `PORT` from environment variables for container deployments with `ktor.deployment.port = ${PORT}`.
