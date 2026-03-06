---
triggers: ["trait bounds", "impl Trait", "dyn dispatch", "generics rust", "associated type", "where clause", "trait object"]
tools_allowed: ["read_file", "write_file", "bash"]
requires_bins: ["cargo"]
category: rust
---

# Rust Traits & Generics

When designing with traits and generics:

1. Use `impl Trait` in argument position for simple cases: `fn process(item: impl Display)`
2. Use `dyn Trait` (trait objects) when you need runtime polymorphism or heterogeneous collections
3. Prefer associated types over generic parameters when there's only one valid type per impl
4. Use `where` clauses for complex bounds instead of inline: `fn foo<T>(x: T) where T: Clone + Debug`
5. Implement `From<T>` instead of `Into<T>` — you get `Into` for free via blanket impl
6. Use `#[derive(Debug, Clone, PartialEq)]` — derive what you can, implement what you must
7. Prefer `AsRef<str>` / `AsRef<Path>` in function parameters for flexible input types
8. Use `trait Foo: Send + Sync + 'static` for traits used across thread/task boundaries
9. Seal traits with a private supertrait to prevent external implementations
10. Use extension traits (e.g., `IteratorExt`) to add methods without orphan rule issues
11. Prefer static dispatch (`impl Trait` / generics) for performance; use `Box<dyn Trait>` when needed
12. Use `Default` trait for builder-pattern defaults and `..Default::default()` struct updates
