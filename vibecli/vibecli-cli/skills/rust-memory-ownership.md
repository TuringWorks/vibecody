---
triggers: ["borrow checker", "lifetime", "ownership", "Pin", "Drop", "smart pointer", "Rc", "Arc", "Box", "Cow"]
tools_allowed: ["read_file", "write_file", "bash"]
requires_bins: ["cargo"]
category: rust
---

# Rust Memory & Ownership

When working with Rust's ownership system:

1. Prefer borrowing (`&T` / `&mut T`) over cloning — clone only when ownership transfer is needed
2. Use `Cow<'a, str>` when a function might or might not need to allocate
3. Use `Box<T>` for heap allocation with single ownership; `Rc<T>` for shared ownership (single-thread)
4. Use `Arc<T>` for shared ownership across threads — combine with `Mutex` or `RwLock` for mutation
5. Lifetime elision rules handle most cases — add explicit lifetimes only when the compiler requires them
6. Named lifetimes: `'a` for the primary, `'b` for secondary — use descriptive names for complex cases
7. Use `Pin<Box<T>>` for self-referential types and async futures that must not move in memory
8. Implement `Drop` for cleanup (file handles, network connections) — never panic in `Drop`
9. Use `std::mem::take` and `std::mem::replace` to move values out of mutable references
10. Avoid `'static` lifetimes on borrowed data — it usually means you should own the data instead
11. Use `String` for owned strings, `&str` for borrowed — accept `impl AsRef<str>` in public APIs
12. `Vec<T>` owns its elements; use `&[T]` for borrowed slices in function parameters
