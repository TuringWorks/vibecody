---
triggers: ["Dropwizard", "dropwizard metrics", "dropwizard jersey", "dropwizard-hibernate"]
tools_allowed: ["read_file", "write_file", "bash"]
requires_bins: ["java"]
category: java
---

# Dropwizard Framework

When working with Dropwizard:

1. Organize your application around the `Application<T extends Configuration>` class; implement `initialize()` for bundles and `run()` for resource registration — keep `run()` focused on wiring, not business logic.
2. Define configuration in a YAML file mapped to your `Configuration` subclass with Jackson annotations; use `@NotEmpty`, `@Min`, `@Max` from Hibernate Validator to enforce config constraints and fail fast at startup.
3. Register JAX-RS resources in the `run()` method with `environment.jersey().register(new MyResource())`; inject dependencies via constructor — Dropwizard does not include a DI container by default, so wire manually or add HK2/Guice.
4. Use Dropwizard's Metrics library (built-in) by annotating resource methods with `@Timed`, `@Metered`, or `@ExceptionMetered`; access the metric registry for custom gauges and counters via `environment.metrics()`.
5. Leverage health checks by extending `HealthCheck` and registering with `environment.healthChecks().register("db", new DbHealthCheck())`; Dropwizard exposes `/healthcheck` on the admin port by default.
6. Use `dropwizard-hibernate` bundle for JPA with connection pooling; define your `DataSourceFactory` in the config YAML and access `SessionFactory` via the `HibernateBundle` — call DAO methods inside `@UnitOfWork` annotated resource methods.
7. For database migrations, use `dropwizard-migrations` which wraps Liquibase; run `java -jar app.jar db migrate config.yml` to apply changesets and `db status` to check pending migrations before deployment.
8. Write integration tests with `DropwizardAppRule` (JUnit 4) or `DropwizardAppExtension` (JUnit 5) which starts the full application; use `rule.client()` to make HTTP requests against the running app in tests.
9. Implement request filtering with Jersey filters — use `ContainerRequestFilter` for auth/validation and `ContainerResponseFilter` for response headers; register them in `run()` alongside resources.
10. Configure the admin connector separately from the application connector in YAML; the admin port (default 8081) serves operational endpoints (`/metrics`, `/healthcheck`, `/threads`) — never expose it publicly in production.
11. Add `dropwizard-auth` for authentication; implement `Authenticator<C, P>` and optionally `Authorizer<P>` for role-based access, then register the `AuthDynamicFeature` with a `@Auth` annotation on resource method parameters.
12. Package as a fat JAR with the Maven Shade plugin; run with `java -jar app.jar server config.yml` — use the `check` command (`java -jar app.jar check config.yml`) in CI to validate configuration without starting the server.
