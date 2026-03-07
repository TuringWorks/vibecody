---
triggers: ["Play Framework", "play2", "playframework", "play java", "play scala", "sbt play"]
tools_allowed: ["read_file", "write_file", "bash"]
requires_bins: ["java"]
category: java
---

# Play Framework

When working with Play Framework (Java and Scala):

1. Define routes in `conf/routes` using the Play routing DSL (`GET /users controllers.UserController.list()`); keep route files organized and use route file includes with `-> /api api.Routes` for modular route namespacing.
2. Use asynchronous actions by returning `CompletionStage<Result>` in Java or `Future[Result]` in Scala — Play is built on Akka/Pekko and is non-blocking by default; never block in controller actions without a custom execution context.
3. Configure custom execution contexts in `application.conf` under `akka.actor` for blocking operations (JDBC calls, file I/O); inject and use them with `CompletableFuture.supplyAsync(() -> ..., dbExecutor)` to avoid starving the default dispatcher.
4. Use Play's built-in JSON support — in Java, use `Json.toJson()` and `Json.fromJson()` with Jackson; in Scala, define implicit `Format[T]` or `Reads[T]`/`Writes[T]` with the Play JSON combinators.
5. Leverage Play's form handling and validation with `Form<T>` in Java or `Form[T]` in Scala; bind from requests with `form.bindFromRequest()` and use constraint annotations or custom validators for input safety.
6. Access databases with Play's built-in Slick integration (Scala) or JPA/EBean (Java); configure connection pools in `application.conf` under `db.default` and use Play Evolutions for schema migrations via `conf/evolutions/default/`.
7. Write tests with `WithApplication` or `WithServer` base classes; use `Helpers.fakeRequest()` and `Helpers.route()` for testing controllers without HTTP overhead, and `WSTestClient` for full integration tests with a running server.
8. Use dependency injection throughout — Play 2.x+ uses Guice by default; annotate controllers with `@Inject` (Java) or `@Inject()` (Scala) and bind custom implementations in a `Module` class registered in `application.conf`.
9. Manage configuration with `application.conf` (HOCON format) and environment-specific overrides using `-Dconfig.resource=production.conf`; use `play.api.Configuration` injection instead of hardcoding `ConfigFactory.load()`.
10. Implement WebSocket handlers by returning `WebSocket` from controller methods; use Akka Streams `Flow[Message, Message, _]` in Scala or `ActorFlow.actorRef()` to handle bidirectional real-time communication.
11. Enable hot-reload in development with `sbt run`; use `~compile` for continuous compilation and `sbt dist` to produce a production-ready zip with startup scripts — avoid `sbt stage` for production since it lacks the wrapper script.
12. For production deployment, run the generated script in `target/universal/stage/bin/` with `-Dplay.http.secret.key` set to a strong secret; configure `play.filters.hosts.allowed` to prevent host header attacks and enable `play.filters.csrf` for form submissions.
