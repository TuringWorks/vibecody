Feature: Hook Abort Protocol

  Scenario: Exit 0 allows execution
    Given a hook exit code of 0
    And hook stdout ""
    When I parse the hook output
    Then the hook should not be blocking

  Scenario: Exit 2 blocks execution
    Given a hook exit code of 2
    And hook stdout "blocked by policy"
    When I parse the hook output
    Then the hook should be blocking
    And the message should contain "blocked by policy"

  Scenario: JSON action block overrides exit 0
    Given a hook exit code of 0
    And hook stdout "{\"action\":\"block\",\"message\":\"safety rule triggered\"}"
    When I parse the hook output
    Then the hook should be blocking
    And the message should contain "safety rule triggered"

  Scenario: All allow outputs aggregate to allow
    Given an allow output
    And an allow output
    When I aggregate the outputs
    Then the hook should not be blocking

  Scenario: Any blocking output makes aggregate blocking
    Given an allow output
    And a blocking output with reason "denied"
    When I aggregate the outputs
    Then the hook should be blocking
    And the message should contain "denied"

  Scenario: Abort signal starts as non-aborted
    Given a new AbortSignal
    Then is_aborted should return false

  Scenario: Cloned signals share abort state
    Given a new AbortSignal with a clone
    When I abort the original signal
    Then the clone should also be aborted

  Scenario: Progress events are emitted and received on the channel
    Given a HookAbortController
    When I emit Started, Running, and Completed events for hook "my-hook"
    Then 3 events should be received on the channel
