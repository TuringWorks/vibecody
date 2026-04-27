Feature: AWS SigV4 credential injection (slice B2.2)
  When the matched policy rule has Inject::AwsSigV4, the broker resolves
  the SecretRef into AWS credentials, signs the decrypted request, and
  forwards it with Authorization + X-Amz-Date headers attached. The
  sandbox never sees the access key or secret.

  Scenario: SigV4 injection adds Authorization + X-Amz-Date on outbound
    Given a self-signed HTTPS upstream that echoes its received headers
    And the broker holds AWS credentials at "@workspace.aws_default" — region "us-east-1" service "s3"
    And a policy that allows the upstream on CONNECT and SigV4-injects "@workspace.aws_default" on GET
    When the client performs CONNECT through the broker, then GET on root over TLS
    Then the upstream observed Authorization starting with "AWS4-HMAC-SHA256 Credential=AKIAIOSFODNN7EXAMPLE/"
    And the upstream observed an X-Amz-Date header
    And the upstream Authorization includes "SignedHeaders="
    And the upstream Authorization includes "Signature="
