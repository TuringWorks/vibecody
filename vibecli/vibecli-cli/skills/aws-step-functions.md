---
triggers: ["Step Functions", "step functions", "aws step", "state machine", "ASL", "express workflow", "step functions map"]
tools_allowed: ["read_file", "write_file", "bash"]
requires_bins: ["aws"]
category: cloud-aws
---

# AWS Step Functions Orchestration

When working with AWS Step Functions:

1. Choose Standard workflows for long-running processes (up to 1 year, exactly-once execution) and Express workflows for high-volume, short-duration tasks (up to 5 minutes, at-least-once); Express costs 10-20x less per execution for event-driven workloads.
2. Use SDK integrations (`.sync` suffix) for synchronous service calls (`arn:aws:states:::lambda:invoke`, `arn:aws:states:::sqs:sendMessage`) so Step Functions waits for completion and captures the result without polling Lambda.
3. Implement error handling with `Catch` and `Retry` on each task state: define `Retry` with `ErrorEquals: ["States.TaskFailed"]`, `IntervalSeconds: 2`, `MaxAttempts: 3`, `BackoffRate: 2.0`, and `Catch` to route to a failure-handling state.
4. Use `Choice` states for branching logic with comparison operators (`StringEquals`, `NumericGreaterThan`, `IsPresent`); always include a `Default` branch to handle unexpected values and avoid stuck executions.
5. Use `Parallel` states to execute independent branches concurrently and aggregate results; each branch's output becomes an array element, so use a `Pass` state with `ResultSelector` to reshape the combined output.
6. Use distributed `Map` state for large-scale parallel processing (up to 10,000 concurrent child executions) with S3 as the item source; set `MaxConcurrency` to control throughput and `ToleratedFailurePercentage` to allow partial failures.
7. Apply `InputPath`, `Parameters`, `ResultPath`, and `OutputPath` to control data flow: use `Parameters` with `.$` suffix for JSONPath references (`"orderId.$": "$.detail.id"`) and `ResultPath: "$.taskResult"` to merge task output into the existing state.
8. Use `ResultSelector` to extract and rename fields from service responses before they enter `ResultPath`, reducing state payload size and simplifying downstream state logic.
9. Implement the callback pattern with `waitForTaskToken`: pass the token to an external system (SQS, human approval), then call `SendTaskSuccess` or `SendTaskFailure` to resume the workflow with the result.
10. Use Activity tasks for work executed by external workers (on-prem servers, ECS tasks) that poll with `GetActivityTask`, process the work, and report back with `SendTaskSuccess`; set `HeartbeatSeconds` to detect stalled workers.
11. Integrate with CDK using `sfn.StateMachine` and chain states with `.next()`: `const definition = submitOrder.next(new sfn.Choice(this, 'CheckStatus').when(sfn.Condition.stringEquals('$.status', 'OK'), processPayment).otherwise(handleError))`.
12. Enable CloudWatch Logs for Express workflows (`LoggingConfiguration: {level: "ALL", destinations: [logGroup]}`) and X-Ray tracing for Standard workflows to debug execution paths; use `aws stepfunctions get-execution-history` to inspect individual state transitions.
