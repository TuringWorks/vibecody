---
triggers: ["jooby", "jooby mvc", "jooby netty", "jooby-apt"]
tools_allowed: ["read_file", "write_file", "bash"]
requires_bins: ["java"]
category: java
---

# Jooby Framework

When working with Jooby:

1. Choose between script and MVC styles — use the script API (`app.get("/", ctx -> "hello")`) for simple services and microservices; use MVC mode with `@Path` and `@GET`/`@POST` annotations for larger applications with controller organization.
2. Enable the `jooby-apt` annotation processor in your build for MVC mode — it generates route metadata at compile time, avoiding runtime reflection and improving startup performance.
3. Select the right server module: `jooby-netty` for high-throughput reactive workloads, `jooby-jetty` for servlet compatibility, or `jooby-undertow` for a balanced option; swap servers by changing one dependency without code changes.
4. Use `ctx.body(MyClass.class)` for request body parsing and install the appropriate renderer (`JacksonModule` for JSON, `GsonModule` as alternative); configure with `install(new JacksonModule())` in the application setup.
5. Manage database access with `jooby-hikari` for connection pooling and pair with `jooby-jdbc`, `jooby-hibernate`, or `jooby-flyway`; configure pools in `application.conf` under `hikari.db` with HOCON syntax.
6. Leverage Jooby's built-in dependency injection — use `require(MyService.class)` to retrieve services or install Guice with `jooby-guice` for complex dependency graphs; bind services in `install()` blocks.
7. Handle errors globally with `app.error((ctx, cause, code) -> ...)` for centralized error mapping; use `StatusCode` constants and throw `StatusCodeException` from handlers for automatic HTTP status code responses.
8. Configure applications using HOCON (`application.conf`) with environment overrides like `application.prod.conf`; access config values with `app.getConfig().getString("key")` or inject `Environment` for profile-aware loading.
9. Use `app.before()` and `app.after()` for cross-cutting concerns like logging, auth, and CORS; apply route-specific filters with `.before(handler)` chaining on individual routes.
10. Write tests using `MockRouter` for unit testing routes without starting a server: `new MockRouter(app).get("/path")` returns the response body; use `JoobyTest` extension for full integration tests with a running server.
11. Serve static assets with `AssetModule` or `AssetHandler` for file serving; configure cache headers and ETags in `application.conf` under `assets` for production-grade static file delivery.
12. Deploy as a fat JAR using `maven-shade-plugin` or `gradle shadowJar`; Jooby's fast startup (~100ms on JVM) makes it suitable for containerized environments — keep the Docker image minimal with `eclipse-temurin:21-jre-alpine` as the base.
