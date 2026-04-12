# Spec-to-Test Generator

BDD Gherkin spec → test stub generator for Rust, TypeScript, and Python. Matches Copilot Workspace v2 and Devin 2.0.

## When to Use
- Converting a `.feature` file into a test skeleton
- Bootstrapping unit tests from a written specification
- Ensuring every scenario has a corresponding test function
- Supporting TDD by generating stubs before implementation

## Commands
- `/spec-to-test generate <feature-file> [--lang rust|ts|py]` — Generate stubs
- `/spec-to-test parse <feature-file>` — Parse and show scenarios
- `/spec-to-test preview <feature-file>` — Preview generated stubs
- `/spec-to-test write <feature-file> <output-file>` — Write stubs to file

## Generated Output
Each Gherkin `Scenario:` becomes:
- **Rust**: `#[test] fn test_<snake_name>() { todo!(...) }`
- **TypeScript**: `it('<title>', () => { throw new Error('not implemented') })`
- **Python**: `def test_<snake_name>(): raise NotImplementedError`

## Example
```
/spec-to-test generate tests/features/auth.feature --lang rust
# → tests/user_login_spec.rs with 2 #[test] stubs
```
