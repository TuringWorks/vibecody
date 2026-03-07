---
triggers: ["Scala", "scala 3", "akka", "akka-http", "http4s", "ZIO", "zio-http", "cats effect", "tapir", "pekko"]
tools_allowed: ["read_file", "write_file", "bash"]
requires_bins: ["scala"]
category: scala
---

# Scala Language and Ecosystem

When working with Scala:

1. Use http4s with Cats Effect for purely functional HTTP services — define routes as `HttpRoutes[F]` and compose them with `<+>` (SemigroupK) for modular route organization.
2. In ZIO-based projects, use `ZLayer` for dependency injection — define service traits with `ZIO.serviceWithZIO` accessors and provide layers at the application edge.
3. Use Tapir to define endpoints as values (`endpoint.get.in("users" / path[Int]).out(jsonBody[User])`) and interpret them to http4s, Akka HTTP, or ZIO HTTP for server-agnostic API definitions.
4. Prefer Scala 3 enums and union types over sealed trait hierarchies for ADTs when targeting Scala 3; use `derives` clause for automatic typeclass derivation (Codec, Schema, Show).
5. Model errors as an ADT (`sealed trait AppError`) and use `EitherT[F, AppError, A]` (Cats) or `ZIO[R, AppError, A]` to propagate typed errors through the entire call stack.
6. Use Circe with semi-automatic derivation (`deriveEncoder`/`deriveDecoder`) for JSON — avoid fully automatic derivation in production as it increases compile times and hides codec issues.
7. Configure Akka/Pekko actors with typed behaviors (`Behaviors.receive[Command]`) — never use `classicSystem.actorOf` in new code; prefer the typed actor API for compile-time message safety.
8. Use `Resource[F]` (Cats Effect) or `ZLayer.scoped` (ZIO) for lifecycle-managed resources like database pools, HTTP clients, and server bindings to guarantee cleanup on shutdown.
9. Write property-based tests with ScalaCheck — use `forAll(Gen.alphaNumStr, Gen.posNum[Int])` to generate random inputs and verify invariants instead of testing only specific examples.
10. Use Doobie or Skunk for database access in Cats Effect projects — compose queries as `ConnectionIO` values and transact them through a `Transactor` rather than running raw JDBC.
11. Apply the tagless final pattern (`trait UserRepo[F[_]]`) to abstract over effect types, enabling testing with `IO` or `Id` and production use with `ZIO` or `IO`.
12. Configure sbt with `scalacOptions ++= Seq("-Wunused:all", "-Wvalue-discard")` and enable `-Xfatal-warnings` in CI to catch unused imports, dead code, and discarded values early.
