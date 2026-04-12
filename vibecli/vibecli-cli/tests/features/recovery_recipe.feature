Feature: Prescriptive recovery recipe registry
  Maps each of 7 failure scenarios to an ordered list of recovery steps.
  Auto-recovery attempts the first automatic step once, then escalates.

  Scenario: Registry contains all 7 failure scenarios
    Given a fresh recovery registry
    Then it should contain 7 recipes

  Scenario: Auto-recovery escalates after max attempts
    Given a fresh recovery registry
    When I execute auto-recovery for "provider_timeout"
    Then the outcome should be "escalated"

  Scenario: Recovery events are logged
    Given a fresh recovery registry
    When I execute auto-recovery for "subagent_crash"
    Then 1 recovery event should be recorded

  Scenario: Every recipe's first step is automatic
    Given a fresh recovery registry
    Then every recipe's first step should have automatic true
