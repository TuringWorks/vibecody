---
triggers: ["Carbon", "carbon lang", "carbon language", "carbon cpp successor", "carbon generics"]
tools_allowed: ["read_file", "write_file", "bash"]
category: carbon
---

# Carbon Language

When working with Carbon:

1. Use `fn` for function declarations with explicit return types after `->` and parameter types before names; Carbon enforces type annotations everywhere for clarity.
2. Define structs with `class` and use `var` for mutable fields, `let` for immutable; prefer immutable bindings by default to signal intent and enable compiler optimizations.
3. Leverage Carbon's checked generics with `interface` and `impl` blocks; generic parameters use `:!` syntax (e.g., `fn Sort[T:! Comparable](arr: Slice(T))`) for compile-time type checking.
4. Use `choice` types (tagged unions) with `match` for exhaustive pattern matching; the compiler warns on non-exhaustive matches, preventing missed cases at compile time.
5. Interop with C++ by using `import Cpp library` to pull in existing headers; wrap C++ types gradually rather than rewriting entire libraries.
6. Apply Carbon's name lookup rules: unqualified names resolve within the current package first, then imports; use `package` declarations at file top and `namespace` for sub-grouping.
7. Use `impl` blocks to implement interfaces for types, keeping the type definition separate from its behavior; this enables retroactive interface implementation on third-party types.
8. Prefer Carbon's pointer and reference model (`T*` for pointers, `T&` for references) with explicit ownership annotations to improve memory safety over raw C++ patterns.
9. Structure projects with one `package` per directory and a `BUILD` file for the build system; Carbon uses Bazel-compatible build rules for compilation.
10. Handle errors with `Optional(T)` and result-like patterns rather than exceptions; use `match` on the result to handle success and error paths explicitly.
11. Write tests using Carbon's test framework with `fn TestName()` annotated as test entries; assert with built-in `Assert` and run via the build system's test command.
12. Keep C++ interop boundaries thin by defining Carbon wrapper types that own the C++ object lifecycle; use RAII in the Carbon wrapper to prevent leaks across the language boundary.
