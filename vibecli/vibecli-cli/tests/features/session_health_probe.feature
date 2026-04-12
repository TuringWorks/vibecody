Feature: Post-compaction session health probe
  After auto-compacting a long conversation, a lightweight probe checks
  whether the tool executor is still responsive before the agent loop
  resumes. An unresponsive executor flags the session as Degraded.

  Scenario: Healthy tool executor keeps session in Progress state
    Given a responsive tool executor
    When I run the health probe
    Then the result should be Healthy
    And the mapped health state should be "PROGRESS"

  Scenario: Failed tool executor flags session as Degraded
    Given an unresponsive tool executor
    When I run the health probe
    Then the result should be Failed
    And the mapped health state should be "DEGRADED"

  Scenario: Probe fires only after significant compaction
    Given a probe with compaction threshold 50
    When 60 messages were compacted
    Then should_probe_after_compaction should return true
    When 10 messages were compacted
    Then should_probe_after_compaction should return false
