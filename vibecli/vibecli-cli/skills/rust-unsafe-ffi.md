---
triggers: ["unsafe rust", "FFI", "bindgen", "raw pointer", "transmute", "extern C", "ffi binding"]
tools_allowed: ["read_file", "write_file", "bash"]
requires_bins: ["cargo"]
category: rust
---

# Rust Unsafe & FFI

When working with unsafe Rust and FFI:

1. Minimize `unsafe` blocks — wrap them in safe abstractions with documented invariants
2. Every `unsafe` block must have a `// SAFETY:` comment explaining why it's sound
3. Use `bindgen` to auto-generate Rust bindings from C headers — don't hand-write them
4. For FFI functions, use `extern "C"` and `#[no_mangle]` on the Rust side
5. Convert C strings with `CStr::from_ptr()` (borrowed) or `CString::new()` (owned)
6. Never use `transmute` when `as` casts or `from_raw_parts` suffice
7. Use `ManuallyDrop` or `mem::forget` carefully — prefer RAII wrappers
8. Raw pointer rules: `*const T` for reads, `*mut T` for writes — check null before deref
9. Use `Pin<Box<T>>` for self-referential structs that cross FFI boundaries
10. Test FFI code with Miri (`cargo +nightly miri test`) to catch undefined behavior
11. Use `cbindgen` to generate C headers from Rust code for reverse FFI
12. Wrap FFI resources in a struct with `Drop` impl for automatic cleanup
