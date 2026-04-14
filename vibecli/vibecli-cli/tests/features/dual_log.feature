Feature: Dual-file session logging (append-only log + bounded context)
  log.jsonl is append-only and never compacted (infinite searchable history).
  context.jsonl is the bounded LLM context window (compacted as needed).
  Before each agent turn, sync_context() pulls new entries from log.jsonl
  into context.jsonl.  grep_log() lets the agent search history without
  enlarging the LLM context.

  Scenario: Appended entries appear in full log and are synced into context
    Given a dual-log with max context 10
    When I append a "user" entry "hello world" with id "u1" at time 1
    And I append an "assistant" entry "hello back" with id "a1" at time 2
    And I sync the context
    Then the full log count should be 2
    And the context count should be 2
    And the unsynced count should be 0

  Scenario: Compaction keeps recent entries and replaces old ones with a summary
    Given a dual-log with max context 20
    When I append 10 "user" entries starting at time 0
    And I sync the context
    And I compact with summary "first 7 summarised" keeping 3 recent entries
    Then the context count should be 4
    And the first context entry should be a compaction summary
    And the full log count should be 10

  Scenario: grep_log finds a historical entry that was evicted from context
    Given a dual-log with max context 2
    When I append a "user" entry "ancient secret" with id "h1" at time 1
    And I append a "user" entry "recent one" with id "h2" at time 2
    And I append a "user" entry "recent two" with id "h3" at time 3
    And I sync the context
    Then the context should not contain entry "h1"
    And grepping for "ancient secret" should return 1 result
    And the grep result id should be "h1"

  Scenario: Serialize and deserialize round-trip preserves all entries
    Given a dual-log with max context 10
    When I append a "system" entry "boot" with id "s0" at time 0
    And I append a "user" entry "query" with id "u0" at time 10
    And I sync the context
    And I serialize and reload the dual-log
    Then the full log count should be 2
    And the context count should be 2
    And the entry at full-log index 1 should have content "query"

  Scenario: Persist to disk and load restores the dual-log
    Given a dual-log with max context 5
    When I append a "user" entry "disk test" with id "d1" at time 100
    And I append an "assistant" entry "disk reply" with id "d2" at time 200
    And I sync the context
    And I persist the dual-log to a temporary directory
    And I load the dual-log from that temporary directory
    Then the full log count should be 2
    And the context count should be 2
    And the entry at full-log index 0 should have content "disk test"
