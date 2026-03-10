# Fast Context Search (SWE-grep)

Optimized code context finder with trigram indexing, symbol-aware search, and ranked results.

## Triggers
- "fast context", "SWE-grep", "fast search", "symbol search"
- "trigram search", "find context", "code lookup", "quick find"

## Usage
```
/fctx find "handleAuth"               # Find symbol across codebase
/fctx refs "UserService"              # Find all references
/fctx impls "Provider"                # Find implementations of trait
/fctx index                           # Build/rebuild search index
/fctx invalidate src/auth.rs          # Invalidate single file
/fctx stats                           # Show index statistics
```

## Features
- 5 match types: Exact, Fuzzy, Semantic, Structural, Symbol
- Trigram index for fast substring matching
- Symbol-aware search (functions, types, variables)
- Structural search (trait implementations, function callers)
- LRU search cache for repeated queries
- Incremental index updates (invalidate single files)
- Relevance-ranked results (exact > symbol > structural > fuzzy)
- File metadata tracking (hash, symbols, imports, exports)
