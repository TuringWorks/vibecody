---
triggers: ["functional programming", "FP patterns", "immutability", "pure functions", "monads", "functors", "algebraic data types", "pattern matching", "higher-order functions", "currying"]
tools_allowed: ["read_file", "write_file", "bash"]
category: architecture
---

# Functional Programming Patterns

When working with functional programming patterns:

1. Prefer pure functions that take inputs and return outputs with no side effects — push I/O and state mutation to the edges of your program. This makes every function independently testable without mocking.

2. Use algebraic data types (sum types + product types) to model domain states exhaustively — e.g., `Result<T, E>`, `Option<T>`, or `Either<L, R>`. Let the compiler enforce that all cases are handled via pattern matching.

3. Favor immutable data structures by default — use persistent data structures or copy-on-write semantics. When mutation is needed for performance, isolate it behind a pure interface (e.g., Rust's interior mutability, Clojure's transients).

4. Apply the functor pattern (`map`) to transform values inside containers without unwrapping them: `option.map(x => x + 1)` works uniformly across Option, Result, List, Future, and any custom type implementing `map`.

5. Chain dependent computations with monadic `flatMap`/`and_then` — this replaces nested null checks and try-catch blocks: `parseInput(s).and_then(validate).and_then(save)` short-circuits on the first failure.

6. Use higher-order functions (`map`, `filter`, `fold`/`reduce`) instead of manual loops — they communicate intent clearly and compose naturally. Prefer `fold` for accumulation: `items.fold(0, |acc, x| acc + x.price)`.

7. Apply currying and partial application to build specialized functions from general ones: `const add = a => b => a + b; const add5 = add(5);` — this enables point-free composition and reusable building blocks.

8. Model errors as values, not exceptions — return `Result<T, E>` or `Either<Error, T>` and propagate with `?` or `flatMap`. Reserve exceptions/panics for truly unrecoverable situations.

9. Use pattern matching for control flow instead of if-else chains — it is exhaustive, self-documenting, and lets you destructure data in the match arms: `match shape { Circle(r) => PI * r * r, Rect(w, h) => w * h }`.

10. Design with composition over inheritance — build complex behavior by composing small functions: `const process = pipe(parse, validate, transform, serialize)`. Use function composition utilities from your language's FP library.

11. Leverage type classes / traits / protocols to achieve ad-hoc polymorphism — define behavior contracts (e.g., `Eq`, `Ord`, `Serialize`) that types opt into, avoiding class hierarchies while enabling generic programming.

12. Keep functions small and single-purpose — if a function does two things, split it into two functions and compose them. Name functions as verbs describing the transformation: `validateEmail`, `normalizeAddress`, `calculateTotal`.
