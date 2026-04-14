Feature: Watch session relay — compact payload generation and replay prevention
  The relay module transforms full session models into Watch-optimised compact
  representations capped at OLED display constraints (184×224pt).
  The NonceRegistry prevents replay attacks on dispatch requests.

  Scenario: Short strings pass through truncate unchanged
    Given a string "hello" of length 5
    When I truncate it to 10 characters
    Then the result should equal "hello"

  Scenario: Long strings are truncated with ellipsis
    Given a string of 100 repeated "a" characters
    When I truncate it to 10 characters
    Then the result should be exactly 10 characters
    And the result should end with "…"

  Scenario: Exact-length strings are not truncated
    Given a string "hello" of length 5
    When I truncate it to 5 characters
    Then the result should equal "hello"

  Scenario: SSE delta event maps to Watch delta kind
    Given an SSE payload with type "token_delta" and text "Hello"
    When I convert it to a WatchAgentEvent
    Then the event kind should be "delta"
    And the event delta should be "Hello"

  Scenario: Tool start event carries tool name and step
    Given an SSE payload with type "tool_start" and name "bash" and step 3
    When I convert it to a WatchAgentEvent
    Then the event kind should be "tool_start"
    And the event tool should be "bash"
    And the event step should be 3

  Scenario: Tool end success maps status to ok
    Given an SSE payload with type "tool_end" and name "read_file" and success true
    When I convert it to a WatchAgentEvent
    Then the event kind should be "tool_end"
    And the event status should be "ok"

  Scenario: Tool end failure maps status to err
    Given an SSE payload with type "tool_end" and name "write" and success false
    When I convert it to a WatchAgentEvent
    Then the event status should be "err"

  Scenario: Done event captures final status
    Given an SSE payload with type "done" and status "complete"
    When I convert it to a WatchAgentEvent
    Then the event kind should be "done"
    And the event status should be "complete"

  Scenario: Error event message is truncated to 200 characters
    Given an SSE payload with type "error" and a 300-character message
    When I convert it to a WatchAgentEvent
    Then the event kind should be "error"
    And the event error should be at most 200 characters
    And the event error should end with "…"

  Scenario: Unknown event type defaults to info
    Given an SSE payload with type "something_unknown"
    When I convert it to a WatchAgentEvent
    Then the event kind should be "info"

  Scenario: Replay nonce is rejected within 30 seconds
    Given a NonceRegistry
    And the current timestamp
    When I record nonce "watch-nonce-A"
    Then recording the same nonce again should fail with "replay"

  Scenario: Different nonces at the same timestamp are all accepted
    Given a NonceRegistry
    And the current timestamp
    When I record nonces "n1", "n2", and "n3"
    Then all three should be accepted

  Scenario: Stale timestamp (>30s old) is rejected
    Given a NonceRegistry
    When I record a nonce with a timestamp 60 seconds in the past
    Then it should fail with "timestamp"

  Scenario: Session summary uses last assistant message as preview
    Given a session with 3 messages: assistant "hi there" and second user "more"
    When I compute the WatchSessionSummary
    Then the last_message_preview should be "hi there"
    And the message_count should be 3

  Scenario: Message content is capped at 512 characters
    Given a message row with 600 characters of content
    When I convert it to a WatchMessage
    Then the content should be at most 512 characters
    And the content should end with "…"
