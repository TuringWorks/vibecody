Feature: Reasoning provider — thinking block support
  The reasoning provider extracts, strips, and budgets <thinking> blocks
  from raw AI model output for o3-class and extended-thinking models.

  Scenario: Extract a single thinking block
    Given the raw response "before <thinking>step one</thinking> after"
    When I parse thinking blocks
    Then there should be 1 thinking block
    And the first block content should be "step one"

  Scenario: Strip thinking blocks from raw response
    Given the raw response "before <thinking>hidden</thinking> after"
    When I strip thinking blocks
    Then the result should contain "before"
    And the result should contain "after"
    And the result should not contain "hidden"

  Scenario: Token budget scales with complexity
    Given a complexity level of 1
    When I compute the token budget
    Then the thinking token budget should be 1024

    Given a complexity level of 10
    When I compute the token budget
    Then the thinking token budget should be 16384

  Scenario: Full response build strips blocks when configured
    Given the raw response "<thinking>internal</thinking>Final answer"
    And strip thinking is enabled
    When I build the reasoning response
    Then the response field should not contain "internal"
    And the response field should contain "Final answer"
