---
triggers: ["AWS CDK", "cdk", "cdk construct", "cdk stack", "cdk pipeline", "cdk deploy", "aws infrastructure as code"]
tools_allowed: ["read_file", "write_file", "bash"]
requires_bins: ["cdk"]
category: cloud-aws
---

# AWS CDK Infrastructure as Code

When working with AWS CDK:

1. Use L2 constructs (e.g., `s3.Bucket`, `lambda.Function`) over L1 (`CfnBucket`) for sensible defaults and helper methods; drop to L1 only when L2 does not expose a property, using `node.defaultChild` and `addPropertyOverride` to patch specific CloudFormation fields.
2. Organize stacks by lifecycle and ownership: separate stateful resources (databases, S3 buckets) from stateless ones (Lambda, API Gateway) so that deployments of compute stacks cannot accidentally destroy data stores.
3. Use CDK Pipelines (`pipelines.CodePipeline`) for self-mutating CI/CD: define `ShellStep` for build/test, add stages with `addStage()`, and the pipeline updates itself when you push CDK changes, eliminating manual `cdk deploy`.
4. Write assertion tests with `assertions.Template.fromStack(stack)`: use `hasResourceProperties` to verify critical config, `resourceCountIs` for resource counts, and `Match.objectLike` for partial matching of nested properties.
5. Apply `cdk.Aspects` for cross-cutting concerns: enforce tagging (`Tags.of(scope).add()`), check that all S3 buckets have encryption enabled, or validate that no security groups allow `0.0.0.0/0` ingress by implementing `IAspect.visit()`.
6. Use `CfnOutput` and `ssm.StringParameter` to share values across stacks; avoid cross-stack references with `Fn.importValue` for frequently updated stacks as it creates hard CloudFormation dependencies that block updates.
7. Define custom constructs as reusable L3 patterns (`class SecureApi extends Construct`) that bundle an API Gateway, Lambda, WAF, and logging with opinionated defaults; publish to a private npm/PyPI registry for cross-team reuse.
8. Use `cdk.RemovalPolicy.RETAIN` on stateful resources (RDS, DynamoDB, S3) and `DESTROY` only in dev stacks gated by a context value (`this.node.tryGetContext('env') === 'dev'`); this prevents accidental data loss on `cdk destroy`.
9. Pass configuration via context values (`-c key=value` or `cdk.json`) and environment variables, not hardcoded strings; use `Stack.of(this).region` and `Stack.of(this).account` for region/account-aware logic.
10. Use `BundlingOptions` with Docker or esbuild for Lambda asset bundling (`NodejsFunction` auto-bundles with esbuild); set `minify: true`, `sourcemap: true`, and `externalModules` to exclude AWS SDK v3 (already in Lambda runtime).
11. Implement custom resources with `cr.AwsCustomResource` for simple SDK calls or `CustomResource` with a Lambda provider for complex provisioning logic; always handle `Create`, `Update`, and `Delete` events and return `PhysicalResourceId` consistently.
12. Run `cdk diff` before every deploy to review changes; enable CloudFormation stack termination protection on production stacks, and use `--require-approval broadening` to gate any IAM or security group changes on manual approval.
