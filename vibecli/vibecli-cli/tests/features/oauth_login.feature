Feature: OAuth login — subscription-based provider authentication
  VibeCody supports OAuth-based login for AI providers so that users with
  existing subscriptions (Claude Pro/Max, GitHub Copilot, Gemini CLI,
  ChatGPT Plus/Pro) can authenticate without supplying a raw API key.

  Scenario: Fresh credentials are not expired
    Given a fresh OAuth token for provider "anthropic_claude" that expires in 3600 seconds
    When I check whether the token is expired
    Then the token should not be expired
    And the valid token should equal "tok_fresh_abc"

  Scenario: Expired credentials are detected
    Given an expired OAuth token for provider "github_copilot"
    When I check whether the token is expired
    Then the token should be expired
    And no valid token should be returned

  Scenario: Logged-in check returns only providers with valid credentials
    Given a fresh OAuth token for provider "anthropic_claude" that expires in 3600 seconds
    And an expired OAuth token for provider "github_copilot"
    When I list providers that are logged in
    Then the logged-in list should contain "anthropic_claude"
    And the logged-in list should not contain "github_copilot"

  Scenario: Device flow simulation stores credentials and fires callbacks
    Given an empty OAuth manager
    When I simulate a device flow for provider "github_copilot" with mock token "mock_tok_xyz"
    Then the flow result should be success
    And provider "github_copilot" should be logged in
    And the valid token for provider "github_copilot" should equal "mock_tok_xyz"

  Scenario: Auth header prefers OAuth token over API key fallback
    Given a fresh OAuth token for provider "anthropic_claude" that expires in 3600 seconds
    When I build the auth header for provider "anthropic_claude" with fallback key "sk-fallback"
    Then the auth header should equal "Bearer tok_fresh_abc"

  Scenario: Auth header falls back to API key when no OAuth token is stored
    Given an empty OAuth manager
    When I build the auth header for provider "openai_codex" with fallback key "sk-apikey"
    Then the auth header should equal "Bearer sk-apikey"
