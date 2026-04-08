Feature: FailoverProvider — multi-provider chain with health-aware ordering

  The FailoverProvider wraps multiple AI providers and automatically falls
  through to the next provider when one fails. With a ProviderHealthTracker
  attached, it prefers the healthiest provider first.

  # ── Name / construction ───────────────────────────────────────────────────────

  Scenario: Empty chain produces "Failover(empty)" name
    Given a failover chain with 0 providers
    Then the provider name should be "Failover(empty)"

  Scenario: Single-provider chain name includes the provider name
    Given a failover chain with providers '["Claude"]'
    Then the provider name should be "Failover(Claude)"

  Scenario: Two-provider chain uses arrow separator
    Given a failover chain with providers '["Claude", "Ollama"]'
    Then the provider name should be "Failover(Claude -> Ollama)"

  Scenario: Four-provider chain name lists all providers
    Given a failover chain with providers '["A", "B", "C", "D"]'
    Then the provider name should be "Failover(A -> B -> C -> D)"

  # ── is_available ──────────────────────────────────────────────────────────────

  Scenario: Empty chain is not available
    Given a failover chain with 0 providers
    Then is_available should return false

  Scenario: Chain with all unavailable providers is not available
    Given a failover chain where all providers are unavailable
    Then is_available should return false

  Scenario: Chain with one available provider is available
    Given a failover chain where only the second provider is available
    Then is_available should return true

  # ── Fallthrough on failure ────────────────────────────────────────────────────

  Scenario: Empty chain chat returns "No providers" error
    Given a failover chain with 0 providers
    When chat is called
    Then it should return an error containing "No providers"

  Scenario: First provider fails — second provider is used
    Given a failover chain where the first provider fails and the second succeeds as "Backup"
    When chat is called
    Then the response should contain "Backup"

  Scenario: First provider succeeds — second is never tried
    Given a failover chain where the first provider succeeds as "Primary" and second as "Secondary"
    When chat is called
    Then the response should contain "Primary"

  Scenario: All providers fail — last error is returned
    Given a failover chain where all providers fail with message "mock"
    When chat is called
    Then it should return an error containing "mock"

  Scenario: complete() falls through on failure
    Given a failover chain where the first provider fails and the second succeeds as "Backup"
    When complete is called
    Then the completion text should contain "Backup"

  Scenario: chat_response() falls through on failure
    Given a failover chain where the first provider fails and the second succeeds as "Backup"
    When chat_response is called
    Then the response text should contain "Backup"

  # ── Health-aware ordering ─────────────────────────────────────────────────────

  Scenario: Without health tracker, providers are tried in original order
    Given a failover chain with providers '["First", "Second"]' and no health tracker
    When both providers succeed and chat is called
    Then the response should contain "First"

  Scenario: Health tracker causes healthier provider to be tried first
    Given a failover chain with providers '["Primary", "Backup"]' and a health tracker
    And "Backup" has 5 successful calls recorded and "Primary" has 5 failed calls
    When both providers succeed and chat is called
    Then the response should contain "Backup"

  Scenario: Health tracker records outcomes for each provider call
    Given a failover chain with providers '["Failing", "Working"]' and a health tracker
    When the first provider fails and the second succeeds on a chat call
    Then the tracker should record 1 failure for "Failing"
    And the tracker should record 1 success for "Working"
