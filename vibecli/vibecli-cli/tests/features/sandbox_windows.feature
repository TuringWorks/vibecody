Feature: Windows-style ACL sandbox policy
  The WindowsSandbox enforces path and network access using prefix-based
  allow/deny rules. Deny rules always take precedence over allow rules.

  Scenario: Allowed path is permitted
    Given a sandbox with allowed path "/workspace"
    When I check access to path "/workspace/src/lib.rs"
    Then the path verdict should be allowed

  Scenario: Denied path is blocked
    Given a sandbox with denied path "/etc"
    When I check access to path "/etc/passwd"
    Then the path verdict should be denied

  Scenario: Deny rule overrides allow rule for the same prefix
    Given a sandbox with allowed path "/workspace" and denied path "/workspace/secret"
    When I check access to path "/workspace/secret/key.pem"
    Then the path verdict should be denied

  Scenario: Network blocked but specific host allowed
    Given a sandbox with no internet and allowed host "internal.corp"
    When I check network access to "internal.corp"
    Then the network verdict should be allowed
    When I check network access to "evil.com"
    Then the network verdict should be denied
