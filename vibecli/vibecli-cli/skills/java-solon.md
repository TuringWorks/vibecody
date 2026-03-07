---
triggers: ["Solon", "solon java", "solon framework", "solon cloud"]
tools_allowed: ["read_file", "write_file", "bash"]
requires_bins: ["java"]
category: java
---

# Solon Framework

When working with the Solon framework:

1. Bootstrap with `Solon.start(App.class, args)` in the main method; Solon starts in under 100ms with minimal classpath scanning, making it ideal for microservices and serverless functions.
2. Define controllers with `@Controller` and `@Mapping("/api/users")`: use `@Get`, `@Post`, `@Put`, `@Delete` method annotations for clean RESTful routing without heavy framework overhead.
3. Use constructor or field injection with `@Inject`: Solon's lightweight IoC container supports singleton and prototype scopes; define beans with `@Component` or programmatically via `context.beanMake()`.
4. Handle request/response with `Context ctx`: access parameters with `ctx.param("id")`, body with `ctx.bodyAsBean(UserDto.class)`, and respond with `ctx.render(result)` for automatic serialization.
5. Configure with `app.yml` or `app.properties`: use profiles via `app-dev.yml` and `app-prod.yml`; access config values with `@Inject("${db.url}") String dbUrl` for environment-specific settings.
6. Add middleware with `@Component` implementing `Filter`: override `doFilter(Context ctx, FilterChain chain)` for request/response interception; control order with `@Mapping(order = -1)`.
7. Use Solon's plugin system for modular features: add `solon-data` for database, `solon-validation` for input validation, `solon-openapi2-knife4j` for Swagger docs; plugins auto-configure on classpath detection.
8. Integrate databases with `solon-data` and preferred ORM: use MyBatis with `@Db` annotation or Wood/Sqltoy for lighter alternatives; configure connection pools in `app.yml` under `datasource`.
9. Implement validation with `@Valid` on controller classes and `@NotNull`, `@Length`, `@Pattern` on DTO fields; Solon validates automatically and returns structured error responses.
10. Use `@Gateway` for API gateway patterns: define routing rules, rate limiting, and authentication at the gateway level; combine with `solon-cloud` for service discovery and config center integration.
11. Support WebSocket and SSE natively: annotate with `@ServerEndpoint("/ws")` and implement `onOpen`, `onMessage`, `onClose` listeners; use `@Produces("text/event-stream")` for server-sent events.
12. Build minimal Docker images: Solon's small footprint (core JAR ~0.5MB) and fast startup suit GraalVM native-image compilation; use `solon-native` plugin and `native-image` to produce sub-10MB static binaries with <50ms startup.
