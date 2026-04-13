Feature: Focus View distraction-free session mode
  FocusManager tracks active sessions, counts distractions, and detects
  when a session should auto-exit based on elapsed time.

  Scenario: Enter and exit a focus session
    Given the focus manager is idle
    When I enter deep focus at time 1000
    Then the manager should be in focus
    When I exit focus at time 2000
    Then the manager should not be in focus
    And the session count should be 1

  Scenario: Distractions are counted on the active session
    Given the focus manager is idle
    When I enter deep focus at time 0
    And I record 3 distractions
    Then the active distraction count should be 3

  Scenario: Auto-exit triggers after the configured limit
    Given the focus manager is idle
    When I enter focus with auto-exit 60 seconds at time 1000
    Then auto-exit at time 1061 should be true
    And auto-exit at time 1059 should be false

  Scenario: Notification level ordering
    Then Silent should be less than Minimal
    And Minimal should be less than Normal
