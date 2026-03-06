---
triggers: ["AWS", "Lambda", "S3", "DynamoDB", "ECS", "IAM", "CloudFormation", "API Gateway"]
tools_allowed: ["read_file", "write_file", "bash"]
requires_bins: ["aws"]
category: devops
---

# AWS Cloud Services

When building on AWS:

1. Use IAM roles (not access keys) for service-to-service auth — least privilege principle
2. Lambda: keep functions small, use layers for shared deps, set memory based on profiling
3. S3: enable versioning for important buckets, use lifecycle rules for cost management
4. DynamoDB: design single-table with GSIs for access patterns — think access patterns first
5. ECS/Fargate: use task definitions for container config, service for scaling/load balancing
6. API Gateway: use REST API for full features, HTTP API for simpler/cheaper Lambda proxies
7. Use VPC for network isolation — private subnets for databases, public for load balancers
8. Use Secrets Manager or SSM Parameter Store for configuration — not environment variables for secrets
9. CloudWatch: set up alarms on Lambda errors, API latency, queue depth
10. Use SQS for async processing — DLQ (Dead Letter Queue) for failed messages
11. Cost optimization: use Savings Plans for steady workloads, Spot Instances for batch/fault-tolerant
12. Use CDK or Terraform over CloudFormation console — infrastructure as code always
