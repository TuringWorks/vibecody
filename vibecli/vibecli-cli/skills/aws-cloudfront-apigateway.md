---
triggers: ["CloudFront", "API Gateway", "aws cloudfront", "aws api gateway", "lambda@edge", "cloudfront functions", "api gateway authorizer", "usage plan"]
tools_allowed: ["read_file", "write_file", "bash"]
requires_bins: ["aws"]
category: cloud-aws
---

# AWS CloudFront CDN and API Gateway

When working with CloudFront and API Gateway:

1. Use CloudFront Functions for lightweight request/response transformations (URL rewrites, header manipulation, redirects) at sub-millisecond latency; use Lambda@Edge only when you need network access, longer execution (up to 30s), or request body access.
2. Configure cache behaviors with path patterns (`/api/*`, `/static/*`) routing to different origins; set `CachePolicyId` for managed policies (`CachingOptimized` for static assets, `CachingDisabled` for APIs) and `OriginRequestPolicyId` to forward specific headers/cookies.
3. Use origin access control (OAC) instead of origin access identity (OAI) for S3 origins; OAC supports SSE-KMS, S3 Object Lambda, and all S3 features with a simpler bucket policy: `"Condition": {"StringEquals": {"AWS:SourceArn": "arn:aws:cloudfront::ACCOUNT:distribution/DIST_ID"}}`.
4. Choose HTTP API (`httpapi`) over REST API for new projects: it is 70% cheaper, supports JWT authorizers natively, auto-deploys, and has lower latency; use REST API only when you need request validation, WAF integration, usage plans, or API keys.
5. Implement Lambda authorizers that cache authorization results: return a policy document with `Resource: "arn:aws:execute-api:*:*:API_ID/*"` (wildcard) and set `authorizerResultTtlInSeconds: 300` to avoid invoking the authorizer on every request.
6. Configure usage plans and API keys for rate limiting external consumers: set `throttle.rateLimit` and `throttle.burstLimit` per plan, and `quota.limit` for monthly request caps; return `429` responses to clients exceeding limits.
7. Use custom domain names with ACM certificates (must be in `us-east-1` for CloudFront, regional for API Gateway); configure Route 53 alias records pointing to the CloudFront distribution or API Gateway domain for zero-TTL DNS resolution.
8. Attach AWS WAF WebACLs to CloudFront distributions for bot protection, IP filtering, rate-based rules, and managed rule groups (AWSManagedRulesCommonRuleSet, SQLiRuleSet); WAF on CloudFront inspects requests before they reach any origin.
9. Enable CloudFront access logging to S3 and API Gateway execution logging to CloudWatch; use `$context.requestId`, `$context.integrationLatency`, and `$context.error.message` in API Gateway log format for debugging integration issues.
10. Use CloudFront signed URLs or signed cookies for private content distribution; generate signatures with a trusted key group (RSA 2048-bit), set expiry to minutes not hours, and restrict the IP range with the `IpAddress` condition in the policy.
11. Configure API Gateway request/response mapping templates to transform between client-facing and backend schemas; use VTL templates for REST API (`$input.json('$.data')`) or Lambda response formatting for HTTP API to decouple frontend contracts from backend models.
12. Implement cache invalidation strategically: use versioned file names (`app.abc123.js`) for static assets to avoid invalidation costs, and `CreateInvalidation` with specific paths (`/api/config`) only for critical updates; wildcard invalidations (`/*`) count as one path but clear the entire cache.
