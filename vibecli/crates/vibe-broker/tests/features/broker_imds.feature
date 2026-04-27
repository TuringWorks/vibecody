Feature: IMDSv2 faker (slice B3)
  AWS SDKs default to walking the credential chain ending at IMDS at
  169.254.169.254. Without something there to answer, requests time out
  and the SDK returns "could not load credentials" — even though the
  broker has the real credentials. The IMDS faker is what closes that
  loop: it answers IMDSv2 requests on a configured loopback address,
  returns role-shaped credentials derived from the broker's SecretStore.

  Scenario: token endpoint hands out a synthetic IMDSv2 token
    Given an IMDS faker bound to a loopback address
    When I PUT "/latest/api/token" with header "x-aws-ec2-metadata-token-ttl-seconds: 21600"
    Then the IMDS response status is 200
    And the IMDS response body is non-empty

  Scenario: role list endpoint returns the configured role name
    Given an IMDS faker bound to a loopback address with role "vibe-broker-role" and creds at "@workspace.aws_default"
    And I have an IMDS token from the faker
    When I GET "/latest/meta-data/iam/security-credentials/" with the IMDS token
    Then the IMDS response status is 200
    And the IMDS response body equals "vibe-broker-role"

  Scenario: role credentials endpoint returns AWS-shaped JSON
    Given an IMDS faker bound to a loopback address with role "vibe-broker-role" and creds at "@workspace.aws_default"
    And I have an IMDS token from the faker
    When I GET "/latest/meta-data/iam/security-credentials/vibe-broker-role" with the IMDS token
    Then the IMDS response status is 200
    And the IMDS response body contains "AccessKeyId"
    And the IMDS response body contains "SecretAccessKey"
    And the IMDS response body contains "Expiration"
    And the IMDS response body contains "AKIAIOSFODNN7EXAMPLE"

  Scenario: requests without an IMDS token are rejected
    Given an IMDS faker bound to a loopback address with role "vibe-broker-role" and creds at "@workspace.aws_default"
    When I GET "/latest/meta-data/iam/security-credentials/" without an IMDS token
    Then the IMDS response status is 401
