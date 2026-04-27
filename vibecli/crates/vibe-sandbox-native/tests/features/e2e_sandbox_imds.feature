Feature: End-to-end sandbox + IMDS via AWS_EC2_METADATA_SERVICE_ENDPOINT
  AWS SDKs walk a credential chain ending at IMDS at 169.254.169.254
  unless AWS_EC2_METADATA_SERVICE_ENDPOINT is set. We set that env var
  in the sandbox at spawn time, pointing at the broker's loopback IMDS
  port. The macOS .sb profile must allow outbound TCP to that port for
  the SDK call to succeed.

  Scenario: sandboxed curl through env-redirected IMDS gets role creds
    Given a fresh macOS sandbox with a bound rw temp dir
    And a running IMDS faker on a loopback address with role "vibe-broker-role"
    And the sandbox profile permits outbound TCP to the IMDS port
    And the sandbox env exposes AWS_EC2_METADATA_SERVICE_ENDPOINT pointing at the IMDS faker
    When the sandbox runs the AWS IMDSv2 dance via curl
    Then the captured response body contains "AccessKeyId"
    And the captured response body contains "AKIAIOSFODNN7EXAMPLE"
