---
triggers: ["serverless", "Lambda", "cold start", "API Gateway", "event trigger", "cloud function", "edge function"]
tools_allowed: ["read_file", "write_file", "bash"]
category: devops
---

# Serverless Architecture

When building serverless applications:

1. Design functions to be stateless — state belongs in databases, caches, or queues
2. Cold starts: minimize bundle size, use provisioned concurrency for latency-sensitive paths
3. Keep functions focused: one function per event type — avoid monolithic Lambda functions
4. Use API Gateway for HTTP triggers — handles routing, auth, throttling, CORS
5. Event sources: S3 events, SQS messages, DynamoDB streams, CloudWatch schedules
6. Timeout configuration: set based on expected duration — default 3s for API, longer for background
7. Memory = CPU: increasing memory also increases CPU allocation — profile to find sweet spot
8. Use layers/extensions for shared dependencies — reduce deployment package size
9. Error handling: use DLQ for async invocations — failed events go to SQS/SNS for retry
10. Local development: use SAM CLI, Serverless Framework, or SST for local testing
11. Connection management: reuse DB connections across invocations (outside handler)
12. Cost model: pay per invocation + duration — free tier generous for low-traffic apps
