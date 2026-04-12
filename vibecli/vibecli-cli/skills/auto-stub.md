# Auto Stub Generator

Generate test stubs and mock implementations from function signatures and trait/interface definitions. Supports Rust and TypeScript. Matches Devin 2.0's automated test stub generator.

## When to Use
- Bootstrapping TDD — generate `#[test]` stubs for all functions in a new module
- Creating mock implementations of traits for unit tests
- Generating TypeScript `it()` stubs for a new service module
- Batch-generating stubs for an entire file with `generate_all()`

## Generated Stub Types
- **TestFunction** — `#[test] fn test_xxx()` with arrange/act/assert skeleton
- **MockImpl** — struct + `impl TraitName` with `unimplemented!()` bodies
- **SpyImpl** — mock that records call arguments (for assertion later)

## Default Values by Type (Rust)
| Type | Default |
|---|---|
| `String` / `&str` | `String::new()` |
| `bool` | `false` |
| `usize` / `i32` / … | `0` |
| `Vec<T>` | `vec![]` |
| `Option<T>` | `None` |
| `Result<T, E>` | `Ok(Default::default())` |

## Commands
- `/stub generate <file>` — generate test stubs for all public functions
- `/stub list` — list functions that don't have tests yet
- `/stub apply <file>` — write stubs to the test file

## Examples
```
/stub generate src/calculator.rs
# Generated 4 test stubs for: add, subtract, multiply, divide

/stub apply src/calculator.rs
# Written to tests/calculator_test.rs

# Generated stub:
# #[test]
# fn test_add() {
#     let x = 0;
#     let y = 0;
#     let result = add(x, y);
#     assert_eq!(result, 0);
# }
```
