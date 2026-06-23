Feature: /loop recurring + self-paced execution (gap C1)
  The /loop engine parses an invocation into a recurring or self-paced spec and
  decides, each tick, whether to run the body again — bounded by a MAX_ITER
  guard and a wall-clock budget so a self-paced loop can never spin forever.

  Scenario: A recurring loop parses an interval and a prompt
    Given the loop argument "5m run the tests"
    When I parse the loop arguments
    Then parsing succeeds
    And the mode is recurring with interval 300 seconds
    And the prompt is "run the tests"

  Scenario: A self-paced loop is requested with auto
    Given the loop argument "auto fix all failing tests"
    When I parse the loop arguments
    Then parsing succeeds
    And the mode is self-paced

  Scenario: A loop with no prompt is rejected
    Given the loop argument "5m"
    When I parse the loop arguments
    Then parsing fails

  Scenario: A self-paced loop stops when the validator reports done
    Given a self-paced job from "auto do the thing"
    When the validator reports done at elapsed 1 seconds
    Then the decision is stop-done

  Scenario: The MAX_ITER guard halts a runaway self-paced loop
    Given a self-paced job from "auto never satisfiable"
    And the job has run 20 iterations
    When the validator reports not-done at elapsed 1 seconds
    Then the decision is stop-max-iter

  Scenario: The wall-clock budget halts even when the validator would say done
    Given a self-paced job from "auto long task" with max duration 10 seconds
    When the validator reports done at elapsed 10 seconds
    Then the decision is stop-expired
