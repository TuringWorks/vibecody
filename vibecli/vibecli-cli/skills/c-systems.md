---
triggers: ["C programming", "malloc", "valgrind", "POSIX", "socket programming", "systems programming C", "memory management C"]
tools_allowed: ["read_file", "write_file", "bash"]
requires_bins: ["gcc"]
category: c
---

# C Systems Programming

When writing systems-level C code:

1. Always check return values of `malloc`, `fopen`, `read`, `write` — handle `NULL`/`-1` cases
2. Use `valgrind --leak-check=full` to detect memory leaks and use-after-free
3. Use `AddressSanitizer` (`-fsanitize=address`) for runtime memory error detection
4. POSIX socket pattern: `socket()` → `bind()` → `listen()` → `accept()` → `read()`/`write()` → `close()`
5. Use `sizeof(type)` not `sizeof(variable)` in `malloc`: `int *arr = malloc(n * sizeof(int));`
6. Always `free()` allocated memory — use cleanup goto pattern for error paths
7. Use `snprintf()` over `sprintf()` to prevent buffer overflows
8. Use `const` for read-only parameters: `void process(const char *input, size_t len)`
9. Use `static` for file-scoped functions and variables — limit symbol visibility
10. Signal handling: use `sigaction()` over `signal()` — more portable and reliable
11. Use `poll()` or `epoll()` for multiplexed I/O — avoid `select()` for large fd sets
12. Compile with `-Wall -Wextra -Werror` — treat warnings as errors
