---
triggers: ["Undertow", "undertow handler", "XNIO", "wildfly undertow", "java nio server", "netty java"]
tools_allowed: ["read_file", "write_file", "bash"]
requires_bins: ["java"]
category: java
---

# Undertow and High-Performance Java HTTP

When working with Undertow and high-performance Java HTTP servers:

1. Create servers with the builder: `Undertow.builder().addHttpListener(8080, "0.0.0.0").setHandler(handler).build().start()`; the handler is the single entry point for all requests.
2. Compose handlers using `RoutingHandler` for path-based dispatch: `new RoutingHandler().get("/api/users", usersHandler).get("/api/users/{id}", userHandler).setFallbackHandler(notFoundHandler)`.
3. Use `HttpHandler` as the core abstraction: implement `handleRequest(HttpServerExchange exchange)` and always dispatch to a worker thread with `exchange.dispatch(handler)` for blocking operations.
4. Never perform blocking I/O on IO threads: check `exchange.isInIoThread()` and dispatch with `exchange.dispatch(blockingHandler)` or wrap with `BlockingHandler` to move execution to the worker pool.
5. Configure worker threads and IO threads separately: `builder.setWorkerThreads(64).setIoThreads(Runtime.getRuntime().availableProcessors() * 2)` matches IO threads to cores and sizes worker pool for blocking workloads.
6. Use `PathHandler` and `PathTemplateHandler` for URL matching: `new PathHandler().addPrefixPath("/api", apiHandler).addExactPath("/health", healthHandler)` supports both prefix and exact matching.
7. Read request bodies asynchronously: use `exchange.getRequestReceiver().receiveFullBytes((ex, bytes) -> { })` or `receiveFullString` instead of blocking `InputStream` reads for maximum throughput.
8. Send responses efficiently: `exchange.getResponseHeaders().put(Headers.CONTENT_TYPE, "application/json")` then `exchange.getResponseSender().send(jsonString)` for simple text; use `ByteBuffer` for binary data.
9. Implement middleware with handler wrapping: create handlers that delegate to inner handlers after preprocessing, like `new SecurityHandler(new LoggingHandler(routingHandler))` for a clean filter chain.
10. Use Undertow's WebSocket support: `Handlers.websocket((exchange, channel) -> { channel.getReceiveSetter().set(listener); channel.resumeReceives(); })` for low-overhead bidirectional communication.
11. Enable HTTP/2 with `builder.setServerOption(UndertowOptions.ENABLE_HTTP2, true)` and configure TLS with `SSLContext`; Undertow supports h2 and h2c (cleartext HTTP/2) for maximum protocol flexibility.
12. For Netty-based servers, structure the pipeline with `ChannelInitializer`: add `HttpServerCodec`, `HttpObjectAggregator`, and custom `SimpleChannelInboundHandler<FullHttpRequest>` in order; tune `EventLoopGroup` sizes based on benchmark results.
