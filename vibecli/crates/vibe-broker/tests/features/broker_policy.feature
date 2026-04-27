Feature: Egress broker policy DSL
  Per-skill / per-agent egress.toml files compile into a Policy that the
  broker matches each request against. Default is deny; rules describe
  the host glob, methods, optional path-prefix, optional path-pattern,
  and the credential to inject.

  Scenario: empty policy denies everything
    Given an empty egress policy
    When I match a request "GET https://api.openai.com/v1/messages"
    Then the policy decision is "Deny"

  Scenario: explicit allow with host glob match
    Given a policy with one rule allowing "*.openai.com" methods "GET, POST" with bearer key "@profile.openai_key"
    When I match a request "POST https://api.openai.com/v1/messages"
    Then the policy decision is "Allow"
    And the inject type is "Bearer"

  Scenario: method not in rule list denies
    Given a policy with one rule allowing "*.openai.com" methods "GET" with bearer key "@profile.openai_key"
    When I match a request "POST https://api.openai.com/v1/messages"
    Then the policy decision is "Deny"

  Scenario: host glob does not match
    Given a policy with one rule allowing "*.openai.com" methods "GET, POST" with bearer key "@profile.openai_key"
    When I match a request "GET https://api.anthropic.com/v1/messages"
    Then the policy decision is "Deny"

  Scenario: path prefix narrows allowed routes
    Given a policy with one rule allowing "api.github.com" methods "GET" with path prefix "/repos/me/myrepo/" and bearer key "@workspace.github_token"
    When I match a request "GET https://api.github.com/repos/me/myrepo/issues"
    Then the policy decision is "Allow"
    When I match a request "GET https://api.github.com/repos/other/other/issues"
    Then the policy decision is "Deny"

  Scenario: TOML round-trip
    Given a sample policy TOML with one rule for "api.example.com"
    When I parse the TOML into a Policy
    Then the policy has 1 rules
    And the first rule host glob is "api.example.com"

  Scenario: AWS SigV4 inject parses
    Given a TOML rule with inject type "aws-sigv4" profile "@workspace.aws_default"
    When I parse the rule
    Then the parsed inject type is "AwsSigV4"
