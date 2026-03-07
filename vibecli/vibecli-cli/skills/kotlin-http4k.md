---
triggers: ["http4k", "http4k lens", "http4k filter", "http4k contract", "http4k testing"]
tools_allowed: ["read_file", "write_file", "bash"]
requires_bins: ["java"]
category: kotlin
---

# Kotlin http4k Framework

When working with http4k:

1. Define handlers as `HttpHandler` functions `(Request) -> Response`; compose applications using `routes("/api/users" bind GET to ::listUsers, "/api/users/{id}" bind GET to ::getUser)` for a purely functional approach.
2. Use lenses for type-safe request/response extraction: `val idLens = Path.int().of("id")` and `val bodyLens = Body.auto<UserDto>().toLens()`; lenses validate and extract in one step, returning 400 on failure.
3. Chain filters for cross-cutting concerns: `ServerFilters.CatchAll().then(ServerFilters.Cors(corsPolicy)).then(DebuggingFilters.PrintRequestAndResponse()).then(app)` applies middleware in order.
4. Use the contract module for OpenAPI: define `contract { routes += "/users" meta { summary = "List users" } bindContract GET to ::listUsers }` to auto-generate Swagger specs from code.
5. Test handlers directly as functions: `val response = app(Request(GET, "/api/users"))` requires no server, no ports, no mocking frameworks; assert on `response.status` and `response.bodyString()`.
6. Use `ApprovalTest` for snapshot testing of API responses: record golden-file responses and verify regressions automatically; combine with contract tests for complete API validation.
7. Configure the server backend separately from the app: `app.asServer(Undertow(8080)).start()` swaps engines (Netty, Jetty, Undertow, SunHttp) without changing application code.
8. Use `RequestContexts` for per-request state: create a `RequestContexts()` lens, install `ServerFilters.InitialiseRequestContext(contexts)`, and store auth principals or trace IDs per request.
9. Leverage built-in OAuth and JWT support: `OAuthProvider` handles the full redirect flow; `JwtVerifier` validates tokens using `JwkAuth` with JWKS endpoint discovery for zero-config JWT validation.
10. Use http4k-connect for type-safe cloud SDK clients: AWS S3, SQS, DynamoDB, and others have http4k adapters that follow the same `HttpHandler` pattern and are testable with fakes.
11. Implement resilience with `RetryFilter` and `CircuitBreaker` filters on outbound clients: wrap `HttpHandler` clients with `ClientFilters.SetBaseUriFrom(Uri.of("http://backend")).then(RetryFilter(retryPolicy)).then(JavaHttpClient())`.
12. Deploy as a single fat JAR with no reflection or classpath scanning; http4k starts in milliseconds, making it ideal for serverless (AWS Lambda via `ApiGatewayV2LambdaFunction`) and container deployments.
