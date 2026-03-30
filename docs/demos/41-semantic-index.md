---
layout: page
title: "Demo 41: Deep Semantic Codebase Index"
permalink: /demos/41-semantic-index/
---


## Overview

VibeCody's semantic index goes beyond text search by parsing your codebase into an AST (Abstract Syntax Tree) and building a structural graph of functions, types, traits, modules, and their relationships. You can query callers, callees, type hierarchies, trait implementations, and cross-module dependencies at the symbol level. This gives you IDE-quality code intelligence directly in the terminal, powered by tree-sitter parsing and an in-memory graph index.

**Time to complete:** ~8 minutes

## Prerequisites

- VibeCLI v0.5.1 installed and on your PATH
- A source code project (Rust, TypeScript, Python, Go, Java, C/C++ supported)
- For VibeUI: the desktop app running with the **SemanticIndex** panel visible

## Semantic Index vs Text Search

| Feature                | `/semindex` (Semantic)                  | `grep` / `rg` (Text)              |
|------------------------|-----------------------------------------|------------------------------------|
| **Understands scope**  | Yes -- knows function boundaries        | No -- matches any line             |
| **Call graph**         | Yes -- callers and callees              | No                                 |
| **Type hierarchy**     | Yes -- inheritance, trait impls         | No                                 |
| **Rename-safe**        | Yes -- tracks symbols, not strings      | No -- matches text literally       |
| **Cross-file**         | Yes -- follows imports and modules      | Yes -- but no structural context   |
| **Speed on large repos** | Fast (indexed, O(1) lookup)           | Fast (brute force, O(n) scan)      |

## Step-by-Step Walkthrough

### 1. Build the Semantic Index

Index your entire codebase. VibeCody parses each file into an AST, extracts symbols, and builds a call graph.

**REPL:**

```bash
vibecli
> /semindex build
```

Example output:

```
Building semantic index...

  Parsing files:  ████████████████████████████████████████ 247/247
  Languages:      Rust (198), TypeScript (42), TOML (7)
  Parse time:     1.8s

Index Statistics:
  Functions:      1,284
  Types/Structs:  312
  Traits:         87
  Impls:          203
  Modules:        64
  Constants:      156
  Call edges:     4,821
  Type edges:     589

Index saved to: .vibecli/semindex.db (2.4 MB)
Incremental updates enabled — changed files re-indexed on save.
```

The index is stored locally and updates incrementally when files change.

### 2. Find Callers of a Function

Discover every location that calls a specific function.

**REPL:**

```bash
vibecli
> /semindex callers validate_token
```

Example output:

```
Callers of validate_token (src/auth/jwt.rs:45):

  #  File                          Line  Function              Context
  1  src/api/middleware.rs          23    auth_middleware        let claims = validate_token(&token, &secret)?;
  2  src/api/handlers.rs           67    get_user_profile       let claims = validate_token(&bearer, &key)?;
  3  src/api/handlers.rs           112   update_user            if let Ok(c) = validate_token(&t, &k) {
  4  src/ws/connection.rs          34    on_connect             validate_token(&msg.token, &config.secret)
  5  tests/auth/jwt_tests.rs       15    test_valid_token       let result = validate_token(&token, "secret");
  6  tests/auth/jwt_tests.rs       28    test_expired_token     let result = validate_token(&expired, "secret");

Found 6 callers across 4 files.
```

### 3. Find Callees of a Function

See what a function calls internally.

**REPL:**

```bash
vibecli
> /semindex callees handle_request
```

Example output:

```
Callees of handle_request (src/api/router.rs:89):

  #  Function              File                         Line
  1  parse_route           src/api/router.rs            12
  2  validate_token        src/auth/jwt.rs              45
  3  rate_limit_check      src/api/middleware.rs         78
  4  execute_handler       src/api/router.rs            134
  5  log_request           src/observability/logger.rs  22
  6  serialize_response    src/api/response.rs          56

Found 6 callees.

Call depth 2 (transitive):
  validate_token → decode_jwt, verify_signature, check_expiry
  rate_limit_check → get_bucket, increment_counter
  execute_handler → (dynamic dispatch — 12 possible handlers)
```

### 4. Explore Type Hierarchy

View inheritance chains, trait implementations, and struct relationships.

**REPL:**

```bash
vibecli
> /semindex hierarchy ApiError
```

Example output:

```
Type Hierarchy for ApiError (src/api/error.rs:8):

  ApiError (enum)
  ├── Variants:
  │   ├── NotFound(String)
  │   ├── Unauthorized(String)
  │   ├── BadRequest(String)
  │   ├── Internal(String)
  │   └── Database(DbError)
  │
  ├── Implements:
  │   ├── std::fmt::Display         src/api/error.rs:25
  │   ├── std::error::Error         src/api/error.rs:38
  │   ├── From<DbError>             src/api/error.rs:45
  │   ├── From<serde_json::Error>   src/api/error.rs:52
  │   └── IntoResponse (axum)       src/api/error.rs:59
  │
  └── Used by:
      ├── handle_request            src/api/router.rs:89
      ├── get_user_profile          src/api/handlers.rs:67
      ├── create_user               src/api/handlers.rs:145
      └── (14 more functions)

Related types:
  DbError → ApiError (via From impl)
  ApiError → axum::Response (via IntoResponse impl)
```

### 5. Find Trait Implementations

List all types that implement a specific trait.

**REPL:**

```bash
vibecli
> /semindex impls AIProvider
```

Example output:

