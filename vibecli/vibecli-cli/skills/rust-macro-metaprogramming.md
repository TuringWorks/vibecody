---
triggers: ["proc macro", "derive macro", "macro_rules", "quote", "syn", "TokenStream", "metaprogramming rust"]
tools_allowed: ["read_file", "write_file", "bash"]
requires_bins: ["cargo"]
category: rust
---

# Rust Macros & Metaprogramming

When writing Rust macros:

1. Prefer `macro_rules!` for simple pattern-matching macros — they're faster to compile than proc macros
2. Use `#[derive(MyTrait)]` proc macros when you need to generate impl blocks from struct definitions
3. Proc macros must live in a separate crate with `proc-macro = true` in Cargo.toml
4. Use `syn` to parse `TokenStream` into an AST, and `quote!` to generate code back
5. Use `#[proc_macro_attribute]` for attribute macros that transform items (e.g., `#[route("/api")]`)
6. Use `#[proc_macro]` for function-like macros: `my_macro!(input)`
7. Test proc macros with `trybuild` for compile-fail tests and `cargo expand` to inspect output
8. In `macro_rules!`, use `$($item:expr),*` for repeating patterns; `$(,)?` for optional trailing comma
9. Use `stringify!` and `concat!` for compile-time string manipulation in declarative macros
10. Avoid deep macro recursion — Rust has a default recursion limit of 128
11. Use `paste!` crate for identifier concatenation: `paste! { [<my_ $name>] }`
12. Document macros with `/// # Examples` showing usage — `cargo doc` renders them properly
