---
triggers: ["rust error", "anyhow", "thiserror", "Result type", "error handling"]
tools_allowed: ["read_file", "write_file", "bash"]
requires_bins: ["cargo"]
category: rust
---

# Rust Error Handling

When implementing error handling in Rust:

1. Use `thiserror` for library error types (custom `#[derive(Error)]` enums)
2. Use `anyhow::Result` for application code and CLI tools
3. Prefer `?` operator over `.unwrap()` — never unwrap in production code
4. Use `.context("descriptive message")` from anyhow for actionable errors
5. Map errors at boundaries: `map_err(|e| MyError::from(e))`
6. Use `#[from]` attribute on thiserror variants for automatic conversion
7. Return `Result<T, Box<dyn std::error::Error>>` only when you can't use anyhow/thiserror
