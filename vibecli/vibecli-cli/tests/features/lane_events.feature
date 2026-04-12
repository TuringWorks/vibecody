Feature: Structured lane events (18 event types)
  Events are emitted as agents progress through workflow lanes,
  covering lifecycle, commits, branches, quality gates, and failures.

  Scenario: Builder creates a valid event with all required fields
    Given a LaneEventBuilder with type "lane_started" and lane "lane-1"
    When I build the event
    Then it should have a non-empty id
    And it should have event type "lane_started"

  Scenario: Builder rejects events without event type
    Given a LaneEventBuilder with lane "lane-1" but no event type
    When I try to build the event
    Then the build should fail

  Scenario: Superseded commits are deduplicated
    Given a CommitCreated event with sha "abc123"
    And a CommitSuperseded event with sha "abc123"
    And a CommitCreated event with sha "def456"
    When I deduplicate the events
    Then CommitCreated "abc123" should be removed
    And CommitCreated "def456" should remain

  Scenario: All 18 event types have distinct display strings
    Then each LaneEventType variant should have a unique display string
