---
triggers: ["Vert.x", "vertx", "vertx-web", "vertx eventbus", "vertx reactive"]
tools_allowed: ["read_file", "write_file", "bash"]
requires_bins: ["java"]
category: java
---

# Eclipse Vert.x

When working with Vert.x:

1. Never block the event loop — all handler code in Verticles must be non-blocking; offload CPU-intensive or blocking operations to worker Verticles with `vertx.deployVerticle(new MyWorker(), new DeploymentOptions().setWorker(true))`.
2. Use the `Router` from `vertx-web` for HTTP routing instead of raw `HttpServer` handlers; define routes with `.route().handler()`, attach `BodyHandler.create()` globally for request body parsing, and use `RoutingContext` for request/response access.
3. Communicate between Verticles using the EventBus with `vertx.eventBus().send()` for point-to-point and `.publish()` for broadcast; register codecs with `eventBus.registerDefaultCodec()` for custom message types.
4. Use `Future<T>` and `Promise<T>` for async composition; chain operations with `.compose()`, combine with `Future.all()` or `Future.join()`, and handle errors with `.recover()` instead of nested callbacks.
5. Configure Verticle deployment with `DeploymentOptions` — set `setInstances()` to the number of CPU cores for event-loop Verticles, and use `setConfig(new JsonObject())` to pass Verticle-specific configuration.
6. For database access, use `vertx-pg-client` or `vertx-mysql-client` (reactive SQL clients) with pooled connections; prefer `PreparedQuery` with tuple parameters for SQL injection safety and connection reuse.
7. Write tests with `vertx-junit5` extension — annotate tests with `@ExtendWith(VertxExtension.class)`, inject `Vertx` and `VertxTestContext`, and call `testContext.completeNow()` or `testContext.failNow()` to signal async test completion.
8. Use `vertx-config` to load configuration from multiple sources (file, env, Consul, Vault) with priority ordering; watch for config changes with `configRetriever.listen()` to hot-reload settings.
9. Enable clustering with `vertx-hazelcast` or `vertx-infinispan` for distributed EventBus; start the clustered Vertx instance with `Vertx.clusteredVertx(options)` and ensure multicast or TCP discovery is configured.
10. Serve static files and templates with `StaticHandler.create()` and template engines (`vertx-web-templ-handlebars`, `vertx-web-templ-thymeleaf`); mount the static handler on a subrouter to isolate API and asset routes.
11. Implement WebSocket handling by upgrading HTTP connections with `router.route("/ws").handler(rc -> rc.request().toWebSocket().onSuccess(ws -> ...))` or use SockJS bridge for EventBus-to-browser communication.
12. For production, use the Vert.x Launcher (`io.vertx.core.Launcher`) as the main class; package as a fat JAR with `maven-shade-plugin`, configure `-Dvertx.options.eventLoopPoolSize` based on cores, and enable metrics with `vertx-micrometer-metrics` for Prometheus export.
