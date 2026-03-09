# AST-Aware Code Editing

Apply deterministic code edits using AST node targeting instead of text-based diffs.

## Triggers
- "ast edit", "structural edit", "rename function", "move method"
- "refactor struct", "extract function", "wrap in module"

## Usage
```
/ast load src/main.rs         # Parse file into AST
/ast find Config               # Find node by name
/ast rename Color Colour       # Rename symbol
/ast delete unused_fn          # Remove a function
/ast wrap helper "mod utils {" # Wrap node in block
```

## Features
- Multi-language parsing: Rust, TypeScript/JavaScript, Python
- 16 node kinds: Function, Method, Class, Struct, Enum, Trait, Interface, Import, Module, Block, Field, Parameter, Variable, Constant, TypeAlias, Impl
- 7 edit operations: Replace, InsertBefore, InsertAfter, Delete, Wrap, Rename, Move
- Confidence-based gating (configurable min threshold)
- Preview edits before applying
- Conflict detection (duplicate targets, missing nodes)
- Signature extraction and parent-child relationships
