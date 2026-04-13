Feature: Desktop action model
  Actions represent GUI automation steps. Destructive actions (Type, Enter)
  are flagged. ScreenBounds validates that click/scroll/move coordinates fall
  within the screen. ActionPlan assembles steps and reports counts.

  Scenario: Type action is flagged as destructive
    Given an action of type Type with text "hello"
    Then the action should be destructive

  Scenario: Screenshot action is not destructive
    Given a Screenshot action
    Then the action should not be destructive

  Scenario: Click within bounds passes validation
    Given screen bounds of 1920 x 1080
    When I validate a click at 100 200
    Then the validation should succeed

  Scenario: Click outside bounds fails validation
    Given screen bounds of 800 x 600
    When I validate a click at 900 700
    Then the validation should fail
