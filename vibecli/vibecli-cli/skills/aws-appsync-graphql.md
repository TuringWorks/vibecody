---
triggers: ["AppSync", "aws appsync", "aws graphql", "appsync resolver", "appsync subscription", "vtl template"]
tools_allowed: ["read_file", "write_file", "bash"]
requires_bins: ["aws"]
category: cloud-aws
---

# AWS AppSync GraphQL API

When working with AWS AppSync:

1. Use the JavaScript (APPSYNC_JS) resolver runtime over VTL for new resolvers; write request/response handlers as ES modules (`export function request(ctx)` / `export function response(ctx)`) with full access to `ctx.args`, `ctx.stash`, and `util` functions.
2. Define pipeline resolvers for multi-step operations: chain functions (e.g., authorize -> validate -> write -> audit) where each function's result passes to the next via `ctx.prev.result` and `ctx.stash` for shared state.
3. Connect DynamoDB datasources using built-in operations: `util.dynamodb.toMapValues()` for `PutItem`, `util.transform.toDynamoDBConditionExpression()` for conditional writes; use `BatchGetItem` and `BatchDeleteItem` operations for bulk access patterns.
4. Implement real-time subscriptions by annotating schema fields with `@aws_subscribe(mutations: ["createMessage"])` and returning the mutation result; subscriptions filter client-side via arguments, so use enhanced subscription filtering for server-side filtering on up to 5 fields.
5. Configure multiple auth modes on a single API: use `API_KEY` for public read access, `COGNITO_USER_POOLS` for authenticated mutations, `AWS_IAM` for service-to-service, and `OPENID_CONNECT` for third-party IdPs; annotate types with `@aws_cognito_user_pools` or `@aws_iam`.
6. Enable caching at the resolver level (`cachingConfig: {ttl: 300, cachingKeys: ["$context.arguments.id"]}`) or full API-level caching; use `$context.identity.sub` in caching keys for per-user caches and invalidate with `aws appsync flush-api-cache`.
7. Use HTTP datasources to integrate REST APIs and Lambda datasources for complex business logic; set `ServiceRoleArn` on each datasource with least-privilege IAM policies scoped to specific DynamoDB tables or Lambda functions.
8. Implement field-level authorization with `@aws_auth` directives and resolver-level checks: inspect `ctx.identity.groups` (Cognito), `ctx.identity.accountId` (IAM), or custom claims to conditionally return data or `util.unauthorized()`.
9. Use `util.error()` in resolvers to return typed GraphQL errors with `errorType` and `errorInfo` fields; structure errors consistently so clients can programmatically handle `NOT_FOUND`, `UNAUTHORIZED`, and `VALIDATION_ERROR` types.
10. Connect to Aurora Serverless via the RDS Data API datasource for relational queries; use `util.transform.toElasticsearchQueryDSL()` for OpenSearch datasources to translate GraphQL filter arguments to search queries.
11. Use merged APIs to combine multiple AppSync source APIs (owned by different teams) into a single endpoint; manage schema conflicts with `@aws_api_key` scoping and configure source API associations with auto-merge on schema updates.
12. Test resolvers locally with the AppSync resolver evaluation API (`aws appsync evaluate-mapping-template`) or the Amplify CLI's mock server; write unit tests that pass context objects to resolver functions and assert the returned DynamoDB/HTTP request objects.
