---
triggers: ["Micronaut", "micronaut-data", "@Controller micronaut", "micronaut GraalVM", "micronaut test"]
tools_allowed: ["read_file", "write_file", "bash"]
requires_bins: ["java"]
category: java
---

# Micronaut Framework

When working with Micronaut:

1. Use `@Controller` with `@Get`, `@Post`, etc. for HTTP endpoints; Micronaut computes routes at compile time via annotation processing so ensure the `micronaut-inject-java` annotation processor is in your build.
2. Prefer constructor injection over field injection — Micronaut performs dependency injection at compile time, not runtime, so all beans must be resolvable during compilation; use `@Requires` to conditionally load beans.
3. Leverage Micronaut Data with `@Repository` interfaces extending `CrudRepository` or `PageableRepository`; query methods are generated at compile time, eliminating runtime proxy overhead typical of Spring Data.
4. Write tests with `@MicronautTest` which boots an embedded server; inject `HttpClient` via `@Client("/")` and use `client.toBlocking()` for synchronous test assertions against live endpoints.
5. Build GraalVM native images with `./gradlew nativeCompile` or `./mvnw package -Dpackaging=native-image`; Micronaut's compile-time DI means fewer reflection issues than other frameworks, but still annotate DTOs with `@Introspected` for serialization.
6. Use `@Introspected` on all POJOs used in JSON serialization — this generates BeanIntrospection metadata at compile time, replacing Jackson's runtime reflection with direct property access.
7. Configure application properties in `application.yml` with environment-specific files like `application-dev.yml`; use `@ConfigurationProperties` for type-safe config binding and `@EachProperty` for dynamic, indexed configuration.
8. Implement reactive HTTP with return types of `Mono`, `Flux`, `Publisher`, or Micronaut's own `HttpResponse<Flux<T>>`; pair with Micronaut R2DBC or Micronaut Data Reactive for end-to-end non-blocking pipelines.
9. Use Micronaut's built-in HTTP client by defining `@Client` interfaces with declarative annotations — these are compiled to efficient clients without reflection, unlike Feign or RestTemplate.
10. Apply `@Cacheable`, `@CachePut`, and `@CacheInvalidate` with the micronaut-cache module; configure backing stores (Caffeine, Redis, Hazelcast) in `application.yml` under `micronaut.caches`.
11. For serverless deployments, use `micronaut-function-aws` or `micronaut-azure-function`; the fast cold-start from compile-time DI makes Micronaut well-suited for Lambda/Cloud Functions without GraalVM.
12. Enable distributed tracing with `micronaut-tracing-opentelemetry-http` and export to Jaeger or Zipkin; Micronaut propagates trace context across `@Client` calls automatically when the tracing module is on the classpath.
