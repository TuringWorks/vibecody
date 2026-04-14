Feature: Parallel tool executor
  After an assistant turn ends, tool calls are preflighted sequentially then
  dispatched concurrently. Results are always returned in the original call
  order regardless of completion order.

  Scenario: All allowed calls execute in parallel
    Given a parallel dispatcher with concurrency 4
    And I stage a tool call 'c1' for tool 'Read' sleeping 30 ms
    And I stage a tool call 'c2' for tool 'Write' sleeping 30 ms
    And I stage a tool call 'c3' for tool 'Bash' sleeping 30 ms
    When I dispatch all calls with allow-all preflight
    Then all 3 results are present
    And no result is blocked
    And the wall time is less than 80 ms

  Scenario: One call is blocked by preflight
    Given a parallel dispatcher with concurrency 4
    And I stage a tool call 'c1' for tool 'Read' sleeping 10 ms
    And I stage a tool call 'c2' for tool 'Bash' sleeping 10 ms
    And I stage a tool call 'c3' for tool 'Write' sleeping 10 ms
    When I dispatch with 'Bash' blocked by preflight
    Then all 3 results are present
    And result for call 'c2' is blocked
    And result for call 'c1' is not blocked
    And result for call 'c3' is not blocked
    And result for call 'c2' has output ''

  Scenario: Results are returned in original call order
    Given a parallel dispatcher with concurrency 3
    And I stage a tool call 'slow' for tool 'SlowTool' sleeping 60 ms
    And I stage a tool call 'med' for tool 'MedTool' sleeping 30 ms
    And I stage a tool call 'fast' for tool 'FastTool' sleeping 5 ms
    When I dispatch all calls with allow-all preflight
    Then the result order matches 'slow,med,fast'

  Scenario: Sequential fallback mode executes calls one at a time
    Given a sequential dispatcher
    And I stage a tool call 's1' for tool 'Read' sleeping 10 ms
    And I stage a tool call 's2' for tool 'Write' sleeping 10 ms
    When I dispatch all calls with allow-all preflight
    Then all 2 results are present
    And no result is blocked
    And the dispatcher mode is Sequential

  Scenario: Empty call list returns empty results
    Given a parallel dispatcher with concurrency 4
    When I dispatch an empty call list
    Then the result list is empty
