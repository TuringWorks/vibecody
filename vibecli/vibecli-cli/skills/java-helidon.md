---
triggers: ["Helidon", "helidon SE", "helidon MP", "helidon webserver", "helidon nima"]
tools_allowed: ["read_file", "write_file", "bash"]
requires_bins: ["java"]
category: java
---

# Helidon Framework

When working with Helidon (SE and MP):

1. Choose Helidon SE for functional, non-CDI microservices with minimal overhead; choose Helidon MP for MicroProfile-compliant services that need JAX-RS, CDI, and MicroProfile Config/Health/Metrics out of the box.
2. In Helidon 4 (Nima), leverage virtual threads for the SE web server — handlers run on virtual threads by default, allowing blocking-style code without reactive complexity while maintaining high concurrency.
3. Define SE routes using the functional `HttpRouting.builder()` API; chain `.get()`, `.post()`, `.put()` with path patterns and handler lambdas, keeping route definitions in a single routing setup method for clarity.
4. Use `application.yaml` for configuration and access values via `Config.create().get("key").asString()`; in MP mode, use `@ConfigProperty` injection and `META-INF/microprofile-config.properties`.
5. For Helidon SE JSON handling, register `JsonpSupport` or `JacksonSupport` on the web server; in MP mode, JSON-B is the default — add `helidon-media-jackson` only if you need Jackson-specific features.
6. Implement health checks in SE with `HealthSupport.builder().addLiveness(...)` and register on the routing; in MP, use `@Liveness` and `@Readiness` annotations on CDI beans implementing `HealthCheck`.
7. Write tests for SE using `WebClient` pointed at `http://localhost` with a test-configured server; for MP, use `@HelidonTest` which starts the CDI container and server, then inject `WebTarget` for assertions.
8. Build GraalVM native images with `mvn package -Pnative-image`; Helidon SE has better native-image support since it avoids CDI proxies — ensure all reflection is declared in `reflect-config.json` for custom classes.
9. Use Helidon's built-in `DbClient` for reactive database access in SE mode; configure it in `application.yaml` under `db` with connection pool settings and use the fluent query API with named parameters.
10. Enable metrics with `MetricsSupport` in SE or the `@Counted`, `@Timed` annotations in MP; Helidon exposes Prometheus-format metrics at `/metrics` by default for easy scraping.
11. Secure endpoints with Helidon Security module — configure `SecurityHttpFeature` in SE with providers for HTTP Basic, JWT, or OIDC; in MP, use `@Authenticated` and `@RolesAllowed` on JAX-RS resources.
12. For production deployments, build a minimal JLink runtime image with `helidon-maven-plugin`'s `jlink-image` goal; this produces a self-contained directory with a custom JRE, ideal for distroless Docker containers.
