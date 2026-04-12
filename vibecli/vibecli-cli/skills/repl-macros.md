# repl-macros

Define and invoke parameterized REPL command macros.

## Usage

```
/macro define <name> "<body>"            # define a no-arg macro
/macro define <name> <p1> <p2> "<body>"  # define with parameters (${p1}, ${p2})
/macro list                              # list all macros
/macro delete <name>                     # remove a macro
@<name> [arg1] [arg2 ...]               # invoke macro with positional args
```

## Features

- Named parameter substitution: `${param}` in the macro body
- Default values: `${param:default}` — used when arg is omitted
- Positional invocation via `@name arg1 arg2`
- Named arg invocation via the API
- Use count tracking and top-used ranking
- Built-in macros: `check`, `test-run`, `review-file`, `commit-module`

## Built-in Macros

| Name | Params | Description |
|------|--------|-------------|
| `check` | — | `cargo check --workspace` |
| `test-run` | `module` | Run tests for a specific module |
| `review-file` | `file`, `depth` | Explain a file at a depth (default: overview) |
| `commit-module` | `module`, `msg` | Stage and commit a single module file |

## Example

```
> /macro define greet name "echo Hello, ${name}!"
  Defined macro 'greet' with 1 parameter(s)

> @greet World
  echo Hello, World!

> @review-file src/auth.rs deep
  /explain src/auth.rs --depth deep
```

## Module

`vibecli/vibecli-cli/src/repl_macros.rs`
