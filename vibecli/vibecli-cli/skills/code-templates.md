# Code Generation Templates

Parameterized scaffolding for common code patterns with `{{variable}}` placeholders. Matches GitHub Copilot Workspace v2's snippet/template feature.

## Built-in Templates
| Name | Language | Description |
|---|---|---|
| `rust-struct` | Rust | Struct with derives |
| `rust-enum` | Rust | Enum with derives |
| `rust-tauri-command` | Rust | Async Tauri command |
| `rust-test-module` | Rust | `#[cfg(test)]` scaffold |
| `ts-react-component` | TypeScript | Functional component |
| `ts-async-service` | TypeScript | Async service function |
| `md-adr` | Markdown | Architecture Decision Record |

## Commands
- `/template list [lang]` — list templates (optionally filtered by language)
- `/template render <name> [vars...]` — render a template
- `/template vars <name>` — show required variables

## Examples
```
/template render rust-struct name=Config derives="Debug,Serialize"
# Output:
# #[derive(Debug,Serialize)]
# pub struct Config {
#     // fields here
# }
```
