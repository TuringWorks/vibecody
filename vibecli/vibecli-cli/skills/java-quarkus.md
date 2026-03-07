---
triggers: ["Quarkus", "quarkus-reactive", "quarkus-native", "@QuarkusTest", "quarkus extension", "quarkus dev services"]
tools_allowed: ["read_file", "write_file", "bash"]
requires_bins: ["java"]
category: java
---

# Quarkus Framework

When working with Quarkus:

1. Use `quarkus dev` for live-reload during development; press `r` in the terminal to re-run tests, `w` to open Dev UI, and `d` to toggle continuous testing automatically on save.
2. Prefer CDI `@ApplicationScoped` beans with constructor injection; Quarkus uses ArC (build-time CDI) so avoid runtime reflection-based DI patterns and `@Dependent` scope for stateless services.
3. Leverage Dev Services for zero-config local databases, Kafka, and Redis — Quarkus auto-starts Testcontainers when no connection URL is configured, sharing containers across test classes with `quarkus.datasource.devservices.shared=true`.
4. For reactive endpoints, use RESTEasy Reactive with `@Path` annotations and return `Uni<T>` or `Multi<T>` from Mutiny; this is the default REST layer in Quarkus 3+ and outperforms the classic RESTEasy stack.
5. Build native executables with `quarkus build --native` or `./mvnw package -Dnative`; test native images with `@QuarkusIntegrationTest` which runs against the actual built artifact rather than the JVM version.
6. Configure Hibernate ORM with Panache for active-record or repository patterns; use `PanacheEntity` for simple models and `PanacheEntityBase` when you need custom ID strategies.
7. Use `@QuarkusTest` for full integration tests that boot the application; use `@QuarkusTestResource` for custom test lifecycle management like spinning up external services.
8. Organize configuration in `application.properties` with profile-specific overrides using `%dev.`, `%test.`, and `%prod.` prefixes; use `@ConfigMapping` interfaces instead of `@ConfigProperty` for grouped configuration.
9. Register custom health checks with `@Liveness` and `@Readiness` from MicroProfile Health; Quarkus auto-exposes `/q/health/live` and `/q/health/ready` endpoints for Kubernetes probes.
10. Use Quarkus extensions over raw library dependencies — extensions are optimized for build-time processing and native compilation; search available extensions with `quarkus extension list`.
11. For GraalVM native issues, register reflection with `@RegisterForReflection` on DTOs and use `quarkus.native.additional-build-args` for advanced GraalVM flags; check the native build report at `target/*-reports/` for missing registrations.
12. Deploy with the container-image extensions (`quarkus-container-image-jib` or `quarkus-container-image-docker`); build and push in one step with `quarkus build -Dquarkus.container-image.push=true` and configure registry coordinates in `application.properties`.
