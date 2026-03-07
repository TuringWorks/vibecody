---
triggers: ["effect system", "IO monad", "ZIO", "cats effect", "arrow", "algebraic effects", "effect handlers", "free monad"]
tools_allowed: ["read_file", "write_file", "bash"]
category: architecture
---

# Effect Systems and IO Monads

When working with effect systems and IO monads:

1. Wrap all side effects (I/O, randomness, time, mutable state) in an effect type (`IO`, `Task`, `ZIO`) — never perform side effects in pure business logic. The effect is a description of work, not its execution.

2. Compose effects with `flatMap`/`for`-comprehensions to build programs as data: `for { config <- loadConfig; db <- connectDb(config); users <- db.query("SELECT ...") } yield users` — nothing executes until the runtime interprets the composed program.

3. In ZIO, use the environment type (`R`) for dependency injection: `ZIO[Database & Logger, AppError, User]` declares what the effect needs, what can fail, and what it produces — wire dependencies at the edge with `ZLayer`.

4. Handle errors in the effect channel, not with exceptions: `io.mapError(toDomainError).catchSome { case NotFound => fallback }`. Use typed error channels (`ZIO[R, E, A]`) to make failure modes explicit in the type signature.

5. Use `Resource`/`ZManaged`/`Scope` for safe resource lifecycle management — acquire in the open step, release in the close step, and the runtime guarantees cleanup even on errors or interruption: `Resource.make(openFile)(closeFile)`.

6. Model concurrent operations with fiber-based primitives: `ZIO.foreachPar(urls)(fetch)` for parallel execution, `race` for first-to-complete, `zipPar` for joining independent effects. Fibers are lightweight — use thousands without concern.

7. Implement retry and timeout policies declaratively: `effect.retry(Schedule.exponential(1.second) && Schedule.recurs(5)).timeout(30.seconds)` — the schedule is composable and testable independently.

8. Use algebraic effects or free monads to define domain-specific DSLs: define an algebra (`sealed trait DbOp[A]`), build programs using `Free[DbOp, A]`, then provide interpreters for production (real DB) and test (in-memory map).

9. In Cats Effect, use `Ref` for concurrent mutable state, `Deferred` for one-shot synchronization, and `Queue` for producer-consumer patterns — these are purely functional alternatives to locks and blocking queues.

10. Test effectful code by providing test interpreters: swap the real `Clock` with `TestClock`, `Random` with `TestRandom`, and database layers with in-memory stubs — all wired through the effect system's dependency mechanism.

11. Use `Fiber.interrupt` and structured concurrency to ensure no leaked fibers — parent effects should supervise child fibers and cancel them on completion or error. ZIO Scope and Cats Effect `Supervisor` enforce this.

12. Keep effect types in signatures at the boundary and push pure logic into plain functions: `def validate(input: Input): Either[Error, Valid]` is pure and testable, `def process(input: Input): ZIO[Db, Error, Result]` wraps the effectful boundary around it.
