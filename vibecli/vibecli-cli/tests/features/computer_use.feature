Feature: Computer Use visual self-testing
  The ComputerUseSession captures screenshot metadata and records visual
  assertions with pass/fail outcomes.

  Scenario: Recording a passing visual assertion
    Given a computer use session
    When I record a passing assertion "login button is visible"
    Then the session should have 1 assertion
    And the last assertion should have passed

  Scenario: Recording a failing visual assertion
    Given a computer use session
    When I record a failing assertion "dashboard loads in 2s"
    Then the session should have 1 assertion
    And the last assertion should not have passed

  Scenario: All assertions pass returns overall pass
    Given a computer use session
    When I record a passing assertion "header is rendered"
    And I record a passing assertion "footer is rendered"
    Then the overall result should be pass

  Scenario: One failure causes overall fail
    Given a computer use session
    When I record a passing assertion "header is rendered"
    And I record a failing assertion "modal does not appear"
    Then the overall result should be fail
