Feature: Green contract quality gates
  Quality levels form a strict hierarchy: TargetedTests < Package < Workspace < MergeReady.
  Higher levels cumulatively satisfy lower-level requirements.

  Scenario: Higher quality levels satisfy lower ones
    Then MergeReady should satisfy TargetedTests
    And TargetedTests should not satisfy Package

  Scenario: All checks passing yields Pass outcome
    Given a MergeReady contract
    And all checks are passing
    When I evaluate the contract
    Then the outcome should be "pass"

  Scenario: Failing tests yields Fail with reason about tests
    Given a TargetedTests contract
    And tests_passed is false
    When I evaluate the contract
    Then the outcome should contain "tests"
