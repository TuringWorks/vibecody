# Web Grounding

Search the web mid-task to find documentation, API references, Stack Overflow solutions, and changelog entries. Grounds agent responses in up-to-date information rather than relying solely on training data.

## When to Use
- Looking up current API documentation for a library or framework
- Finding solutions to error messages or stack traces
- Checking latest release notes or migration guides
- Verifying best practices against current community standards
- Researching unfamiliar libraries before suggesting them as dependencies

## Commands
- `/web search <query>` — Search the web and return summarized results
- `/web docs <library> <topic>` — Search official docs for a specific topic
- `/web changelog <package> <version>` — Find changelog entries for a version
- `/web stackoverflow <error>` — Search Stack Overflow for an error message
- `/web verify <claim>` — Fact-check a technical claim against web sources
- `/web cache` — Show cached web results from this session
- `/web clear-cache` — Clear the web result cache

## Examples
```
/web search "tokio 1.36 breaking changes"
# Found 4 results:
# 1. tokio.rs/blog/2024-02-tokio-1.36 — New io_uring support, ...
# 2. github.com/tokio-rs/tokio/releases/tag/tokio-1.36.0 — ...

/web docs react-query "mutation error handling"
# From tanstack.com/query/latest/docs/react/guides/mutations:
# Use onError callback or the error property from useMutation...

/web stackoverflow "EPERM operation not permitted npm install windows"
# Top answer (score 342): Run terminal as Administrator, or...
```

## Best Practices
- Prefer official documentation sources over blog posts for accuracy
- Cache results within a session to avoid redundant searches
- Verify version-specific information matches your actual dependency version
- Use web grounding for anything released after the model training cutoff
- Combine web results with codebase context for the most relevant answers
