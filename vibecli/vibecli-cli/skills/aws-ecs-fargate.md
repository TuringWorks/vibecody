---
triggers: ["ECS", "Fargate", "aws ecs", "ecs task", "ecs service", "fargate spot", "ecs exec", "aws container"]
tools_allowed: ["read_file", "write_file", "bash"]
requires_bins: ["aws"]
category: cloud-aws
---

# AWS ECS/Fargate Container Orchestration

When working with ECS and Fargate:

1. Define task definitions with explicit CPU and memory limits at both the task level (`cpu: "512"`, `memory: "1024"`) and container level; Fargate enforces valid combinations, so reference the supported configurations table to avoid deployment failures.
2. Use `awsvpc` network mode (required for Fargate) and assign security groups per task; restrict ingress to the ALB security group only and use VPC endpoints for ECR, S3, and CloudWatch to avoid NAT gateway data costs.
3. Configure ALB target groups with `targetType: "ip"` for Fargate tasks; set health check path, interval (15s), healthy threshold (2), and deregistration delay (30s) to enable fast rolling deployments.
4. Enable ECS Service Connect or Cloud Map service discovery for service-to-service communication using DNS names (`http://api.prod.local:8080`) instead of hardcoded load balancer URLs.
5. Use capacity provider strategies with `FARGATE` as base and `FARGATE_SPOT` for burst capacity (`{"capacityProvider": "FARGATE_SPOT", "weight": 3}`) to reduce compute costs by up to 70% for fault-tolerant workloads.
6. Enable ECS Exec (`aws ecs execute-command --interactive --command "/bin/sh"`) for debugging running containers; requires `enableExecuteCommand: true` in the service and SSM agent sidecar (auto-injected by Fargate platform 1.4+).
7. Configure deployment circuit breakers (`deploymentCircuitBreaker: {enable: true, rollback: true}`) to automatically roll back failed deployments when tasks cannot reach a healthy state within the threshold.
8. Use task IAM roles (`taskRoleArn`) for application-level AWS API access and execution roles (`executionRoleArn`) only for ECR pull and CloudWatch Logs; never share roles across services.
9. Send container logs to CloudWatch Logs with the `awslogs` driver; set `awslogs-multiline-pattern` for stack traces and use `awslogs-create-group: "true"` to auto-create log groups with a retention policy.
10. Implement graceful shutdown by handling `SIGTERM` in your application with a 30-second drain period; set `stopTimeout` in the container definition to match and configure the ALB deregistration delay accordingly.
11. Use ECS scheduled tasks (`aws events put-rule --schedule-expression "rate(1 hour)"`) for cron jobs instead of running always-on containers; Fargate tasks spin up, execute, and terminate, paying only for execution time.
12. Store secrets in AWS Secrets Manager or SSM Parameter Store and reference them in task definitions via `secrets` field (`valueFrom: "arn:aws:secretsmanager:..."`) instead of environment variables to avoid exposing sensitive values in the console or API responses.
