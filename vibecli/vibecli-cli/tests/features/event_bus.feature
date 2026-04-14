Feature: Typed in-process lifecycle event bus
  The EventBus delivers BusEvents to matching subscribers in priority order,
  supports blocking handlers for ToolCall and BeforeProviderRequest events,
  and retains a bounded history of emitted events.

  Scenario: Subscribe and emit — handler receives the event
    Given a fresh EventBus
    And I subscribe with filter "All" at priority 0
    When I emit a "agent_start" event with turn 1
    Then the subscriber should have received 1 event
    And the received event type should be "agent_start"

  Scenario: Unsubscribe stops further delivery
    Given a fresh EventBus
    And I subscribe with filter "All" at priority 0
    When I emit a "user_input" event
    And I unsubscribe the last subscription
    And I emit a "user_input" event
    Then the subscriber should have received 1 event

  Scenario: Blocking handler vetoes a ToolCall event
    Given a fresh EventBus
    And I subscribe a blocking handler with reason "sandbox policy" at priority 0
    When I emit a "tool_call" event for tool "Bash"
    Then the emit result should be blocked with reason "sandbox policy"

  Scenario: Filter by type delivers only matching events
    Given a fresh EventBus
    And I subscribe with filter "ByType:tool_call" at priority 0
    When I emit a "agent_start" event with turn 1
    And I emit a "tool_call" event for tool "Edit"
    And I emit a "agent_end" event with turn 1
    Then the subscriber should have received 1 event
    And the received event type should be "tool_call"

  Scenario: Filter by prefix delivers all events with that prefix
    Given a fresh EventBus
    And I subscribe with filter "ByPrefix:memory_" at priority 0
    When I emit a "memory_write" event with key "ctx"
    And I emit a "memory_read" event with key "ctx"
    And I emit a "memory_delete" event with key "ctx"
    And I emit a "user_input" event
    Then the subscriber should have received 3 events

  Scenario: Priority ordering — higher priority handler runs first
    Given a fresh EventBus
    And I subscribe a priority-recording handler at priority 10
    And I subscribe a priority-recording handler at priority 0
    And I subscribe a priority-recording handler at priority 5
    When I emit a "user_input" event
    Then the priority execution order should be "10,5,0"
