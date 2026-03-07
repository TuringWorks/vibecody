---
triggers: ["Zig", "zig lang", "zap zig", "zig http", "zig build system", "zig allocator"]
tools_allowed: ["read_file", "write_file", "bash"]
requires_bins: ["zig"]
category: zig
---

# Zig Web and Systems

When working with Zig for web and systems programming:

1. Use `std.http.Server` for built-in HTTP serving and prefer the non-blocking I/O path with `std.io.poll` to handle concurrent connections without threads.
2. Define a custom allocator strategy per request — use `std.heap.ArenaAllocator` wrapping `std.heap.page_allocator` for request-scoped memory that frees in one shot when the request completes.
3. Structure your `build.zig` with explicit dependency steps: declare library dependencies via `b.dependency()`, expose C headers with `addIncludePath`, and use `b.installArtifact()` to control output artifacts.
4. Leverage `comptime` for route registration — build a dispatch table at compile time using tuple/struct iteration so route matching has zero runtime allocation overhead.
5. Handle errors with Zig's error union types (`!T`) throughout the handler chain; never discard errors silently — use `catch |err|` blocks to log and return proper HTTP status codes.
6. Use `std.json` for parsing and serializing request/response bodies; call `std.json.parseFromSlice` with an arena allocator and define explicit struct types for type-safe deserialization.
7. For TLS support, link against system OpenSSL or use `std.crypto.tls.Client`; configure certificates via `std.crypto.Certificate.Bundle` and always validate peer certificates in production.
8. Use `@embedFile` to include static assets (HTML, CSS, JS) directly in the binary at compile time, eliminating runtime file I/O and simplifying deployment to a single executable.
9. Implement middleware as function pointers with a consistent signature `fn (*Request, *Response, NextFn) anyerror!void` and chain them in a slice iterated by the dispatcher.
10. For concurrent workloads, use `std.Thread.Pool` with a bounded worker count rather than spawning unbounded threads; pass work items via `std.Thread.Pool.spawn`.
11. Write tests inline with `test "name" {}` blocks in each module; run `zig build test` in CI and use `std.testing.expectEqual` and `std.testing.expectError` for assertions.
12. Cross-compile for deployment targets directly from `build.zig` by setting `.target = b.resolveTargetQuery(.{ .os_tag = .linux, .cpu_arch = .x86_64 })` — no separate toolchain needed.
