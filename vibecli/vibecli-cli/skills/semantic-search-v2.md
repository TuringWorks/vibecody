# semantic-search-v2

Hybrid semantic code search — lexical + structural + embedding.

## Usage

```
/search <query>                                    # hybrid search (default)
/search <query> --strategy lexical|structural|semantic|hybrid
/search <query> --lang rust --max 10
/search <query> --file src/auth
```

## Strategies

| Strategy | Method | Best For |
|----------|--------|----------|
| `lexical` | Token-based inverted index | Exact keyword matches |
| `structural` | Symbol/AST presence | Function/type name lookup |
| `semantic` | Embedding cosine similarity | Conceptual search |
| `hybrid` | 40% lexical + 40% semantic + 20% structural | General use |

## Features

- Inverted index for O(1) token lookup
- Mock embeddings (LCG bag-of-tokens) for offline use; plug in real embeddings at runtime
- Language and file-path filters
- Minimum score threshold (default: 0.1)
- Context window builder: top results assembled into `max_chars` budget
- File removal invalidates and rebuilds affected index entries

## Example

```
> /search "validate user token" --strategy hybrid --lang rust
  [0.87] src/auth.rs:45–62  fn validate_token(token: &str) -> bool
  [0.73] src/session.rs:12–28  fn check_session(sid: &str) -> Option<Session>
  [0.61] src/middleware.rs:88–105  fn auth_middleware(req: &Request) -> Result<()>
```

## Module

`vibecli/vibecli-cli/src/semantic_search_v2.rs`
