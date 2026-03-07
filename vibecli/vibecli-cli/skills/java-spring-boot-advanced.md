---
triggers: ["Spring WebFlux", "R2DBC", "GraalVM native image", "spring modulith", "spring virtual threads", "reactive spring", "spring native"]
tools_allowed: ["read_file", "write_file", "bash"]
requires_bins: ["java"]
category: java
---

# Advanced Spring Boot

When working with advanced Spring Boot features (WebFlux, R2DBC, GraalVM, Modulith, Virtual Threads):

1. Use `Mono` and `Flux` return types in WebFlux controllers; never block inside a reactive chain — offload blocking calls with `Schedulers.boundedElastic()` or move them to a dedicated thread pool.
2. Configure R2DBC connection pooling via `spring.r2dbc.pool.max-size` and `spring.r2dbc.pool.initial-size`; prefer `DatabaseClient` for complex queries and `R2dbcEntityTemplate` for simple CRUD over raw `ConnectionFactory`.
3. For GraalVM native images, register reflection hints in `RuntimeHintsRegistrar` or use `@RegisterReflectionForBinding`; run `./mvnw -Pnative native:compile` early and often since native compilation catches issues late builds miss.
4. Apply Spring Modulith by organizing code into top-level packages per bounded context; use `@ApplicationModuleTest` to verify module boundaries and `Documenter` to generate module diagrams.
5. Enable virtual threads with `spring.threads.virtual.enabled=true` on Java 21+; this benefits traditional blocking MVC apps more than WebFlux — avoid mixing both paradigms in the same service.
6. Write integration tests with Testcontainers using `@ServiceConnection` (Spring Boot 3.1+) to auto-configure datasource, Redis, or Kafka containers without manual property wiring.
7. Use `WebTestClient` for WebFlux endpoint testing and `StepVerifier` for unit-testing reactive streams; assert intermediate signals, not just the final result.
8. Configure `spring-boot-docker-compose` module to auto-start `docker-compose.yml` services during local development, eliminating manual container management.
9. Leverage `@Observability` and Micrometer Observation API for unified tracing across reactive and imperative code; export to OTLP with `management.otlp.tracing.endpoint`.
10. In native images, replace runtime proxies with compile-time generation — use `@HttpExchange` interfaces instead of Feign clients, and avoid CGLIB-heavy patterns like `@Configuration(proxyBeanMethods = true)`.
11. For R2DBC migrations, use Flyway with `spring.flyway.url` pointing to the JDBC URL of the same database since Flyway does not support R2DBC natively; run migrations at startup before the reactive pool initializes.
12. Profile reactive applications with BlockHound in test scope to detect accidental blocking calls; add `BlockHound.install()` in a test initializer and let it fail the build on violations.
