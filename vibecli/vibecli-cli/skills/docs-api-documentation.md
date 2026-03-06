---
triggers: ["OpenAPI", "Swagger", "JSDoc", "rustdoc", "typedoc", "API documentation", "openapi spec"]
tools_allowed: ["read_file", "write_file", "bash"]
category: documentation
---

# API Documentation

When documenting APIs:

1. Use OpenAPI 3.1 (Swagger) for REST APIs — define schemas, endpoints, auth, examples
2. Write descriptions for every endpoint: what it does, when to use it, side effects
3. Include request/response examples for every endpoint — real-world, not lorem ipsum
4. Document error responses: list all possible error codes with descriptions and fix guidance
5. Use `$ref` for shared schemas — DRY principle applies to documentation too
6. Rust: use `///` doc comments with `# Examples` sections — `cargo doc` builds HTML
7. TypeScript: use TSDoc (`@param`, `@returns`, `@example`) — TypeDoc generates static sites
8. JavaScript: use JSDoc for type hints + documentation in non-TypeScript projects
9. Include authentication section: which endpoints need auth, how to obtain tokens
10. Version your API docs alongside your API — docs in the same repo, same PR
11. Use tools like Redocly, Stoplight, or swagger-ui to render interactive documentation
12. Test your API examples: use Dredd or Schemathesis to verify docs match implementation
