Feature: Worker bootstrap six-state lifecycle
  Manages the spawning handshake for subprocess agents: detects readiness
  via visual prompts, rejects shell false-positives, validates delivery,
  and supports auto-recovery.

  Scenario: New worker starts in Spawning state
    Given a new worker "w1" with task "fix tests"
    Then its state should be "spawning"

  Scenario: Valid state transition succeeds
    Given a new worker "w1" with task "task"
    When I transition to "ready_for_prompt"
    Then the state should be "ready_for_prompt"

  Scenario: Invalid transition from terminal state fails
    Given a worker that has reached "finished" state
    When I try to transition to "running"
    Then the transition should fail with an error

  Scenario: Shell-only prompt is rejected as a false positive
    Given output line "$ "
    Then detect_readiness should return false

  Scenario: Agent prompt is detected as ready
    Given output line "> "
    Then detect_readiness should return true
