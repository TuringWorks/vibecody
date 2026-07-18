Feature: CircuitBreaker state transitions
  The CircuitBreaker monitors agent health and trips into STALLED, SPINNING,
  DEGRADED, or BLOCKED when it detects the agent is stuck.

  Background:
    Given a fresh CircuitBreaker with default thresholds

  # ── Stall detection ──────────────────────────────────────────────────────────

  Scenario: Default state is Progress
    Then the health state should be "PROGRESS"

  Scenario: Think calls increment the stall counter
    When I record 3 Think steps with success
    Then the stall counter should be 3

  Scenario: A productive file write resets the stall counter
    When I record 2 Think steps with success
    And I record 1 WriteFile step with success
    Then the stall counter should be 0
    And the health state should be "PROGRESS"

  Scenario: Stall is detected at threshold
    Given a CircuitBreaker with stall_threshold 3
    When I record 3 Think steps with success
    Then the health state should be "STALLED"

  Scenario: One step below threshold does not stall
    Given a CircuitBreaker with stall_threshold 3
    When I record 2 Think steps with success
    Then the health state should be "PROGRESS"

  Scenario: Failed productive call increments stall counter
    Given a CircuitBreaker with stall_threshold 2
    When I record 2 Bash steps with failure
    Then the health state should be "STALLED"

  # ── Spin detection ────────────────────────────────────────────────────────────

  Scenario: Repeated identical errors trigger SPINNING
    Given a CircuitBreaker with spin_threshold 3 and stall_threshold 100
    When I record 3 identical Bash error steps
    Then the health state should be "SPINNING"

  Scenario: Successful step clears error hashes
    Given a CircuitBreaker with spin_threshold 3 and stall_threshold 100
    When I record 2 identical Bash error steps
    And I record 1 Bash step with success
    Then the error hashes should be empty
    And the health state should be "PROGRESS"

  Scenario: Different error messages do not trigger SPINNING
    Given a CircuitBreaker with spin_threshold 3 and stall_threshold 100
    When I record 3 distinct Bash error steps
    Then the health state should be "PROGRESS"

  # ── Blocked detection ─────────────────────────────────────────────────────────

  Scenario: BLOCKED fires after max_rotations reached
    Given a CircuitBreaker with stall_threshold 1 and max_rotations 2
    When I record 1 Think step with success
    Then the health state should be "STALLED"
    When I record 1 WriteFile step with success
    And I record 1 Think step with success
    Then the approach_rotations should be 2
    When I record 1 Think step with success
    Then the health state should be "BLOCKED"

  Scenario: Approach rotations increment on each STALLED transition
    Given a CircuitBreaker with stall_threshold 1 and max_rotations 10
    When I record 1 Think step with success
    Then the approach_rotations should be 1
    When I record 1 WriteFile step with success
    And I record 1 Think step with success
    Then the approach_rotations should be 2

  # ── Degradation detection ─────────────────────────────────────────────────────

  Scenario: Output volume decline of 70% triggers DEGRADED
    Given a CircuitBreaker with stall_threshold 100 and degradation_pct 70.0
    When I record 3 Bash steps with success and output size 1000
    And I record 3 Bash steps with success and output size 100
    Then the health state should be "DEGRADED"

  Scenario: Stable output does not trigger DEGRADED
    Given a CircuitBreaker with stall_threshold 100 and degradation_pct 70.0
    When I record 3 Bash steps with success and output size 1000
    And I record 3 Bash steps with success and output size 900
    Then the health state should be "PROGRESS"

  Scenario: Fewer than 6 output samples never trigger DEGRADED
    Given a CircuitBreaker with stall_threshold 100 and degradation_pct 70.0
    When I record 5 Bash steps with success and output size 1000
    Then the health state should be "PROGRESS"

  # ── Rotation hint content ─────────────────────────────────────────────────────

  Scenario: Rotation hint for STALLED mentions step count and rotation counter
    Given a CircuitBreaker with stall_threshold 3
    When I record 3 Think steps with success
    Then the rotation hint should contain "STALLED"
    And the rotation hint should contain "Rotation"

  Scenario: Rotation hint for SPINNING mentions repeated error count
    Given a CircuitBreaker with spin_threshold 3 and stall_threshold 100
    When I record 3 identical Bash error steps
    Then the rotation hint should contain "SPINNING"

  Scenario: Rotation hint for DEGRADED mentions output decline
    Given a CircuitBreaker with stall_threshold 100 and degradation_pct 70.0
    When I record 3 Bash steps with success and output size 1000
    And I record 3 Bash steps with success and output size 100
    Then the rotation hint should contain "DEGRADING"

  # ── State display strings ─────────────────────────────────────────────────────

  Scenario Outline: AgentHealthState displays as expected string
    Given a health state of "<state>"
    Then its display string should be "<display>"

    Examples:
      | state    | display  |
      | STALLED  | STALLED  |
      | SPINNING | SPINNING |
      | DEGRADED | DEGRADED |
      | BLOCKED  | BLOCKED  |
      | PROGRESS | PROGRESS |
