---
triggers: ["C++17", "C++20", "smart pointer", "RAII", "move semantics", "unique_ptr", "shared_ptr", "modern C++"]
tools_allowed: ["read_file", "write_file", "bash"]
requires_bins: ["g++"]
category: cpp
---

# Modern C++ (C++17/20)

When writing modern C++:

1. Use `std::unique_ptr` for single ownership, `std::shared_ptr` for shared — avoid raw `new`/`delete`
2. Use `auto` for type inference — `auto result = computeValue();`
3. Use `std::optional<T>` instead of pointers or sentinel values for maybe-absent values
4. Use `std::string_view` for non-owning string references — avoid `const char*`
5. Use structured bindings: `auto [key, value] = map.find(x);`
6. Use `constexpr` for compile-time computation; `if constexpr` for compile-time branching
7. Use `std::variant` instead of `union` — type-safe with `std::visit`
8. Use range-based for loops: `for (const auto& item : container)`
9. RAII: acquire resources in constructors, release in destructors — never leak
10. Move semantics: implement move constructor/assignment for types with owned resources
11. Use `std::filesystem` for path manipulation, directory iteration, file operations
12. Use `[[nodiscard]]` on functions whose return value must not be ignored
