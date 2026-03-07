---
triggers: ["AWS Lambda", "lambda function", "serverless framework", "SAM template", "lambda layers", "cold start", "lambda@edge"]
tools_allowed: ["read_file", "write_file", "bash"]
requires_bins: ["aws"]
category: devops
---

# AWS Lambda and Serverless Functions

When working with AWS Lambda and serverless functions:

1. Minimize cold starts by keeping deployment packages small (under 50MB), using provisioned concurrency for latency-sensitive functions, and choosing lightweight runtimes (Node.js, Python) over JVM-based ones.

2. Structure handlers to separate initialization from invocation — put SDK client creation and config loading outside the handler function so they are reused across warm invocations: `const db = new DynamoDB(); exports.handler = async (event) => { /* use db */ }`.

3. Define infrastructure with SAM templates for AWS-native workflows: `Type: AWS::Serverless::Function` with `Events`, `Policies`, and `Environment` — run `sam local invoke` and `sam local start-api` for local testing.

4. Use Lambda Layers to share common dependencies across functions — package shared libraries into a layer (`aws lambda publish-layer-version --layer-name common-deps --zip-file fileb://layer.zip`) and reference by ARN in each function.

5. Set memory proportionally — Lambda allocates CPU linearly with memory. Benchmark with `aws-lambda-power-tuning` to find the cost-optimal memory setting; often 512MB-1024MB is the sweet spot for compute-bound tasks.

6. Implement idempotency for all event-driven functions — use DynamoDB conditional writes or a dedicated idempotency key table to prevent duplicate processing when Lambda retries on failure.

7. Configure dead-letter queues (SQS or SNS) for async invocations to capture failed events: `DeadLetterConfig: {TargetArn: !GetAtt DLQ.Arn}`. Monitor DLQ depth with CloudWatch alarms.

8. Use environment variables for configuration and AWS Secrets Manager or Parameter Store for secrets — never hardcode credentials. Reference SSM parameters in SAM with `{{resolve:ssm:/app/db-url}}`.

9. Set appropriate timeout values (default 3s is too low for most workloads) and match them with downstream client timeouts. API Gateway has a hard 29-second limit for synchronous invocations.

10. Deploy Lambda@Edge for latency-critical request/response transformations at CloudFront — use for header manipulation, A/B testing, auth token validation. Keep packages under 1MB and execution under 5 seconds.

11. Use function URLs or API Gateway REST/HTTP APIs depending on needs — HTTP API is cheaper and faster for simple proxying; REST API provides request validation, WAF integration, and usage plans.

12. Monitor with structured JSON logging (not `console.log` strings), X-Ray tracing for distributed call graphs, and CloudWatch Insights queries: `fields @timestamp, @message | filter @message like /ERROR/ | sort @timestamp desc | limit 20`.
