---
triggers: ["GraphQL", "schema SDL", "resolver", "mutation", "subscription", "N+1 graphql", "apollo"]
tools_allowed: ["read_file", "write_file", "bash"]
category: api
---

# GraphQL API Design

When building GraphQL APIs:

1. Design schema-first: define types in SDL, then implement resolvers
2. Use `Query` for reads, `Mutation` for writes, `Subscription` for real-time events
3. Solve N+1 with DataLoader — batch and cache database lookups per request
4. Use `input` types for mutation arguments: `createUser(input: CreateUserInput!): User!`
5. Return union types for errors: `union CreateUserResult = User | ValidationError`
6. Use `connection` pattern for pagination: `edges { node cursor }` + `pageInfo { hasNextPage }`
7. Implement query depth limiting and complexity analysis — prevent abuse
8. Use fragments for shared fields: `fragment UserFields on User { id name email }`
9. Version via schema evolution (deprecation) — not URL versioning (`/v1/graphql`)
10. Use `@deprecated(reason: "Use newField")` directive for backward-compatible changes
11. Implement proper `null` semantics: non-null (`String!`) for required, nullable for optional
12. Use code generation (`graphql-codegen`, `juniper`, `async-graphql`) for type-safe resolvers