```
Implementations of trait AIProvider (crates/vibe-ai/src/provider.rs:12):

  #  Type              File                                      Methods
  1  OllamaProvider    crates/vibe-ai/src/ollama.rs:28           chat, stream, models
  2  ClaudeProvider    crates/vibe-ai/src/claude.rs:35           chat, stream, models
  3  OpenAIProvider    crates/vibe-ai/src/openai.rs:30           chat, stream, models
  4  GeminiProvider    crates/vibe-ai/src/gemini.rs:22           chat, stream, models
  5  GrokProvider      crates/vibe-ai/src/grok.rs:18             chat, stream, models
  6  GroqProvider      crates/vibe-ai/src/groq.rs:20             chat, stream, models
  7  MistralProvider   crates/vibe-ai/src/mistral.rs:25          chat, stream, models
  8  DeepSeekProvider  crates/vibe-ai/src/deepseek.rs:19         chat, stream, models
  ...

Found 18 implementations.
```

### 6. Search Symbols by Name

Find symbols matching a pattern across the entire index.

**REPL:**

```bash
vibecli
> /semindex search "validate*"
```

Example output:

```
Symbols matching "validate*":

  #  Symbol                Kind       File                         Line
  1  validate_token        function   src/auth/jwt.rs              45
  2  validate_input        function   src/api/validation.rs        12
  3  validate_email        function   src/api/validation.rs        34
  4  validate_config       function   src/config.rs                89
  5  ValidateRequest       struct     src/api/handlers.rs          22
  6  Validator             trait      src/api/validation.rs        5

Found 6 symbols.
```

### 7. View Module Dependencies

See how modules depend on each other.

**REPL:**

```bash
vibecli
> /semindex deps src/api
```

Example output:

```
Module Dependencies for src/api/:

  src/api/
  ├── imports from:
  │   ├── src/auth/          (jwt, sessions)
  │   ├── src/db/            (queries, models)
  │   ├── src/observability/ (logger, metrics)
  │   └── external:
  │       ├── axum           (Router, Json, Extension)
  │       ├── serde          (Deserialize, Serialize)
  │       └── tokio          (spawn, sleep)
  │
  └── imported by:
      ├── src/main.rs
      └── tests/integration/

Circular dependencies: none detected
```

### 8. Incremental Updates

The index updates automatically when files change. You can also trigger a manual refresh.

**REPL:**

```bash
vibecli
> /semindex refresh
```

Example output:

```
Incremental index update:
  Changed files:  3 (src/api/handlers.rs, src/auth/jwt.rs, Cargo.toml)
  Re-parsed:      2 source files
  New symbols:    +4 (2 functions, 1 struct, 1 impl)
  Removed:        -1 (1 function deleted)
  Call edges:     +8, -3
  Update time:    0.2s
```

### 9. VibeUI SemanticIndex Panel

Open the **SemanticIndex** panel in VibeUI to see:

- **Overview** tab: index statistics, language breakdown, last build time
- **Explorer** tab: browse the symbol tree by module, click to jump to source
- **Call Graph** tab: interactive visualization of caller/callee relationships
- **Hierarchy** tab: type hierarchy trees with trait implementation details
- **Search** tab: symbol search with kind filters (function, type, trait, const)

## Configuration Reference

Add semantic index settings to `~/.vibecli/config.toml`:

```toml
[semindex]
enabled = true
auto_refresh = true
languages = ["rust", "typescript", "python", "go"]
exclude = ["target/", "node_modules/", ".git/"]
max_depth = 3
index_path = ".vibecli/semindex.db"
```

## Demo Recording JSON

```json
{
  "meta": {
    "title": "Deep Semantic Codebase Index",
    "description": "Build an AST-level index of your codebase and query callers, callees, and type hierarchies.",
    "duration_seconds": 180,
    "version": "0.5.1"
  },
  "steps": [
    {
      "id": 1,
      "action": "repl",
      "commands": [
        { "input": "/semindex build", "delay_ms": 8000 }
      ],
      "description": "Build the semantic index for the entire codebase"
    },
    {
      "id": 2,
      "action": "repl",
      "commands": [
        { "input": "/semindex callers validate_token", "delay_ms": 3000 }
      ],
      "description": "Find all callers of a function"
    },
    {
      "id": 3,
      "action": "repl",
      "commands": [
        { "input": "/semindex callees handle_request", "delay_ms": 3000 }
      ],
      "description": "Find all callees of a function"
    },
    {
      "id": 4,
      "action": "repl",
      "commands": [
        { "input": "/semindex hierarchy ApiError", "delay_ms": 3000 }
      ],
      "description": "Explore the type hierarchy"
    },
    {
      "id": 5,
      "action": "repl",
      "commands": [
        { "input": "/semindex impls AIProvider", "delay_ms": 3000 }
      ],
      "description": "List all implementations of a trait"
    },
    {
      "id": 6,
      "action": "repl",
      "commands": [
        { "input": "/semindex search \"validate*\"", "delay_ms": 2000 }
      ],
      "description": "Search symbols by name pattern"
    },
    {
      "id": 7,
      "action": "repl",
      "commands": [
        { "input": "/semindex deps src/api", "delay_ms": 3000 }
      ],
      "description": "View module dependency graph"
    },
    {
      "id": 8,
      "action": "vibeui_interaction",
      "panel": "SemanticIndex",
      "tab": "Call Graph",
      "description": "Visualize caller/callee relationships in VibeUI"
    },
    {
      "id": 9,
      "action": "vibeui_interaction",
      "panel": "SemanticIndex",
      "tab": "Hierarchy",
      "description": "Explore type hierarchies visually"
    }
  ]
}
```

## What's Next

- [Demo 8: Code Search & Embeddings](../08-code-search/) -- Text-level semantic search
- [Demo 10: Code Transforms](../10-code-transforms/) -- AST-based refactoring tools
- [Demo 40: Web Search Grounding](../40-web-grounding/) -- Enrich answers with live web data
