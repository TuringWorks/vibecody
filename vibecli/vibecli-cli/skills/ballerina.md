---
triggers: ["Ballerina", "ballerina lang", "ballerina service", "ballerina connector", "bal build", "ballerina integration"]
tools_allowed: ["read_file", "write_file", "bash"]
requires_bins: ["bal"]
category: ballerina
---

# Ballerina Language

When working with Ballerina:

1. Define HTTP services using the `service` keyword on `http:Listener` and annotate resource methods with HTTP verbs (`get`, `post`, `put`, `delete`) for clean REST API design.
2. Use Ballerina records as typed data contracts for request/response payloads; the compiler enforces structural typing so missing or extra fields are caught at build time.
3. Leverage built-in connectors (`http:Client`, `kafka:Producer`, `rabbitmq:Client`, `mysql:Client`) instead of raw socket code; configure connection pools and timeouts in the client init.
4. Handle errors explicitly with the `check` keyword and Ballerina's union error types (`T|error`); avoid silent failures by propagating errors to the caller or logging with `log:printError`.
5. Use `data mappings` and `transform` expressions for ETL-style field mapping between records; prefer query expressions (`from ... select`) over manual iteration for filtering and projecting collections.
6. Define gRPC services from `.proto` files using `bal grpc --input` to generate stubs, then implement the generated service interface with concrete logic.
7. Expose GraphQL APIs by returning record types from `graphql:Service` resource methods; Ballerina auto-generates the schema from the return types.
8. Use `start` and `wait` for async message-based patterns; combine with `worker` declarations to run parallel computations within a single function.
9. Enable built-in observability by adding `observe` to `Ballerina.toml`; traces, metrics, and logs are emitted automatically for all network calls without code changes.
10. Structure multi-module projects with `Ballerina.toml` at the root and one directory per module; use `import <org>/<module>` for cross-module dependencies and `bal pack` for library distribution.
11. Write integration tests in `tests/` directories using `@test:Config` annotations; use `@test:Mock` to stub out external connector calls and assert with `test:assertEquals`.
12. Run `bal build` to produce a thin `.jar` with an embedded runtime; use `bal build --cloud=docker` to generate a Dockerfile and container image in one step for cloud-native deployment.
