Feature: MockAIProvider for deterministic CI testing
  A reusable mock AIProvider with sequenced responses and scenario-based
  prefix matching for reliable offline testing.

  Scenario: Sequenced responses are returned in order
    Given a mock provider with responses "hello" and "world"
    When I call chat twice
    Then the first response should be "hello"
    And the second response should be "world"

  Scenario: Provider reports unavailable when configured
    Given a mock provider configured as unavailable
    Then is_available should return false

  Scenario: Call count is tracked across invocations
    Given a mock provider with 3 responses
    When I call chat 3 times
    Then call_count should be 3

  Scenario: Scenario prefix matching returns deterministic response
    Given a mock provider with scenario prefix "fix" returning "I fixed the bug"
    When I call chat with message "fix the failing test"
    Then the response should be "I fixed the bug"
