---
triggers: ["extract method", "refactor pattern", "replace conditional", "introduce parameter", "code refactoring"]
tools_allowed: ["read_file", "write_file", "bash"]
category: review
---

# Refactoring Patterns

When applying refactoring patterns:

1. Extract Method: pull 5+ lines of cohesive logic into a named function — improves readability
2. Extract Variable: name complex expressions — `let is_eligible = age >= 18 && has_id && !is_blocked;`
3. Replace Conditional with Polymorphism: use trait/interface dispatch instead of long if/match chains
4. Introduce Parameter Object: group 3+ related parameters into a struct/class
5. Replace Magic Numbers: use named constants — `const MAX_RETRIES: u32 = 3;`
6. Move Method: relocate methods to the class/module that has the data they need
7. Replace Temp with Query: convert local variables to method calls when recomputation is cheap
8. Decompose Conditional: extract conditions into well-named boolean methods
9. Replace Nested Conditionals with Guard Clauses: early return for special cases
10. Encapsulate Field: make fields private, expose through methods — control access and validation
11. Pull Up / Push Down: move shared behavior to parent, specialized behavior to child
12. Always refactor with tests green — run tests after each transformation step
