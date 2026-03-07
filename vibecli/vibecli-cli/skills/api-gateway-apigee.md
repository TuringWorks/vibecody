---
triggers: ["Apigee", "apigee proxy", "apigee policy", "apigee edge", "apigee X", "apigee api management", "apigee developer portal"]
tools_allowed: ["read_file", "write_file", "bash"]
category: devops
---

# Apigee API Gateway

When working with Apigee API Gateway:

1. Structure API proxies with clear separation of concerns: PreFlow for auth and rate limiting, Conditional Flows for route-specific logic, PostFlow for logging and response transformation. Keep the proxy bundle in version control with `apiproxy/` directory structure.

2. Implement OAuth 2.0 with Apigee's built-in policies: use `OAuthV2` policy for token generation and `VerifyAccessToken` in the PreFlow of every secured proxy: `<OAuthV2 name="VerifyToken"><Operation>VerifyAccessToken</Operation></OAuthV2>`.

3. Apply rate limiting with `Quota` and `SpikeArrest` policies — SpikeArrest smooths traffic bursts (`<Rate>10ps</Rate>` = 10 per second), Quota enforces total calls per time window per API key: `<Allow count="1000"/><Interval>1</Interval><TimeUnit>hour</TimeUnit>`.

4. Use TargetServers for backend host configuration instead of hardcoding URLs — define servers per environment and reference them in TargetEndpoint: `<HTTPTargetConnection><LoadBalancer><Server name="backend-prod"/></LoadBalancer></HTTPTargetConnection>`.

5. Transform requests and responses with `AssignMessage` policy for header/payload manipulation and `JSONToXML`/`XMLToJSON` for format conversion. Use `ExtractVariables` with JSONPath or XPath to pull values into flow variables.

6. Implement caching with `ResponseCache` policy to reduce backend load — set cache keys based on URI, query parameters, and headers: `<CacheKey><KeyFragment ref="request.uri"/></CacheKey><ExpirySettings><TimeoutInSec>300</TimeoutInSec></ExpirySettings>`.

7. Use Shared Flows for reusable policy chains (auth, CORS, logging) — attach them via `FlowCallout` policy or as Flow Hooks that run on all proxies in an environment automatically.

8. Deploy across environments (dev, test, prod) using `apigeecli` or Apigee Maven plugin in CI/CD: `apigeecli apis deploy --name my-proxy --env prod --org $ORG --token $TOKEN --ovr`. Use environment-scoped KVMs for config.

9. Store sensitive configuration in Encrypted Key Value Maps (KVMs): `<KeyValueMapOperations name="GetConfig" mapIdentifier="secrets"><Get assignTo="private.api_key"><Key><Parameter>backend_key</Parameter></Key></Get></KeyValueMapOperations>`. Prefix variables with `private.` to prevent tracing exposure.

10. Enable Apigee Analytics and custom reports to track API performance, error rates, and developer adoption. Use `StatisticsCollector` policy to capture custom dimensions: `<Statistic name="plan_type" ref="developer.app.plan" type="string"/>`.

11. Set up the developer portal (Drupal-based or integrated) for API documentation, app registration, and key management. Publish OpenAPI specs and configure API Products that bundle proxies with quota plans.

12. Handle errors consistently with `FaultRules` and `RaiseFault` — define a default fault rule that formats all errors as JSON with status code, error code, and message: `<RaiseFault><FaultResponse><Set><StatusCode>429</StatusCode><Payload contentType="application/json">{"error":"rate_limit_exceeded"}</Payload></Set></FaultResponse></RaiseFault>`.
